use basws_server::prelude::{InstallationConfig, Uuid};
use chrono::{DateTime, Utc};
use migrations::sqlx;

use crate::pool;

#[derive(Debug)]
pub struct Installation {
    pub id: Uuid,
    pub account_id: Option<i64>,
    pub nonce: Option<Vec<u8>>,
    pub private_key: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
}

impl Installation {
    pub async fn load_or_create(installation_id: Option<Uuid>) -> Result<Self, sqlx::Error> {
        if let Some(installation_id) = installation_id {
            match sqlx::query_as!(
                Self,
                "SELECT id, account_id, nonce, private_key, created_at FROM installations WHERE id = $1",
                installation_id
            )
            .fetch_one(pool())
            .await
            {
                Ok(installation) => {
                    if installation.private_key.is_some() {
                        return Ok(installation);
                    }
                }
                Err(sqlx::Error::RowNotFound) => {}
                Err(err) => return Err(err),
            }
        }

        Self::create(pool()).await
    }

    pub async fn create<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        executor: E,
    ) -> Result<Self, sqlx::Error> {
        let default_config = InstallationConfig::default();
        sqlx::query_as!(
        Self,
        "INSERT INTO installations (id, private_key) VALUES ($1, $2) RETURNING id, account_id, nonce, private_key, created_at",
        default_config.id, Vec::from(default_config.private_key)
    )
            .fetch_one(executor)
            .await
    }

    pub async fn set_nonce<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &self,
        nonce: Option<Vec<u8>>,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE installations SET nonce=$2 WHERE id = $1",
            self.id,
            nonce
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn set_account_id_for_installation_id<
        'e,
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    >(
        installation_id: Uuid,
        account_id: Option<i64>,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE installations SET account_id = $1, nonce = NULL WHERE id = $2",
            account_id,
            installation_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn set_account_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &mut self,
        account_id: Option<i64>,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        self.account_id = account_id;
        Self::set_account_id_for_installation_id(self.id, account_id, executor).await
    }
}
