use once_cell::sync::OnceCell;
use sqlx::PgPool;
use std::env;

static POOL: OnceCell<PgPool> = OnceCell::new();

pub fn pool() -> &'static PgPool {
    POOL.get().expect("uninitialized pool access")
}

pub async fn initialize() {
    if POOL.get().is_none() {
        let pool = PgPool::connect(&env::var("DATABASE_URL").expect("DATABASE_URL not set"))
            .await
            .expect("Error initializing postgres pool");
        let _ = POOL.set(pool).unwrap();
    }
}
