#![deny(unsafe_code)]
#![warn(
    clippy::cargo,
    clippy::nursery,
    clippy::pedantic,
    clippy::restriction,
    future_incompatible,
    rust_2018_idioms,
    rustdoc::all
)]
#![warn(
    box_pointers,
    macro_use_extern_crate,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_crate_level_docs,
    missing_debug_implementations,
    missing_docs,
    non_ascii_idents,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unaligned_references,
    unreachable_pub,
    unused_crate_dependencies,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences
)]
#![allow(
    clippy::blanket_clippy_restriction_lints,
    clippy::else_if_without_else,
    clippy::exhaustive_enums,
    clippy::future_not_send,
    clippy::implicit_return,
    clippy::missing_inline_in_public_items,
    clippy::multiple_inherent_impl,
    clippy::non_ascii_literal,
    clippy::pattern_type_mismatch,
    clippy::redundant_pub_crate,
    clippy::shadow_reuse,
    clippy::unreachable,
    clippy::wildcard_enum_match_arm,
    rustdoc::private_doc_tests,
    variant_size_differences
)]

//! TODO: top-level documentation

mod builder;
mod certificate;
mod error;
mod quic;

#[cfg(feature = "certificate")]
pub use certificate::generate_self_signed;
pub use certificate::{Certificate, PrivateKey};
pub use error::{Error, Result};
pub use quic::Server;
