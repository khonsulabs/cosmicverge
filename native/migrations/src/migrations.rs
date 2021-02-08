use sqlx_simple_migrator::{Migration, MigrationError};

use crate::connection::pool;

mod migration_0001_accounts;
mod migration_0002_pilots;

const JONS_ACCOUNT_ID: i64 = 1;
const JONS_TWITCH_ID: &str = "435235857";

pub fn migrations() -> Vec<Migration> {
    vec![
        migration_0001_accounts::migration(),
        migration_0002_pilots::migration(),
    ]
}

pub async fn run_all() -> Result<(), MigrationError> {
    Migration::run_all(pool(), migrations()).await
}
