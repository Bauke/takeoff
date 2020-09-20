use std::{
  fs::{create_dir_all, read_to_string},
  path::PathBuf,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use toml::value::Table;
use tracing::{debug, instrument};

use crate::Launchpad;

/// A [`Document`] describes a Markdown file found in a [`Launchpad`]'s
/// [`source`](Launchpad::source) directory.
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Document {
  /// The content of the Markdown file without the `<!-- Metadata -->` block if
  /// it was present.
  pub content: String,
  /// The parsed metadata.
  pub metadata: Metadata,
  /// The absolute path of the source Markdown file.
  pub source_path: PathBuf,
}

impl Document {
  /// Creates a new [`Document`] from a file. If parsing [`Metadata`] fails for
  /// this document, then an error is printed and [`Metadata::default()`] will
  /// be used instead.
  #[instrument]
  pub fn new(path: PathBuf) -> Result<Self> {
    debug!("Parsing");
    let (metadata, source) = Metadata::parse(&read_to_string(&path)?);
    let metadata = metadata.unwrap_or_else(|err| {
      eprintln!(
        "Error parsing metadata for {:?} (using default): {}",
        path, err
      );
      Metadata::default()
    });

    let document = Self {
      content: source,
      metadata,
      source_path: path,
    };
    Ok(document)
  }

  /// Creates all the directories required and returns the HTML path where
  /// this [`Document`] should be written to.
  pub fn create_destination(&self, launchpad: &Launchpad) -> Result<PathBuf> {
    let parent_dirs = self
      .source_path
      .strip_prefix(&launchpad.source)?
      .parent()
      .unwrap();

    let output = launchpad.output.join(parent_dirs);

    create_dir_all(&output)?;
    let file_stem = self.source_path.file_stem().unwrap();
    Ok(output.join(file_stem).with_extension("html"))
  }
}

/// [`Metadata`] contains all the data found in a [`Document`]'s Markdown
/// `<!-- Metadata -->` block.
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Metadata {
  /// Extra custom metadata to include when rendering this [`Document`].
  #[serde(default)]
  pub extra: Table,
  /// A boolean indicating whether this [`Document`] should be ignored entirely.
  #[serde(default)]
  pub ignore: bool,
  /// A boolean indicating whether this [`Document`] is a draft. See
  /// [`Launchpad::mode`] for details.
  #[serde(default = "Metadata::is_draft_default")]
  pub is_draft: bool,
  /// The template to use for this [`Document`].
  ///
  /// Relative to [`Templates::directory`](crate::launchpad::Templates).
  #[serde(default)]
  pub template: Option<String>,
}

impl Default for Metadata {
  fn default() -> Self {
    Self {
      extra: Table::default(),
      ignore: false,
      is_draft: Self::is_draft_default(),
      template: None,
    }
  }
}

impl Metadata {
  /// Tries to parse [`Metadata`] from a TOML string with [`serde`] and [`toml`].
  pub fn from_toml(source: &str) -> Result<Self> {
    toml::from_str(source).map_err(Into::into)
  }

  /// Tries to parse [`Metadata`] from a Markdown string's
  /// `<!-- Metadata -->` block.
  ///
  /// If the Markdown string does not start with a `<!-- Metadata -->` block
  /// then `Metadata::default()` is returned in the [`Result`] and the
  /// Markdown as the String.
  ///
  /// If the Markdown does start with a `<!-- Metadata -->` block then the
  /// contents inside it will try to parse a [`Metadata`] with
  /// [`Metadata::from_toml()`]. The returned Markdown string will then also
  /// have the Metadata block removed.
  #[instrument(skip(source))]
  pub fn parse(source: &str) -> (Result<Self>, String) {
    const METADATA_START: &str = "<!-- Metadata\n";
    const METADATA_END: &str = "\n-->";

    // If no metadata is included, return default metadata.
    if !source.starts_with(METADATA_START) && !source.contains(METADATA_END) {
      debug!("No Metadata block found, returning early");
      return (Ok(Self::default()), source.to_string());
    }

    // Grab the start and end indexes of the metadata TOML.
    let start = source.find(METADATA_START).unwrap() + METADATA_START.len();
    let end = source.find(METADATA_END).unwrap();

    // Get the metadata TOML itself.
    let metadata_toml = &source[start..end];

    // Remove the metadata from the Markdown.
    let new_input = source[end + METADATA_END.len()..].to_string();

    (Self::from_toml(metadata_toml), new_input)
  }

  pub(crate) fn is_draft_default() -> bool {
    true
  }
}
