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
