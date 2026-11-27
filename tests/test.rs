extern crate dprint_development;
extern crate dprint_plugin_tidy;

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use dprint_core::configuration::*;
use dprint_development::*;
use dprint_plugin_tidy::configuration::Configuration;
use dprint_plugin_tidy::configuration::resolve_config;
use dprint_plugin_tidy::*;
use serde_json::Value;

#[test]
fn test_specs() {
  let global_config = GlobalConfiguration::default();

  run_specs(
    &PathBuf::from("./tests/specs"),
    &ParseSpecOptions {
      default_file_name: "file.ts",
    },
    &RunSpecsOptions {
      fix_failures: false,
      format_twice: true,
    },
    {
      let global_config = global_config.clone();
      Arc::new(move |file_path, file_text, _range, spec_config| {
        let spec_config: ConfigKeyMap = serde_json::from_value(spec_config.clone().into()).unwrap();
        let config_result = resolve_config(spec_config, &global_config);
        ensure_no_diagnostics(&config_result.diagnostics);

        format_text(file_path, file_text, &config_result.config).map_err(|err| err.into())
      })
    },
    Arc::new(move |_file_path, _file_text, _spec_config| panic!("Plugin does not support dprint-core tracing.")),
  )
}

#[test]
fn should_fail_on_parse_error() {
  let err = format_text(Path::new("file.ts"), "const a = (\n", &default_config()).unwrap_err();
  assert_eq!(err.to_string(), "Syntax error at line 2, column 1: Unexpected token");
}

#[test]
fn should_not_format_files_tidy_does_not_know() {
  let result = format_text(Path::new("file.rs"), "fn  main() {}\n", &default_config()).unwrap();
  assert!(result.is_none());
}

#[test]
fn should_not_format_disabled_sections() {
  let config = config_with(&[("js.enabled", ConfigKeyValue::Bool(false))]);
  assert!(
    format_text(Path::new("file.ts"), "const a=1", &config)
      .unwrap()
      .is_none()
  );
  assert!(!file_extensions(&config).contains(&"ts".to_string()));
  // other sections still format
  assert!(
    format_text(Path::new("file.css"), "a{color:red}", &config)
      .unwrap()
      .is_some()
  );

  let config = config_with(&[("js", object(&[("enabled", ConfigKeyValue::Bool(false))]))]);
  assert!(
    format_text(Path::new("file.ts"), "const a=1", &config)
      .unwrap()
      .is_none()
  );
}

#[test]
fn should_use_global_config() {
  let global_config = GlobalConfiguration {
    line_width: Some(20),
    indent_width: Some(4),
    use_tabs: Some(false),
    ..Default::default()
  };
  let config = resolve_config(ConfigKeyMap::new(), &global_config).config;
  let result = format_text(
    Path::new("file.ts"),
    "function f() { return callSomething(a, b) }",
    &config,
  )
  .unwrap();
  assert_eq!(
    result.unwrap(),
    "function f() {\n    return callSomething(\n        a,\n        b,\n    );\n}\n"
  );
}

#[test]
fn should_prefer_plugin_config_over_global_config() {
  let global_config = GlobalConfiguration {
    use_tabs: Some(true),
    ..Default::default()
  };
  let mut config_map = ConfigKeyMap::new();
  config_map.insert("tabs".to_string(), ConfigKeyValue::Bool(false));
  let config = resolve_config(config_map, &global_config).config;
  let result = format_text(Path::new("file.ts"), "function f() { return 1 }", &config).unwrap();
  assert_eq!(result.unwrap(), "function f() {\n  return 1;\n}\n");
}

#[test]
fn should_use_crlf_when_configured() {
  let config = config_with_new_line_kind(NewLineKind::CarriageReturnLineFeed, &[]);
  let result = format_text(Path::new("file.css"), "a{color:red}\n", &config).unwrap();
  assert_eq!(result.unwrap(), "a {\r\n  color: red;\r\n}\r\n");
}

#[test]
fn should_keep_line_endings_with_auto() {
  let config = config_with_new_line_kind(NewLineKind::Auto, &[]);
  let result = format_text(Path::new("file.css"), "a{color:red}\r\n", &config).unwrap();
  assert_eq!(result.unwrap(), "a {\r\n  color: red;\r\n}\r\n");
  let result = format_text(Path::new("file.css"), "a{color:red}\n", &config).unwrap();
  assert_eq!(result.unwrap(), "a {\n  color: red;\n}\n");
}

#[test]
fn should_prefer_eol_over_new_line_kind() {
  let config = config_with_new_line_kind(NewLineKind::Auto, &[("eol", ConfigKeyValue::String("lf".to_string()))]);
  let result = format_text(Path::new("file.css"), "a{color:red}\r\n", &config).unwrap();
  assert_eq!(result.unwrap(), "a {\n  color: red;\n}\n");
}

#[test]
fn should_write_crlf_markdown() {
  // tidy always writes Markdown with LF, so the plugin changes the line endings
  let config = config_with_new_line_kind(NewLineKind::CarriageReturnLineFeed, &[]);
  let result = format_text(Path::new("file.md"), "# Title\n* a\n", &config).unwrap();
  assert_eq!(result.unwrap(), "# Title\r\n\r\n- a\r\n");

  let config = config_with_new_line_kind(NewLineKind::Auto, &[]);
  let result = format_text(Path::new("file.md"), "# Title\r\n* a\r\n", &config).unwrap();
  assert_eq!(result.unwrap(), "# Title\r\n\r\n- a\r\n");
  let result = format_text(Path::new("file.md"), "# Title\r\n\r\n- a\r\n", &config).unwrap();
  assert!(result.is_none());

  let config = config_with_new_line_kind(
    NewLineKind::CarriageReturnLineFeed,
    &[("markdown.eol", ConfigKeyValue::String("lf".to_string()))],
  );
  let result = format_text(Path::new("file.md"), "# Title\r\n* a\r\n", &config).unwrap();
  assert_eq!(result.unwrap(), "# Title\n\n- a\n");
}

#[test]
fn should_report_invalid_option_values() {
  let cases = [
    ("js.semi", ConfigKeyValue::String("no".to_string()), "js.semi"),
    ("width", ConfigKeyValue::String("wide".to_string()), "width"),
    ("unknown", ConfigKeyValue::Bool(true), "unknown"),
    ("js.unknown", ConfigKeyValue::Bool(true), "js.unknown"),
    ("js.enabled", ConfigKeyValue::Number(1), "js.enabled"),
    // this one needs `js.sortImports`
    (
      "js.importOrderSeparation",
      ConfigKeyValue::Bool(true),
      "js.importOrderSeparation",
    ),
  ];
  for (key, value, property_name) in cases {
    let mut config_map = ConfigKeyMap::new();
    config_map.insert(key.to_string(), value);
    let result = resolve_config(config_map, &GlobalConfiguration::default());
    let property_names = result
      .diagnostics
      .iter()
      .map(|d| d.property_name.as_str())
      .collect::<Vec<_>>();
    assert_eq!(property_names, [property_name], "{key}");
  }
}

#[test]
fn should_output_resolved_config_with_dprint_names() {
  let config = config_with(&[
    ("width", ConfigKeyValue::Number(100)),
    ("js", object(&[("semi", ConfigKeyValue::Bool(false))])),
    ("markdown.enabled", ConfigKeyValue::Bool(false)),
  ]);
  let value = serde_json::to_value(&config).unwrap();
  assert_eq!(value["width"], 100);
  assert_eq!(value["js.semi"], false);
  assert_eq!(value["js.enabled"], true);
  assert_eq!(value["markdown.enabled"], false);
}

/// The schema must describe tidy's options, so this fails when an update to
/// tidy adds, removes, or changes the default of an option.
#[test]
fn should_have_schema_matching_tidys_options() {
  let schema: Value = serde_json::from_str(include_str!("../deployment/schema.json")).unwrap();
  let properties = schema["properties"].as_object().unwrap();

  let mut expected = Vec::new();
  let mut expected_names = BTreeSet::new();
  for option in tidy::schema::shared() {
    expected_names.insert(option.name.to_string());
    expected.push((option.name.to_string(), option));
  }
  for section in tidy::schema::SECTIONS {
    let enabled_name = format!("{section}.enabled");
    assert_eq!(properties[&enabled_name]["default"], true, "{enabled_name}");
    expected_names.insert(enabled_name);
    for option in tidy::schema::section(section) {
      let name = format!("{section}.{}", option.name);
      expected_names.insert(name.clone());
      expected.push((name, option));
    }
  }
  let schema_names = properties.keys().cloned().collect::<BTreeSet<_>>();
  assert_eq!(schema_names, expected_names);

  for (name, option) in &expected {
    let property = &properties[name];
    if option.needs.is_empty() && !option.default.is_null() {
      assert_eq!(property["default"], option.default, "default of {name}");
    } else {
      // setting these without `js.sortImports` is an error, so they can't have a default
      assert!(property.get("default").is_none(), "{name} shouldn't have a default");
    }
  }

  // the defaults are what the plugin resolves without any configuration
  let mut config_map = ConfigKeyMap::new();
  for (name, property) in properties {
    if let Some(default) = property.get("default") {
      config_map.insert(name.clone(), serde_json::from_value(default.clone()).unwrap());
    }
  }
  let result = resolve_config(config_map, &GlobalConfiguration::default());
  ensure_no_diagnostics(&result.diagnostics);

  // every property is in exactly one section
  let mut section_names = Vec::new();
  for section in schema["x-sections"].as_array().unwrap() {
    for name in section["properties"].as_array().unwrap() {
      section_names.push(name.as_str().unwrap());
    }
  }
  assert_eq!(section_names.len(), properties.len());
  assert_eq!(
    section_names.into_iter().collect::<BTreeSet<_>>(),
    properties.keys().map(|name| name.as_str()).collect::<BTreeSet<_>>()
  );
}

fn default_config() -> Configuration {
  resolve_config(Default::default(), &GlobalConfiguration::default()).config
}

fn config_with(values: &[(&str, ConfigKeyValue)]) -> Configuration {
  let result = resolve_config(to_config_map(values), &GlobalConfiguration::default());
  ensure_no_diagnostics(&result.diagnostics);
  result.config
}

fn config_with_new_line_kind(new_line_kind: NewLineKind, values: &[(&str, ConfigKeyValue)]) -> Configuration {
  let global_config = GlobalConfiguration {
    new_line_kind: Some(new_line_kind),
    ..Default::default()
  };
  let result = resolve_config(to_config_map(values), &global_config);
  ensure_no_diagnostics(&result.diagnostics);
  result.config
}

fn object(values: &[(&str, ConfigKeyValue)]) -> ConfigKeyValue {
  ConfigKeyValue::Object(to_config_map(values))
}

fn to_config_map(values: &[(&str, ConfigKeyValue)]) -> ConfigKeyMap {
  values
    .iter()
    .map(|(key, value)| (key.to_string(), value.clone()))
    .collect()
}
