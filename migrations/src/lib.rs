#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    // clippy::missing_docs_in_private_items,
    clippy::nursery,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![cfg_attr(doc, warn(rustdoc))]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    // clippy::missing_panics_doc, // not on stable yet
    clippy::multiple_crate_versions,
    clippy::option_if_let_else,
)]

mod connection;

pub use connection::{initialize, pool};
mod migrations;
pub use self::migrations::run_all;

#[cfg(feature = "test-util")]
pub use self::migrations::undo_all;

pub use sqlx;
pub use sqlx_simple_migrator::Migration;
