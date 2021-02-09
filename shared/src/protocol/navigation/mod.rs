use std::fmt::Display;

use crate::{
    protocol::{Pilot, PilotId},
    ships::ShipId,
    solar_systems::{Solar, SolarSystemId},
};
use euclid::{approxeq::ApproxEq, Angle, Point2D, Vector2D};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Hash, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SolarSystemLocationId(pub i64);

#[cfg(feature = "redis")]
mod redis {
    use super::SolarSystemLocationId;
    use redis::{FromRedisValue, ToRedisArgs};

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PilotingAction {
    Idle,
    NavigateTo(PilotLocation),
}

impl Default for PilotingAction {
    fn default() -> Self {
        PilotingAction::Idle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PilotedShip {
    pub pilot_id: PilotId,
    pub ship: ShipInformation,
    pub physics: PilotPhysics,
    pub action: PilotingAction,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ActivePilot {
    pub pilot: Pilot,
    pub location: PilotLocation,
    pub action: PilotingAction,
}

#[derive(Default, Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct PilotPhysics {
    pub location: Point2D<f32, Solar>,
    pub rotation: Angle<f32>,
    pub linear_velocity: Vector2D<f32, Solar>,
    pub flight_plan: Option<FlightPlan>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ShipInformation {
    pub ship: ShipId,
    pub mass_of_cargo: f32,
}

impl Default for ShipInformation {
    fn default() -> Self {
        Self {
            ship: ShipId::Shuttle,
            mass_of_cargo: 0.,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct FlightPlan {
    pub made_for: PilotingAction,
    pub elapsed_in_current_maneuver: f32,
    pub initial_position: Point2D<f32, Solar>,
    pub initial_velocity: Vector2D<f32, Solar>,
    pub initial_orientation: Angle<f32>,
    pub maneuvers: Vec<FlightPlanManeuver>,
}

impl FlightPlan {
    pub fn new(ship: &PilotedShip) -> Self {
        Self {
            made_for: ship.action.clone(),
            initial_position: ship.physics.location,
            initial_velocity: ship.physics.linear_velocity,
            initial_orientation: ship.physics.rotation,
            elapsed_in_current_maneuver: 0.,
            maneuvers: Default::default(),
        }
    }

    pub fn last_location_for(&self, ship: &PilotedShip) -> Point2D<f32, Solar> {
        if let Some(last_maneuver) = self.maneuvers.last() {
            last_maneuver.target
        } else {
            ship.physics.location
        }
    }

    pub fn last_rotation_for(&self, ship: &PilotedShip) -> Angle<f32> {
        if let Some(last_maneuver) = self.maneuvers.last() {
            last_maneuver.target_rotation
        } else {
            ship.physics.rotation
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct FlightPlanManeuver {
    pub duration: f32,
    pub target: Point2D<f32, Solar>,
    pub target_rotation: Angle<f32>,
    pub target_velocity: Vector2D<f32, Solar>,
}
