use std::collections::HashMap;

use syntect::parsing::SyntaxSet;
use tera::{Result, Value};

use crate::{markdown::render_markdown, syntax_highlighting::highlight_code};

/// A filter for Tera that stringifies something to JSON, adds syntax
/// highlighting and returns it inside a HTML code block. Use with Tera's `safe`
/// filter to render HTML: `json_dump(document) | safe`.
pub fn tera_json_dump<'r, 's>(
  input: &'r Value,
  _args: &'s HashMap<String, Value>,
) -> Result<Value> {
  let stringified = serde_json::to_string_pretty(input)?;

  let syntax_set = SyntaxSet::load_defaults_newlines();

  let syntax = syntax_set
    .find_syntax_by_name("JSON")
    .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

  Ok(highlight_code(&stringified, &syntax, &syntax_set).into())
}

/// A filter for Tera to apply syntax highlighting to a string of code.
///
/// An optional `language` argument can be specified to select what language to
/// use, otherwise `plaintext` is used instead.
///
/// This filter can fail if the input is not a string.
pub fn tera_highlight_code<'r, 's>(
  input: &'r Value,
  args: &'s HashMap<String, Value>,
) -> Result<Value> {
  let source = input.as_str().expect("Expected input to be a String");

  let language = args
    .get("language")
    .and_then(Value::as_str)
    .unwrap_or_default();

  let syntax_set = SyntaxSet::load_defaults_newlines();

  let syntax = syntax_set
    .find_syntax_by_token(&language)
    .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

  Ok(highlight_code(&source, &syntax, &syntax_set).into())
}

/// A filter for Tera to render some Markdown to HTML.
///
/// This filter can fail if the input is not a string.
pub fn tera_render_markdown<'r, 's>(
  input: &'r Value,
  _: &'s HashMap<String, Value>,
) -> Result<Value> {
  let source = input.as_str().expect("Expected input to be a String");

  Ok(render_markdown(source).into())
}
