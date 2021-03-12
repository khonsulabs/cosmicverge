use migrations::sqlx;

pub struct TwitchProfile;

impl TwitchProfile {
    pub async fn associate<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        twitch_id: &str,
        account_id: i64,
        username: &str,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("INSERT INTO twitch_profiles (id, account_id, username) VALUES ($1, $2, $3) ON CONFLICT (id) DO UPDATE SET account_id = $2, username = $3 ",
            twitch_id,
            account_id,
            username,
        ).execute(executor).await.map(|_| ())
    }
    pub async fn delete<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        twitch_id: &str,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM twitch_profiles WHERE id = $1", twitch_id,)
            .execute(executor)
            .await
            .map(|_| ())
    }
}
