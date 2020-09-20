use std::{
  env::current_dir,
  ffi::OsStr,
  fs::{copy, create_dir_all, remove_dir_all, write},
  path::PathBuf,
};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};
use tracing::{debug, instrument, trace};
use walkdir::WalkDir;

use crate::{
  document::Document,
  markdown::render_markdown,
  templating::{tera_highlight_code, tera_json_dump, tera_render_markdown},
};

/// The entry point for Takeoff.
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Launchpad {
  /// If set to `true`, any `.scss` files found in [`Launchpad::statics`]
  /// will be compiled and output as CSS with [`grass`]. Sass partials
  /// (`.scss` files starting with an underscore) will also be skipped.
  ///
  /// Defaults to `false`.
  pub compile_sass: bool,
  /// The Launchpad mode.
  ///
  /// * [`Mode::Development`] will render all [`Document`]s.
  /// * [`Mode::Production`] only renders [`Document`]s with `is_draft = false`
  /// and will trigger a complete rebuild of the site (completely removing
  /// `output` before starting).
  ///
  /// Defaults to [`Mode::Development`]
  pub mode: Mode,
  /// The root directory to output the site files to.
  ///
  /// Defaults to `"public"`.
  pub output: PathBuf,
  /// The root directory to look for Markdown and static files.
  ///
  /// Defaults to `"source"`.
  pub source: PathBuf,
  /// Static files to copy or process\*.
  ///
  /// The first string in the tuple should be a valid [`glob`] string that
  /// targets files to copy. Relative to [`Launchpad::source`].
  ///
  /// And the second string should be a directory to copy the files matched by
  /// the glob to. Relative to [`Launchpad::output`].
  ///
  /// \* Optionally, some files can be automatically processed as well, see
  /// [`Launchpad::compile_sass`].
  pub statics: Vec<(String, String)>,
  /// Templating settings.
  pub templates: Templates,
  /// The [`tera`] instance to re-use for templating.
  #[serde(skip)]
  pub tera: Tera,
}

impl Launchpad {
  /// Creates a new [`LaunchpadBuilder`].
  pub fn prepare() -> LaunchpadBuilder {
    LaunchpadBuilder::default()
  }

  /// Parses and returns all [`Document`]s defined by this [`Launchpad`].
  #[instrument(skip(self))]
  pub fn parse_documents(&self) -> Result<Vec<Document>> {
    let mut documents = vec![];

    debug!("Walking {:?}", self.source);
    for entry in WalkDir::new(&self.source)
      .follow_links(true)
      .into_iter()
      .filter_map(Result::ok)
    {
      let path = entry.path();
      if path.extension().and_then(OsStr::to_str) == Some("md") {
        trace!("Parsing {:?}", path);
        let mut document = Document::new(path.into())?;
        document.content = render_markdown(&document.content);
        documents.push(document);
      }
    }

    debug!("Launchpad mode: {:?}", self.mode);
    if self.mode == Mode::Production {
      debug!("Excluding documents where is_draft = true");
    }

    let documents = documents.into_iter().filter(|doc| {
      if self.mode == Mode::Production {
        trace!("Excluding {:?}", doc.source_path);
        !doc.metadata.is_draft
      } else {
        true
      }
    });

    Ok(documents.collect())
  }

  /// Generates the site defined by this [`Launchpad`].
  #[instrument(skip(self))]
  pub fn take_off(&self) -> Result<()> {
    if self.mode == Mode::Production && self.output.exists() {
      debug!("Removing {:?}", self.output);
      remove_dir_all(&self.output)?;
    }

    let documents = self.parse_documents()?;

    let mut context = Context::new();
    context.insert("launchpad", self);
    context.insert("documents", &documents);

    for document in documents {
      debug!("Rendering {:?}", document.source_path);

      context.insert("document", &document);
      context.insert("metadata", &document.metadata);
      context.insert("extra", &document.metadata.extra);

      let template = document
        .metadata
        .template
        .as_ref()
        .unwrap_or(&self.templates.default);

      trace!("Using template: {}", template);
      let html = self.tera.render(&template, &context)?;

      let destination = document.create_destination(self)?;
      trace!("Writing to {:?}", destination);
      write(destination, &html)?;
    }

    for (source, destination) in &self.statics {
      let source = self.source.join(source);
      let destination = self.output.join(destination);
      create_dir_all(&destination)?;
      debug!("Processing ({:?},{:?})", source, destination);

      for entry in glob::glob(source.to_str().unwrap())?
        .filter_map(Result::ok)
        .filter(|e| e.is_file())
      {
        let entry_destination = destination.join(entry.file_name().unwrap());
        let file_name = entry_destination
          .file_name()
          .and_then(OsStr::to_str)
          .unwrap();

        if self.compile_sass && file_name.ends_with(".scss") {
          // Don't attempt to compile Sass partials.
          if file_name.starts_with('_') {
            trace!("Skipping Sass partial {:?}", entry);
            continue;
          }

          trace!("Compiling Sass for {:?}", entry);
          let css = grass::from_path(
            &entry.as_path().to_str().unwrap(),
            &grass::Options::default(),
          )
          .map_err(|err| anyhow!(err.to_string()))?;

          let entry_destination = entry_destination.with_extension("css");
          trace!("Writing Sass to {:?}", entry_destination);
          write(entry_destination, css)?;
        } else {
          trace!("Copying {:?} to {:?}", entry, entry_destination);
          copy(entry, entry_destination)?;
        }
      }
    }

    Ok(())
  }
}

/// The mode for [`Launchpad`] to take off in. See [`Launchpad::mode`] for
/// details.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[non_exhaustive]
pub enum Mode {
  Development,
  Production,
}

/// Templating settings for [`Launchpad`].
#[derive(Debug, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Templates {
  /// The template to be used when a [`Document`] does not define one.
  ///
  /// Relative to [`Templates::directory`].
  ///
  /// Defaults to `"base.html"`.
  #[serde(default = "Templates::templates_default_default")]
  pub default: String,
  /// The directory that [`tera`] should look in for templates.
  ///
  /// Relative to [`Launchpad::source`].
  ///
  /// Defaults to `"templates"`.
  #[serde(default = "Templates::templates_directory_default")]
  pub directory: PathBuf,
}

impl Default for Templates {
  fn default() -> Self {
    Self {
      default: Templates::templates_default_default(),
      directory: Templates::templates_directory_default(),
    }
  }
}

impl Templates {
  pub(crate) fn templates_default_default() -> String {
    "base.html".to_string()
  }

  pub(crate) fn templates_directory_default() -> PathBuf {
    "templates".into()
  }
}

/// A builder to configure a [`Launchpad`].
#[derive(Debug)]
#[non_exhaustive]
pub struct LaunchpadBuilder(pub(crate) Launchpad);

impl Default for LaunchpadBuilder {
  fn default() -> Self {
    Self(Launchpad {
      compile_sass: false,
      mode: Mode::Development,
      output: "public".into(),
      source: "source".into(),
      statics: vec![],
      templates: Templates::default(),
      tera: Tera::default(),
    })
  }
}

impl LaunchpadBuilder {
  /// Set [`Launchpad::compile_sass`].
  #[instrument(skip(self))]
  pub fn compile_sass(self, compile_sass: bool) -> Self {
    trace!("Setting compile_sass to {}", compile_sass);
    Self(Launchpad {
      compile_sass,
      ..self.0
    })
  }

  /// Set [`Launchpad::mode`].
  #[instrument(skip(self))]
  pub fn mode(self, mode: Mode) -> Self {
    trace!("Setting mode to {:?}", mode);
    Self(Launchpad { mode, ..self.0 })
  }

  /// Set Launchpad's mode to [`Mode::Development`].
  pub fn mode_development(self) -> Self {
    self.mode(Mode::Development)
  }

  /// Set Launchpad's mode to [`Mode::Production`].
  pub fn mode_production(self) -> Self {
    self.mode(Mode::Production)
  }

  /// Set [`Launchpad::output`].
  #[instrument(skip(self))]
  pub fn output(self, output: &str) -> Self {
    trace!("Setting output to {}", output);
    Self(Launchpad {
      output: output.into(),
      ..self.0
    })
  }

  /// Set [`Launchpad::source`].
  #[instrument(skip(self))]
  pub fn source(self, source: &str) -> Self {
    trace!("Setting source to {}", source);
    Self(Launchpad {
      source: source.into(),
      ..self.0
    })
  }

  /// Set [`Launchpad::statics`].
  #[instrument(skip(self))]
  pub fn statics(self, statics: Vec<(&str, &str)>) -> Self {
    let mut owned_statics = vec![];
    for tuple in statics {
      owned_statics.push((tuple.0.to_string(), tuple.1.to_string()));
    }

    trace!("Setting statics to {:?}", owned_statics);
    Self(Launchpad {
      statics: owned_statics,
      ..self.0
    })
  }

  /// Set [`Templates::default`].
  #[instrument(skip(self))]
  pub fn templates_default(self, default: &str) -> Self {
    trace!("Setting templates.default to {}", default);
    Self(Launchpad {
      templates: Templates {
        default: default.to_string(),
        ..self.0.templates
      },
      ..self.0
    })
  }

  /// Set [`Templates::directory`].
  #[instrument(skip(self))]
  pub fn templates_directory(self, directory: &str) -> Self {
    trace!("Setting templates.directory to {}", directory);
    Self(Launchpad {
      templates: Templates {
        directory: directory.into(),
        ..self.0.templates
      },
      ..self.0
    })
  }

  /// Finalize the build and return the resulting [`Launchpad`].
  #[instrument(skip(self))]
  pub fn build(self) -> Result<Launchpad> {
    let source = &self.0.source;
    let directory =
      current_dir()?.join(source.join(&self.0.templates.directory));
    let directory = directory.to_str().unwrap();
    let files = if directory.ends_with('/') {
      directory.to_string()
    } else {
      directory.to_string() + "/"
    } + "**/*.html";

    debug!("{}", files);
    let mut tera = Tera::new(&files)?;
    tera.register_filter("highlight_code", tera_highlight_code);
    tera.register_filter("json_dump", tera_json_dump);
    tera.register_filter("render_markdown", tera_render_markdown);

    let launchpad = Launchpad {
      output: current_dir()?.join(self.0.output),
      source: current_dir()?.join(self.0.source),
      tera,
      ..self.0
    };
    Ok(launchpad)
  }
}
