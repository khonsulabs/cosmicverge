use sqlx_simple_migrator::{migration_name, Migration};

pub fn migration() -> Migration {
    Migration::new(migration_name!())
        .with_up("CREATE TYPE log_level AS ENUM('error', 'warning', 'info', 'debug', 'trace');")
        .with_down("DROP TYPE IF EXISTS log_level;")
        .with_up(
            r#"
            CREATE TABLE logs (
                id SERIAL PRIMARY KEY,
                level log_level NOT NULL,
                process TEXT NOT NULL,
                message TEXT NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                payload JSONB NULL
            ) WITH (FILLFACTOR=90)
        "#,
        )
        .with_up("CREATE INDEX logs_by_timestamp ON logs(timestamp, message)")
        .with_up("CLUSTER logs USING logs_by_timestamp")
        .with_down("DROP TABLE IF EXISTS logs")
}
