mod migration_0001_accounts;

use crate::connection::pool;
use sqlx_simple_migrator::{Migration, MigrationError};

const JONS_ACCOUNT_ID: i64 = 1;
const JONS_TWITCH_ID: &str = "435235857";

pub fn migrations() -> Vec<Migration> {
    vec![
        migration_0001_accounts::migration(),
    ]
}

pub async fn run_all() -> Result<(), MigrationError> {
    let pool = pool();

    Migration::run_all(pool, migrations()).await
}
