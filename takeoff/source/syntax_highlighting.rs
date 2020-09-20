use syntect::{
  html::{ClassStyle, ClassedHTMLGenerator},
  parsing::{SyntaxReference, SyntaxSet},
};

/// Highlights some code with [`syntect`]. See the source code for
/// [`tera_highlight_code`](crate::templating::tera_highlight_code)
/// for an example.
pub fn highlight_code(
  source: &str,
  syntax: &SyntaxReference,
  syntax_set: &SyntaxSet,
) -> String {
  let mut generator = ClassedHTMLGenerator::new_with_class_style(
    &syntax,
    &syntax_set,
    ClassStyle::SpacedPrefixed { prefix: "code-" },
  );

  for line in source.lines() {
    generator.parse_html_for_line(line);
  }

  generator.finalize()
}
