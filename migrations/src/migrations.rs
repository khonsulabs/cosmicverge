use sqlx_simple_migrator::{Migration, MigrationError};

use crate::connection::pool;

mod migration_0001_accounts;
mod migration_0002_pilots;
mod migration_0003_permissions;

#[must_use]
fn migrations() -> Vec<Migration> {
    vec![
        migration_0001_accounts::migration(),
        migration_0002_pilots::migration(),
        migration_0003_permissions::migration(),
    ]
}

pub async fn run_all() -> Result<(), MigrationError> {
    Migration::run_all(pool(), migrations()).await
}

pub async fn undo_all() -> Result<(), MigrationError> {
    Migration::undo_all(pool(), migrations()).await
}
