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
    clippy::cast_sign_loss,
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    // clippy::missing_panics_doc, // not on stable yet
    clippy::multiple_crate_versions,
    clippy::option_if_let_else,
    // Clippy is bugged
    clippy::similar_names, // This is not buggy on nightly, so it should be re-checked. affect sql::query!() false positives
    // Clippy is super bugged
    clippy::used_underscore_binding
)]

pub use ::migrations::{
    self, initialize, pool,
    sqlx::{self, database::HasStatement, Database, Execute, Executor},
};
pub use basws_server;
pub use cosmicverge_shared;

pub mod schema;

#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error("conflict")]
    Conflict,

    #[error("other sql error: {0}")]
    Other(sqlx::Error),
}

impl From<sqlx::Error> for DatabaseError {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::Database(database_error) = &error {
            if database_error
                .code()
                .map(|c| c == "23505")
                .unwrap_or_default()
            {
                return Self::Conflict;
            }
        }

        Self::Other(error)
    }
}

pub trait SqlxResultExt<T> {
    fn map_database_error(self) -> Result<Option<T>, DatabaseError>;
}

impl<T> SqlxResultExt<T> for Result<T, sqlx::Error> {
    fn map_database_error(self) -> Result<Option<T>, DatabaseError> {
        match self {
            Ok(result) => Ok(Some(result)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(other) => Err(DatabaseError::from(other)),
        }
    }
}

pub mod test_util {
    use std::env;

    use once_cell::sync::Lazy;
    use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

    #[derive(Default)]
    pub struct DatabaseStatus {
        initialized: bool,
    }
    static DATABASE_STATUS: Lazy<RwLock<DatabaseStatus>> =
        Lazy::new(|| RwLock::new(DatabaseStatus::default()));

    fn database_status() -> &'static RwLock<DatabaseStatus> {
        &*DATABASE_STATUS
    }

    async fn initialize() {
        {
            if database_status().read().await.initialized {
                return;
            }
        }

        let mut status = database_status().write().await;
        if !status.initialized {
            dotenv::dotenv().unwrap();
            migrations::initialize(Some(
                env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL not set"),
            ))
            .await;
            status.initialized = true;

            // Make sure the database is clean to begin with.
            wipe_database().await;
        }
    }

    async fn wipe_database() {
        migrations::undo_all().await.unwrap();
        migrations::run_all().await.unwrap();
    }

    // Execute a database test that is considered "safe", which means that it uses transactions to ensure it can't leak or conflict with any other ongoing test
    pub async fn initialize_safe_test() -> RwLockReadGuard<'static, DatabaseStatus> {
        initialize().await;
        database_status().read().await
    }

    // Execute a database test that is considered "unsafe", which means that it
    // may leak data if a test fails. The guard returned is exclusive, which
    // prevents any other database test from running while this is executing
    pub async fn initialize_exclusive_test() -> RwLockWriteGuard<'static, DatabaseStatus> {
        initialize().await;
        let guard = database_status().write().await;

        wipe_database().await;

        guard
    }
}
