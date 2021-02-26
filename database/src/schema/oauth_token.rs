use migrations::sqlx;

pub struct OAuthToken;

impl OAuthToken {
    pub async fn update<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        account_id: i64,
        service: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("INSERT INTO oauth_tokens (account_id, service, access_token, refresh_token) VALUES ($1, $2, $3, $4) ON CONFLICT (account_id, service) DO UPDATE SET access_token = $3, refresh_token = $4",
            account_id,
            service,
            access_token,
            refresh_token,

        ).execute(executor).await.map(|_|())
    }
}
