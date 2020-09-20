/// All things belonging to [`Document`](document::Document).
pub mod document;

/// All things belonging to [`Launchpad`].
pub mod launchpad;

/// Markdown handling and rendering functionality.
pub mod markdown;

/// Syntax highlighting functionality.
pub mod syntax_highlighting;

/// Extra templating functionality for [`tera`].
pub mod templating;

pub use launchpad::Launchpad;
