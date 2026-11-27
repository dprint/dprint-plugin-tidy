use std::borrow::Cow;
use std::path::Path;

use dprint_core::configuration::resolve_new_line_kind;

use crate::configuration::Configuration;
use crate::file_kind::language_of;
use crate::file_kind::section_of;

/// Error that occurred while formatting a file.
#[derive(Debug, thiserror::Error)]
pub enum FormatError {
  /// The text could not be parsed.
  #[error("Syntax error at line {line}, column {column}: {message}")]
  Syntax {
    line: usize,
    column: usize,
    message: String,
  },
  /// Any other error from tidy (ex. the text nests too deeply to format).
  #[error("{0}")]
  Tidy(tidy::Error),
}

/// Formats the text of the file as the language its name says it is, or returns
/// `None` when it isn't a file the configuration formats or is already formatted.
pub fn format_text(file_path: &Path, input_text: &str, config: &Configuration) -> Result<Option<String>, FormatError> {
  format_text_with_formatter(&mut tidy::Formatter::default(), file_path, input_text, config)
}

/// Formats the text with a formatter whose buffers are reused from one file to the next.
pub(crate) fn format_text_with_formatter(
  formatter: &mut tidy::Formatter,
  file_path: &Path,
  input_text: &str,
  config: &Configuration,
) -> Result<Option<String>, FormatError> {
  let Some(language) = language_of(file_path) else {
    return Ok(None);
  };
  if !config.is_section_enabled(section_of(language)) {
    return Ok(None);
  }

  let is_crlf = config
    .new_line_kind
    .is_some_and(|kind| resolve_new_line_kind(input_text, kind) == "\r\n");
  let options = config.tidy_options(is_crlf);
  let path = file_path.to_string_lossy();
  let formatted = formatter
    .format(input_text, language, options, Some(&path), &mut tidy::embed::Verbatim)
    .map_err(|err| to_format_error(err, input_text))?;
  let formatted = match language {
    tidy::Language::Markdown if config.markdown_crlf.unwrap_or(is_crlf) => Cow::Owned(to_crlf(formatted)),
    _ => Cow::Borrowed(formatted),
  };
  if formatted == input_text {
    Ok(None)
  } else {
    Ok(Some(formatted.into_owned()))
  }
}

fn to_crlf(text: &str) -> String {
  let mut output = String::with_capacity(text.len() + text.len() / 16);
  for line in text.split_inclusive('\n') {
    match line.strip_suffix('\n') {
      Some(line) => {
        output.push_str(line.strip_suffix('\r').unwrap_or(line));
        output.push_str("\r\n");
      }
      None => output.push_str(line),
    }
  }
  output
}

fn to_format_error(err: tidy::Error, text: &str) -> FormatError {
  match err {
    tidy::Error::Syntax { message, at } => {
      let (line, column) = line_and_column(text, at);
      FormatError::Syntax { line, column, message }
    }
    err => FormatError::Tidy(err),
  }
}

/// Gets the 1-indexed line and column of the byte offset in the text.
fn line_and_column(text: &str, offset: usize) -> (usize, usize) {
  let mut offset = offset.min(text.len());
  while !text.is_char_boundary(offset) {
    offset -= 1;
  }
  let text_before = &text[..offset];
  let line = text_before.matches('\n').count() + 1;
  let line_start = text_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
  let column = text_before[line_start..].chars().count() + 1;
  (line, column)
}
