use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"
            CREATE TABLE pilots (
                id BIGSERIAL PRIMARY KEY,
                account_id BIGINT NOT NULL REFERENCES accounts(id),
                name TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )
        "#,
        )
        .with_up("CREATE INDEX pilots_by_name ON pilots(lower(name))")
        .with_down("DROP TABLE IF EXISTS pilots")
        .with_up(
            "ALTER TABLE installations ADD COLUMN created_at TIMESTAMPTZ NOT NULL DEFAULT now()",
        )
        .with_down("ALTER TABLE installations DROP COLUMN IF EXISTS created_at")
}
