use std::fmt::Display;

use euclid::{approxeq::ApproxEq, Point2D};
use serde::{Deserialize, Serialize};

pub use self::{flight_plan::*, piloting::*};
use crate::solar_systems::{Solar, SolarSystemId};

mod flight_plan;
mod piloting;

#[derive(Debug, Copy, Hash, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SolarSystemLocationId(pub i64);

#[cfg(feature = "redis")]
mod redis {
    use redis::{FromRedisValue, ToRedisArgs};

    use super::SolarSystemLocationId;

    impl FromRedisValue for SolarSystemLocationId {
        fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
            let value = i64::from_redis_value(v)?;
            Ok(Self(value))
        }
    }

    impl ToRedisArgs for SolarSystemLocationId {
        fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite,
        {
            self.0.write_redis_args(out)
        }
    }
}

impl Display for SolarSystemLocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PilotLocation {
    pub system: SolarSystemId,
    pub location: SolarSystemLocation,
}

impl Default for PilotLocation {
    fn default() -> Self {
        Self {
            system: SolarSystemId::SM0A9F4,
            location: SolarSystemLocation::InSpace(Default::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SolarSystemLocation {
    InSpace(Point2D<f32, Solar>),
    Docked(SolarSystemLocationId),
}

impl PartialEq for SolarSystemLocation {
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
