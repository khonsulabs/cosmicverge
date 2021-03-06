use std::fmt::Display;

use euclid::{approxeq::ApproxEq, Point2D};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};

use crate::solar_systems::{self, Solar};

#[derive(Debug, Copy, Hash, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Id(pub i64);

impl Id {
    #[must_use]
    pub fn into_location<T: FromPrimitive>(self) -> Option<T> {
        T::from_i64(self.0)
    }
}

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Universe {
    pub system: solar_systems::SolarSystemId,
    pub location: Location,
}

impl Default for Universe {
    fn default() -> Self {
        Self {
            system: solar_systems::SolarSystemId::SM0A9F4,
            location: Location::InSpace(Point2D::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Location {
    InSpace(Point2D<f32, Solar>),
    Docked(Id),
}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::InSpace(self_location) => match other {
                Self::InSpace(other_location) => self_location.approx_eq(other_location),
                _ => false,
            },
            Self::Docked(self_location) => match other {
                Self::Docked(other_location) => self_location == other_location,
                _ => false,
            },
        }
    }
}
