use chrono::{DateTime, Utc};

use crate::{sqlx, DatabaseError, SqlxResultExt};

#[derive(Debug, Clone)]
pub struct Pilot {
    pub id: i64,
    pub account_id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl Into<cosmicverge_shared::Pilot> for Pilot {
    fn into(self) -> cosmicverge_shared::Pilot {
        cosmicverge_shared::Pilot {
            id: self.id,
            created_at: self.created_at,
            name: self.name,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PilotError {
    #[error("invalid name")]
    InvalidName,
    #[error("name already taken")]
    NameAlreadyTaken,
    #[error("sql error {0}")]
    Database(#[from] DatabaseError),
}

impl From<sqlx::Error> for PilotError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(DatabaseError::from(err))
    }
}

impl Pilot {
    pub async fn create<
        'e,
        E: sqlx::Acquire<
            'e,
            Database = sqlx::Postgres,
            Connection = sqlx::pool::PoolConnection<sqlx::Postgres>,
        >,
    >(
        account_id: i64,
        name: &str,
        executor: E,
    ) -> Result<Self, PilotError> {
        let mut e = executor.acquire().await?;
        let name = Self::validate_and_clean_name(name, &mut e).await?;
        sqlx::query_as!(
        Self,
            "INSERT INTO pilots (account_id, name) VALUES ($1, $2) RETURNING id, account_id, name, created_at",
            account_id,
            name
        )
            .fetch_one(&mut e)
            .await.map_err(|e|e.into())
    }

    pub async fn load<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        id: i64,
        executor: E,
    ) -> Result<Option<Self>, DatabaseError> {
        sqlx::query_as!(
            Self,
            "SELECT id, account_id, name, created_at FROM pilots WHERE id = $1",
            id
        )
        .fetch_one(executor)
        .await
        .map_database_error()
    }

    pub async fn find_by_name<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        name: &str,
        executor: E,
    ) -> Result<Option<Self>, DatabaseError> {
        sqlx::query_as!(
            Self,
            "SELECT id, account_id, name, created_at FROM pilots WHERE lower(name) = lower($1)",
            name
        )
        .fetch_one(executor)
        .await
        .map_database_error()
    }

    pub async fn list_by_account_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        account_id: i64,
        executor: E,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "SELECT id, account_id, name, created_at FROM pilots WHERE account_id = $1",
            account_id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn validate_and_clean_name<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        name: &str,
        executor: E,
    ) -> Result<String, PilotError> {
        let name = cosmicverge_shared::Pilot::cleanup_name(name).map_err(|_| PilotError::InvalidName)?;
        if Self::find_by_name(&name, executor).await?.is_some() {
            return Err(PilotError::NameAlreadyTaken);
        }

        Ok(name)
    }
}

pub fn convert_db_pilots(pilots: Vec<Pilot>) -> Vec<cosmicverge_shared::Pilot> {
    pilots.into_iter().map(|p| p.into()).collect()
}
