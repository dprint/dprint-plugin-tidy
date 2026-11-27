use std::sync::Arc;

use dprint_core::configuration::NewLineKind;
use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

/// The resolved configuration.
///
/// The options are tidy's, the ones a `tidy.json` has, so tidy decides their
/// defaults and checks their values.
#[derive(Clone, Serialize)]
pub struct Configuration {
  /// The options that are set, by their names in the dprint configuration
  /// (ex. `width` or `js.semi`), along with `<section>.enabled` for every section.
  #[serde(flatten)]
  pub(crate) properties: Map<String, Value>,
  /// The line ending when it's decided by the file (`auto`).
  #[serde(rename = "newLineKind", skip_serializing_if = "Option::is_none")]
  pub(crate) new_line_kind: Option<NewLineKind>,
  #[serde(skip)]
  pub(crate) disabled_sections: Vec<String>,
  #[serde(skip)]
  pub(crate) options: Arc<tidy::Options>,
  /// The options for a file with CRLF line endings when `new_line_kind` is set.
  #[serde(skip)]
  pub(crate) crlf_options: Option<Arc<tidy::Options>>,
  /// If Markdown is written with CRLF line endings, or `None` when that's decided
  /// by the file. tidy always writes it with LF.
  #[serde(skip)]
  pub(crate) markdown_crlf: Option<bool>,
}

impl Configuration {
  /// Gets if the files of a section of the configuration (ex. `js` or `markdown`) are formatted.
  pub fn is_section_enabled(&self, section: &str) -> bool {
    !self.disabled_sections.iter().any(|disabled| disabled == section)
  }

  /// Gets tidy's options for a file with the provided line endings.
  pub(crate) fn tidy_options(&self, is_crlf: bool) -> &tidy::Options {
    match &self.crlf_options {
      Some(crlf_options) if is_crlf => crlf_options,
      _ => &self.options,
    }
  }
}
