use anyhow::Result;
use takeoff::{launchpad::Mode, Launchpad};
use tracing::Level;

fn create_launchpad(mode: Mode, level: Level) -> Result<Launchpad> {
  tracing_subscriber::fmt().with_max_level(level).init();

  let statics = vec![("static/scss/*.scss", "css/")];

  Launchpad::prepare()
    .compile_sass(true)
    .mode(mode)
    .output("../public")
    .source("../docs")
    .statics(statics)
    .templates_default("document.html")
    .templates_directory("templates")
    .build()
}

#[test]
fn test_build_takeoff_website() -> Result<()> {
  create_launchpad(Mode::Production, Level::INFO)?.take_off()
}

#[test]
#[ignore = "Only run this test when developing the Takeoff website."]
fn test_develop_takeoff_website() -> Result<()> {
  create_launchpad(Mode::Development, Level::DEBUG)?.take_off()
}
