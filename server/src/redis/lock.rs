use std::borrow::Cow;

use redis::{aio::MultiplexedConnection, RedisError};

pub struct Lock {
    key: Cow<'static, str>,
    expire_after_msec: i32,
}

impl Lock {
    pub fn new(name: String) -> Self {
        Self {
            key: Cow::from(name),
            expire_after_msec: 1000,
        }
    }

    pub fn named(name: &'static str) -> Self {
        Self {
            key: Cow::from(name),
            expire_after_msec: 1000,
        }
    }

    pub const fn expire_after_secs(mut self, expire_after: i32) -> Self {
        self.expire_after_msec = expire_after * 1000;
        self
    }

    pub const fn expire_after_msecs(mut self, expire_after: i32) -> Self {
        self.expire_after_msec = expire_after;
        self
    }

    pub async fn acquire(
        &self,
        connection: &mut MultiplexedConnection,
    ) -> Result<bool, RedisError> {
        redis::cmd("SET")
            .arg(&[
                &self.key,
                "locked",
                "PX",
                &self.expire_after_msec.to_string(),
                "NX",
            ])
            .query_async(connection)
            .await
    }
}
