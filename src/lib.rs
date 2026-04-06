//! Lemming

#![warn(clippy::all, rust_2018_idioms)]
#![deny(
    missing_docs,
    clippy::all,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cargo,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic
)]
#![warn(clippy::multiple_crate_versions)]

pub(crate) mod app;
pub(crate) mod central_panel;
pub(crate) mod format;

pub use app::LemmingApp;
