use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"
            CREATE TABLE accounts (
                id BIGSERIAL PRIMARY KEY,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )
        "#,
        )
        .with_up("INSERT INTO accounts DEFAULT VALUES")
        .with_down("DROP TABLE IF EXISTS accounts")
        .with_up(
            r#"
            CREATE TABLE installations (
                id UUID PRIMARY KEY,
                account_id BIGINT NULL REFERENCES accounts(id),
                nonce BYTEA NULL,
                private_key BYTEA NULL
            )
        "#,
        )
        .with_up(
            r#"
            CREATE TABLE oauth_tokens (
                account_id BIGINT NOT NULL REFERENCES accounts(id),
                service TEXT NOT NULL,
                refresh_token TEXT,
                access_token TEXT NOT NULL,
                expires TIMESTAMP NULL,
                PRIMARY KEY (service, account_id)
            )
        "#,
        )
        .with_down(
            r#"
            DROP TABLE IF EXISTS oauth_tokens
        "#,
        )
        .with_up(
            r#"
            CREATE TABLE twitch_profiles (
                id TEXT PRIMARY KEY,
                account_id BIGINT NOT NULL REFERENCES accounts(id),
                username TEXT NOT NULL
            )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS twitch_profiles")
}
