use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Hash, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PilotId(pub i64);

#[cfg(feature = "redis")]
mod redis {
    use redis::{FromRedisValue, ToRedisArgs};

    use super::PilotId;

    impl FromRedisValue for PilotId {
        fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
            let value = i64::from_redis_value(v)?;
            Ok(Self(value))
        }
    }

    impl ToRedisArgs for PilotId {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            self.0.write_redis_args(out)
        }
    }
}

impl Display for PilotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Pilot {
    pub id: PilotId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(thiserror::Error, Debug)]
pub enum PilotNameError {
    #[error("invalid character")]
    InvalidCharacter,
    #[error("too long")]
    TooLong,
}

impl Pilot {
    // TODO unit test
    pub fn cleanup_name(name: &str) -> Result<String, PilotNameError> {
        enum ParseState {
            InWord,
            AfterSpace,
        }
        let name = name.trim();
        let mut cleaned = String::with_capacity(name.len());
        let mut parse_state = None;
        for c in name.chars() {
            // TODO: whitelist specific unicode ranges
            if !c.is_ascii_alphanumeric() {
                if c == ' ' {
                    // Skip sequential spaces
                    if matches!(parse_state, Some(ParseState::AfterSpace)) {
                        continue;
                    }
                    parse_state = Some(ParseState::AfterSpace);
                } else {
                    return Err(PilotNameError::InvalidCharacter);
                }
            } else {
                parse_state = Some(ParseState::InWord);
            }

            cleaned.push(c)
        }

        if cleaned.len() > 40 {
            Err(PilotNameError::TooLong)
        } else {
            Ok(cleaned)
        }
    }
}
