use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Hash, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Id(pub i64);

#[cfg(feature = "redis")]
mod redis {
    use redis::{FromRedisValue, ToRedisArgs};

    use super::Id;

    impl FromRedisValue for Id {
        fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
            let value = i64::from_redis_value(v)?;
            Ok(Self(value))
        }
    }

    impl ToRedisArgs for Id {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            self.0.write_redis_args(out)
        }
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Pilot {
    pub id: Id,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(thiserror::Error, Debug)]
pub enum NameError {
    #[error("invalid character")]
    InvalidCharacter,
    #[error("too long")]
    TooLong,
}

impl Pilot {
    // TODO unit test
    pub fn cleanup_name(name: &str) -> Result<String, NameError> {
        enum ParseState {
            InWord,
            AfterSpace,
        }
        let name = name.trim();
        let mut cleaned = String::with_capacity(name.len());
        let mut parse_state = None;
        for c in name.chars() {
            // TODO: whitelist specific unicode ranges
            if c.is_ascii_alphanumeric() {
                parse_state = Some(ParseState::InWord);
            } else if c == ' ' {
                // Skip sequential spaces
                if matches!(parse_state, Some(ParseState::AfterSpace)) {
                    continue;
                }
                parse_state = Some(ParseState::AfterSpace);
            } else {
                return Err(NameError::InvalidCharacter);
            }

            cleaned.push(c)
        }

        if cleaned.len() > 40 {
            Err(NameError::TooLong)
        } else {
            Ok(cleaned)
        }
    }
}
