pub use ::migrations::{
    initialize, migrations, pool,
    sqlx::{self, database::HasStatement, Database, Execute, Executor},
};

pub mod schema;

#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error("row not found")]
    RowNotFound,
    #[error("conflict")]
    Conflict,

    #[error("other sql error: {0}")]
    Other(sqlx::Error),
}

impl From<sqlx::Error> for DatabaseError {
    fn from(error: sqlx::Error) -> Self {
        match &error {
            sqlx::Error::RowNotFound => return DatabaseError::RowNotFound,
            sqlx::Error::Database(database_error) => {
                if database_error
                    .code()
                    .map(|c| c == "23505")
                    .unwrap_or_default()
                {
                    return DatabaseError::Conflict;
                }
            }
            _ => {}
        }

        DatabaseError::Other(error)
    }
}

pub trait SqlxResultExt<T> {
    fn map_database_error(self) -> Result<T, DatabaseError>;
}

impl<T> SqlxResultExt<T> for Result<T, sqlx::Error> {
    fn map_database_error(self) -> Result<T, DatabaseError> {
        self.map_err(DatabaseError::from)
    }
}
