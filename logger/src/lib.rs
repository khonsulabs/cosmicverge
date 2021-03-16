#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    // clippy::missing_docs_in_private_items,
    clippy::nursery,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms,
    // missing_docs
)]
#![cfg_attr(doc, warn(rustdoc))]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::multiple_crate_versions,
    // clippy::missing_panics_doc, // not on stable yet
    clippy::option_if_let_else,
)]

pub mod backend;
mod log;
mod manager;
pub mod tracing;

pub use self::{log::*, manager::*, tracing::*};

mod macros;
#[cfg(test)]
mod test_util;
