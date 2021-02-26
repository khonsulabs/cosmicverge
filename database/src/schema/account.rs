use basws_server::prelude::Uuid;
use migrations::sqlx;

#[derive(Debug, Clone)]
pub struct Account {
    pub id: i64,
}

impl Account {
    pub async fn find_by_installation_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        installation_id: Uuid,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error> {
        match sqlx::query_as!(
            Self,
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

    pub async fn find_by_twitch_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        twitch_id: &str,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error> {
        match sqlx::query_as!(
                Self,
                "SELECT accounts.id FROM accounts INNER JOIN twitch_profiles ON twitch_profiles.account_id = accounts.id WHERE twitch_profiles.id = $1",
                twitch_id
            )
            .fetch_one(executor)
            .await {
            Ok(result) => Ok(Some(result)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub async fn load<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        account_id: i64,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error> {
        match sqlx::query_as!(
            Self,
            "SELECT accounts.id FROM accounts WHERE accounts.id = $1",
            account_id,
        )
        .fetch_one(executor)
        .await
        {
            Ok(result) => Ok(Some(result)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub async fn create<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        executor: E,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(Self, "INSERT INTO accounts DEFAULT VALUES RETURNING id")
            .fetch_one(executor)
            .await
    }
}
