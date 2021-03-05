use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up(
            r#"
            CREATE TABLE permission_groups (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now()
            )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS permission_groups")
        .with_up(
            r#"
            CREATE TABLE permission_group_statements (
                id SERIAL PRIMARY KEY,
                permission_group_id INT NOT NULL REFERENCES permission_groups(id),
                service TEXT NOT NULL,
                permission TEXT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE(permission_group_id, service, permission)
            )
        "#,
        )
        .with_up("CREATE INDEX permission_group_statements_by_group_id ON permission_group_statements(permission_group_id)")
        .with_down("DROP TABLE IF EXISTS permission_group_statements")
        .with_up(
            r#"
            CREATE TABLE account_permission_groups (
                account_id BIGINT NOT NULL REFERENCES accounts(id),
                permission_group_id INT NOT NULL REFERENCES permission_groups(id)
            )
        "#,
        )
        .with_down("DROP TABLE IF EXISTS account_permission_groups")
        .with_up("ALTER TABLE accounts ADD COLUMN superuser BOOL NOT NULL DEFAULT false")
        .with_down("ALTER TABLE accounts DROP COLUMN IF EXISTS superuser")
}
