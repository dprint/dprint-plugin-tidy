use std::sync::Arc;

use dprint_core::configuration::*;
use serde_json::Map;
use serde_json::Value;

use super::Configuration;

/// Resolves configuration from a collection of key value strings.
///
/// The keys are tidy's options. Those of a section are written either with the
/// section's name before a dot (ex. `"js.semi": false`) or in an object named
/// after the section (ex. `"js": { "semi": false }`), and `<section>.enabled`
/// turns off formatting the section's files.
///
/// # Example
///
/// ```
/// use dprint_core::configuration::ConfigKeyMap;
/// use dprint_core::configuration::resolve_global_config;
/// use dprint_plugin_tidy::configuration::resolve_config;
///
/// let mut config_map = ConfigKeyMap::new(); // get a collection of key value pairs from somewhere
/// let global_config_result = resolve_global_config(&mut config_map);
///
/// // check global_config_result.diagnostics here...
///
/// let config_map = ConfigKeyMap::new(); // get a collection of k/v pairs from somewhere
/// let config_result = resolve_config(
///     config_map,
///     &global_config_result.config
/// );
///
/// // check config_result.diagnostics here and use config_result.config
/// ```
pub fn resolve_config(
  config: ConfigKeyMap,
  global_config: &GlobalConfiguration,
) -> ResolveConfigurationResult<Configuration> {
  let mut diagnostics = Vec::new();

  // the global configuration only applies where it's set, so the defaults are tidy's
  let mut tidy_config = Map::new();
  if let Some(line_width) = global_config.line_width {
    tidy_config.insert("width".to_string(), line_width.into());
  }
  if let Some(indent_width) = global_config.indent_width {
    tidy_config.insert("indent".to_string(), indent_width.into());
  }
  if let Some(use_tabs) = global_config.use_tabs {
    tidy_config.insert("tabs".to_string(), use_tabs.into());
  }
  let mut new_line_kind = None;
  match global_config.new_line_kind {
    Some(NewLineKind::LineFeed) => {
      tidy_config.insert("eol".to_string(), "lf".into());
    }
    Some(NewLineKind::CarriageReturnLineFeed) => {
      tidy_config.insert("eol".to_string(), "crlf".into());
    }
    Some(NewLineKind::Auto) => new_line_kind = Some(NewLineKind::Auto),
    None => {}
  }

  let mut disabled_sections = Vec::new();
  for (key, value) in config {
    let value = to_json_value(value);
    match key.split_once('.') {
      Some((section, "enabled")) if is_section(section) => {
        resolve_enabled(section, value, &mut disabled_sections, &mut diagnostics);
      }
      Some((section, name)) => insert_section_value(&mut tidy_config, section, name.to_string(), value),
      None => match value {
        Value::Object(values) if is_section(&key) => {
          for (name, value) in values {
            if name == "enabled" {
              resolve_enabled(&key, value, &mut disabled_sections, &mut diagnostics);
            } else {
              insert_section_value(&mut tidy_config, &key, name, value);
            }
          }
        }
        value => {
          tidy_config.insert(key, value);
        }
      },
    }
  }

  // an `eol` in the plugin's configuration wins over the global `newLineKind`
  if tidy_config.contains_key("eol") {
    new_line_kind = None;
  }

  // tidy always writes Markdown with LF line endings, so the plugin changes them
  let markdown_eol = tidy_config
    .get("markdown")
    .and_then(|markdown| markdown.get("eol"))
    .or_else(|| tidy_config.get("eol"));
  let markdown_crlf = match markdown_eol {
    Some(eol) => Some(eol == "crlf"),
    None if new_line_kind.is_some() => None,
    None => Some(false),
  };

  let tidy_config = Value::Object(tidy_config);
  let options = match to_tidy_options(&tidy_config) {
    Ok(options) => options,
    Err(diagnostic) => {
      diagnostics.push(diagnostic);
      tidy::Options::default()
    }
  };
  let crlf_options = new_line_kind.and_then(|_| {
    let mut crlf_config = tidy_config.clone();
    crlf_config["eol"] = "crlf".into();
    to_tidy_options(&crlf_config).ok().map(Arc::new)
  });

  ResolveConfigurationResult {
    config: Configuration {
      properties: to_properties(tidy_config, &disabled_sections),
      new_line_kind,
      disabled_sections,
      options: Arc::new(options),
      crlf_options,
      markdown_crlf,
    },
    diagnostics,
  }
}

/// Gets if the name is that of a section of the configuration (ex. `js` or `markdown`).
fn is_section(name: &str) -> bool {
  tidy::schema::SECTIONS.contains(&name)
}

fn resolve_enabled(
  section: &str,
  value: Value,
  disabled_sections: &mut Vec<String>,
  diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
  match value {
    Value::Bool(true) => {}
    Value::Bool(false) => disabled_sections.push(section.to_string()),
    _ => diagnostics.push(ConfigurationDiagnostic {
      property_name: format!("{section}.enabled"),
      message: format!("`{section}.enabled` must be true or false"),
    }),
  }
}

fn insert_section_value(tidy_config: &mut Map<String, Value>, section: &str, name: String, value: Value) {
  let section_value = tidy_config
    .entry(section.to_string())
    .or_insert_with(|| Value::Object(Map::new()));
  // when it isn't an object, tidy reports that
  if let Value::Object(values) = section_value {
    values.insert(name, value);
  }
}

fn to_tidy_options(tidy_config: &Value) -> Result<tidy::Options, ConfigurationDiagnostic> {
  let config = tidy::config::Config::from_value(tidy_config).map_err(|message| ConfigurationDiagnostic {
    property_name: property_name_of(&message),
    message,
  })?;
  let mut options = tidy::Options::default();
  config.apply(&mut options);
  Ok(options)
}

/// Gets the property an error from tidy is about, which it quotes in backticks
/// (ex. "`js.semi` must be true or false", or "`js`: `importOrder` needs ...").
fn property_name_of(message: &str) -> String {
  let mut quoted = message.split('`').skip(1).step_by(2);
  match (quoted.next(), quoted.next()) {
    (Some(section), Some(name)) if is_section(section) && message.starts_with(&format!("`{section}`: `")) => {
      format!("{section}.{name}")
    }
    (Some(name), _) => name.to_string(),
    (None, _) => String::new(),
  }
}

/// Flattens the configuration into the properties of the dprint configuration.
fn to_properties(tidy_config: Value, disabled_sections: &[String]) -> Map<String, Value> {
  let Value::Object(tidy_config) = tidy_config else {
    unreachable!("the configuration is an object");
  };
  let mut properties = Map::new();
  for (key, value) in tidy_config {
    match value {
      Value::Object(values) if is_section(&key) => {
        for (name, value) in values {
          properties.insert(format!("{key}.{name}"), value);
        }
      }
      value => {
        properties.insert(key, value);
      }
    }
  }
  for section in tidy::schema::SECTIONS {
    let is_enabled = !disabled_sections.iter().any(|disabled| disabled == section);
    properties.insert(format!("{section}.enabled"), is_enabled.into());
  }
  properties
}

fn to_json_value(value: ConfigKeyValue) -> Value {
  match value {
    ConfigKeyValue::String(value) => Value::String(value),
    ConfigKeyValue::Number(value) => value.into(),
    ConfigKeyValue::Bool(value) => Value::Bool(value),
    ConfigKeyValue::Array(values) => Value::Array(values.into_iter().map(to_json_value).collect()),
    ConfigKeyValue::Object(values) => Value::Object(
      values
        .into_iter()
        .map(|(key, value)| (key, to_json_value(value)))
        .collect(),
    ),
    ConfigKeyValue::Null => Value::Null,
  }
}
