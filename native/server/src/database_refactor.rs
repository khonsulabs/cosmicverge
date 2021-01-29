use basws_server::prelude::{InstallationConfig, Uuid};
use cosmicverge_shared::{
    Installation, UserProfile,
};

use database::{pool, sqlx};

use chrono::{DateTime, Utc};

pub async fn get_profile_by_installation_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
    executor: E,
    installation_id: Uuid,
) -> Result<Option<UserProfile>, sqlx::Error>
{
    match sqlx::query_as!(
        UserProfile,
        "SELECT accounts.id FROM accounts INNER JOIN installations ON installations.account_id = accounts.id WHERE installations.id = $1",
        installation_id,
    )
        .fetch_one(executor)
        .await {
        Ok(result) => Ok(Some(result)),
        Err(sqlx::Error::RowNotFound) => Ok(None),
        Err(err) => Err(err)
    }
}

pub async fn get_profile_by_account_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
    executor: E,
    account_id: i64,
) -> Result<Option<UserProfile>, sqlx::Error>
{
    match sqlx::query_as!(
        UserProfile,
        "SELECT accounts.id FROM accounts WHERE accounts.id = $1",
        account_id,
    )
        .fetch_one(executor)
        .await {
        Ok(result) => Ok(Some(result)),
        Err(sqlx::Error::RowNotFound) => Ok(None),
        Err(err) => Err(err)
    }
}

pub async fn lookup_or_create_installation(
    installation_id: Option<Uuid>,
) -> Result<Installation, sqlx::Error>
{
    if let Some(installation_id) = installation_id {
        match sqlx::query_as!(
            Installation,
            "SELECT id, account_id, nonce, private_key FROM installations WHERE id = $1",
            installation_id
        )
            .fetch_one(pool())
            .await {
            Ok(installation) => if installation.private_key.is_some() {
                return Ok(installation);
            },
            Err(sqlx::Error::RowNotFound) => {},
            Err(err) => return Err(err),
        }
    }

    create_installation(pool()).await
}

async fn create_installation<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
    executor: E,
) -> Result<Installation, sqlx::Error>
{
    println!("Creating installation");
    let default_config = InstallationConfig::default();
    sqlx::query_as!(
        Installation,
        "INSERT INTO installations (id, private_key) VALUES ($1, $2) RETURNING id, account_id, nonce, private_key",
        default_config.id, Vec::from(default_config.private_key)
    )
        .fetch_one(executor)
        .await
}

pub async fn set_installation_nonce<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
    executor: E,
    installation_id: Uuid,
    nonce: Option<Vec<u8>>,
) -> Result<(), sqlx::Error>
{
    sqlx::query!(
        "UPDATE installations SET nonce=$2 WHERE id = $1",
        installation_id,
        nonce
    )
        .execute(executor)
        .await?;
    Ok(())
}

pub async fn set_installation_account_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
    executor: E,
    installation_id: Uuid,
    account_id: Option<i64>,
) -> Result<(), sqlx::Error>
{
    sqlx::query!(
        "UPDATE installations SET account_id = $1, nonce = NULL WHERE id = $2",
        account_id,
        installation_id
    )
        .execute(executor)
        .await?;
    Ok(())
}
