use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, App, Arg};
use takeoff::{launchpad::Mode, Launchpad};
use tracing_subscriber::filter::LevelFilter;

pub fn main() -> Result<()> {
  let cli = App::new("takeoff")
    .author(crate_authors!())
    .long_about(crate_description!())
    .version(crate_version!())
    .arg(
      Arg::with_name("verbosity")
        .help("The maximum tracing level to output.")
        .short("l")
        .long("verbosity")
        .takes_value(true)
        .default_value("warn")
        .possible_values(&[
          "silent", "error", "warn", "info", "debug", "trace",
        ]),
    )
    .arg(
      Arg::with_name("compile sass")
        .help("Whether to compile .scss files found in the statics.")
        .short("c")
        .long("compile-sass")
        .takes_value(false),
    )
    .arg(
      Arg::with_name("mode")
        .help(
          "Whether to run in development or production mode. Development \
outputs all documents while production skips documents with is_draft = true.",
        )
        .short("m")
        .long("mode")
        .takes_value(true)
        .default_value("development")
        .possible_values(&["development", "production"]),
    )
    .arg(
      Arg::with_name("output")
        .help("The directory to output the resulting files.")
        .short("o")
        .long("output")
        .takes_value(true)
        .default_value("public"),
    )
    .arg(
      Arg::with_name("source")
        .help("The directory to look for source files.")
        .short("s")
        .long("source")
        .takes_value(true)
        .default_value("source"),
    )
    .arg(
      Arg::with_name("statics")
        .help(
          "Static files to include in processing, relative to the source \
directory. Input must be a valid glob and a directory, separated by a colon.",
        )
        .short("f")
        .long("statics")
        .takes_value(true)
        .multiple(true),
    )
    .arg(
      Arg::with_name("templates default")
        .help("The template to use when a document doesn't specify one.")
        .short("t")
        .long("templates-default")
        .takes_value(true)
        .default_value("base.html"),
    )
    .arg(
      Arg::with_name("templates directory")
        .help(
          "The directory to look for templates in, relative to the \
source directory.",
        )
        .short("d")
        .long("templates-directory")
        .takes_value(true)
        .default_value("templates"),
    )
    .get_matches();

  let verbosity = match cli.value_of("verbosity").unwrap() {
    "silent" => LevelFilter::OFF,
    "error" => LevelFilter::ERROR,
    "warn" => LevelFilter::WARN,
    "info" => LevelFilter::INFO,
    "debug" => LevelFilter::DEBUG,
    "trace" => LevelFilter::TRACE,
    _ => unreachable!(),
  };

  tracing_subscriber::fmt().with_max_level(verbosity).init();

  let compile_sass = cli.is_present("compile sass");
  let mode = match cli.value_of("mode").unwrap() {
    "development" => Mode::Development,
    "production" => Mode::Production,
    _ => unreachable!(),
  };
  let output = cli.value_of("output").unwrap();
  let source = cli.value_of("source").unwrap();
  let statics = {
    let statics = cli.values_of("statics").unwrap_or_default();
    let mut converted = vec![];
    for input in statics {
      if !input.contains(':') {
        eprintln!("Input static \"{}\" does not contain a semicolon", input);
        std::process::exit(1);
      }

      let mut split = input.split(':');
      converted.push((split.next().unwrap(), split.next().unwrap()))
    }

    converted
  };
  let templates_default = cli.value_of("templates default").unwrap();
  let templates_directory = cli.value_of("templates directory").unwrap();

  let launchpad = Launchpad::prepare()
    .compile_sass(compile_sass)
    .mode(mode)
    .output(output)
    .source(source)
    .statics(statics)
    .templates_default(templates_default)
    .templates_directory(templates_directory)
    .build()?;

  launchpad.take_off()
}
