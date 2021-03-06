use chrono::{DateTime, Utc};
use cosmicverge_shared::{
    protocol::{self, pilot},
    MAX_PILOTS_PER_ACCOUNT,
};

use crate::{sqlx, DatabaseError, SqlxResultExt};

#[derive(Debug, Clone)]
pub struct Pilot {
    pub id: i64,
    pub account_id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl From<Pilot> for protocol::Pilot {
    fn from(pilot: Pilot) -> Self {
        Self {
            id: pilot.id(),
            created_at: pilot.created_at,
            name: pilot.name,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid name")]
    InvalidName,
    #[error("name already taken")]
    NameAlreadyTaken,
    #[error("too many pilots")]
    TooManyPilots,
    #[error("sql error {0}")]
    Database(#[from] DatabaseError),
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(DatabaseError::from(err))
    }
}

impl Pilot {
    #[must_use]
    pub const fn id(&self) -> pilot::Id {
        pilot::Id(self.id)
    }

    pub async fn create<
        'e,
        E: sqlx::Acquire<
                'e,
                Database = sqlx::Postgres,
                Connection = sqlx::pool::PoolConnection<sqlx::Postgres>,
            > + Send,
    >(
        account_id: i64,
        name: &str,
        executor: E,
    ) -> Result<Self, Error> {
        let mut e = executor.acquire().await?;
        let existing_pilot_count = sqlx::query!(
            "SELECT count(*) as pilot_count FROM pilots WHERE account_id = $1 GROUP BY account_id",
            account_id
        )
        .fetch_one(&mut e)
        .await
        .map(|r| r.pilot_count)
        .ok()
        .flatten()
        .unwrap_or_default();
        if existing_pilot_count as usize >= MAX_PILOTS_PER_ACCOUNT {
            return Err(Error::TooManyPilots);
        }

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
        id: pilot::Id,
        executor: E,
    ) -> Result<Option<Self>, DatabaseError> {
        sqlx::query_as!(
            Self,
            "SELECT id, account_id, name, created_at FROM pilots WHERE id = $1",
            id.0
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
    ) -> Result<String, Error> {
        let name = protocol::Pilot::cleanup_name(name).map_err(|_| Error::InvalidName)?;
        if Self::find_by_name(&name, executor).await?.is_some() {
            return Err(Error::NameAlreadyTaken);
        }

        Ok(name)
    }
}

#[must_use]
pub fn convert_db_pilots(pilots: Vec<Pilot>) -> Vec<protocol::Pilot> {
    pilots.into_iter().map(|p| p.into()).collect()
}
