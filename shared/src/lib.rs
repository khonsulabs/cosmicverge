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
    clippy::missing_panics_doc,
    clippy::option_if_let_else,
    // Clippy is bugged
    clippy::use_self
)]

//! Shared abstraction that will be re-used throughout the project.

pub use euclid;
pub use num_traits;
pub use strum;
pub use strum_macros;
pub mod permissions;
#[cfg(feature = "persy")]
pub mod persy;
pub mod protocol;
pub mod ships;
pub mod solar_system_simulation;
pub mod solar_systems;
mod version;

/// Maximum amount of pilots that can be created per account.
pub const MAX_PILOTS_PER_ACCOUNT: usize = 2;
