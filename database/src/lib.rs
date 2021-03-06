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
    clippy::missing_panics_doc,
    clippy::multiple_crate_versions,
    clippy::option_if_let_else,
    // Clippy is bugged
    clippy::use_self,
    // Clippy is super bugged
    clippy::used_underscore_binding
)]

pub use ::migrations::{
    initialize, migrations, pool,
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
                return DatabaseError::Conflict;
            }
        }

        DatabaseError::Other(error)
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

#[cfg(test)]
mod test_util {
    use once_cell::sync::Lazy;
    use sqlx::PgPool;
    use tokio::sync::Mutex;

    pub async fn pool() -> &'static PgPool {
        static INITIALIZED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
        let mut initialized = INITIALIZED.lock().await;
        if !*initialized {
            dotenv::dotenv().unwrap();
            migrations::initialize().await;
            *initialized = true;
        }
        migrations::pool()
    }
}
