use euclid::{Angle, Point2D, Vector2D};
use serde::{Deserialize, Serialize};

use super::{Action, Ship};
use crate::solar_systems::{Solar, SolarSystemId};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub made_for: Action,
    pub elapsed_in_current_maneuver: f32,
    pub initial_system: SolarSystemId,
    pub initial_position: Point2D<f32, Solar>,
    pub initial_velocity: Vector2D<f32, Solar>,
    pub initial_orientation: Angle<f32>,
    pub maneuvers: Vec<Maneuver>,
}

impl Plan {
    #[must_use]
    pub fn new(ship: &Ship, current_system: SolarSystemId) -> Self {
        Self {
            made_for: ship.action.clone(),
            initial_system: current_system,
            initial_position: ship.physics.location,
            initial_velocity: ship.physics.linear_velocity,
            initial_orientation: ship.physics.rotation,
            elapsed_in_current_maneuver: 0.,
            maneuvers: Vec::default(),
        }
    }

    #[must_use]
    pub fn last_system(&self) -> SolarSystemId {
        if let Some(maneuver) = self.maneuvers.last() {
            maneuver.system
        } else {
            self.initial_system
        }
    }

    #[must_use]
    pub fn last_location_for(&self, ship: &Ship) -> Point2D<f32, Solar> {
        if let Some(maneuver) = self.maneuvers.last() {
            maneuver.target
        } else {
            ship.physics.location
        }
    }

    #[must_use]
    pub fn last_velocity_for(&self, ship: &Ship) -> Vector2D<f32, Solar> {
        if let Some(maneuver) = self.maneuvers.last() {
            maneuver.target_velocity
        } else {
            ship.physics.linear_velocity
        }
    }

    #[must_use]
    pub fn last_rotation_for(&self, ship: &Ship) -> Angle<f32> {
        if let Some(maneuver) = self.maneuvers.last() {
            maneuver.target_rotation
        } else {
            ship.physics.rotation
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Maneuver {
    pub kind: ManeuverKind,
    pub system: SolarSystemId,
    pub duration: f32,
    pub target: Point2D<f32, Solar>,
    pub target_rotation: Angle<f32>,
    pub target_velocity: Vector2D<f32, Solar>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ManeuverKind {
    Movement,
    Jump,
}
