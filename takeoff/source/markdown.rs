use pulldown_cmark::{
  html::push_html, CodeBlockKind, Event, Options, Parser, Tag,
};
use syntect::parsing::SyntaxSet;

use crate::syntax_highlighting::highlight_code;

/// Renders some Markdown to HTML using [`pulldown_cmark`].
pub fn render_markdown(source: &str) -> String {
  // Create the parser with all options enabled.
  let parser = Parser::new_ext(source, Options::all());

  // Load the syntaxes from Syntect.
  let syntax_set = SyntaxSet::load_defaults_newlines();

  // Define some state we'll use in the rendering.
  let mut code_language = String::new();
  let mut code_to_highlight = String::new();
  let mut events = vec![];
  let mut in_code_block = false;
  let mut syntax = syntax_set.find_syntax_plain_text();

  for event in parser {
    match event {
      Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(language))) => {
        // When a fenced codeblock is started, assign it to the state.
        code_language = language.to_string();
        syntax = syntax_set
          .find_syntax_by_token(&language)
          .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
        in_code_block = true;
      }
      Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
        // When a fenced code is closed, highlight the code that was inside it.
        if in_code_block {
          // Add the highlighted code to the final result.
          events.push(Event::Html(
            format!(
              r#"<pre class="language-{}"><code>{}</code></pre>"#,
              code_language,
              highlight_code(&code_to_highlight, syntax, &syntax_set)
            )
            .into(),
          ));

          // Reset the state.
          code_to_highlight = String::new();
          in_code_block = false;
          syntax = syntax_set.find_syntax_plain_text();
        }
      }
      Event::Text(text) => {
        if in_code_block {
          // When we're parsing some text and we're in a code block, add it to
          // the code we need to highlight.
          code_to_highlight.push_str(&text);
        } else {
          // Otherwise just return it as it came in.
          events.push(Event::Text(text));
        }
      }
      _ => events.push(event),
    };
  }

  // Finally, render the HTML into a string.
  let mut html = String::new();
  push_html(&mut html, events.into_iter());

  html
}
