use euclid::{Angle, Point2D, Vector2D};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::{PilotedShip, PilotingAction},
    solar_systems::{Solar, SolarSystemId},
};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct FlightPlan {
    pub made_for: PilotingAction,
    pub elapsed_in_current_maneuver: f32,
    pub initial_system: SolarSystemId,
    pub initial_position: Point2D<f32, Solar>,
    pub initial_velocity: Vector2D<f32, Solar>,
    pub initial_orientation: Angle<f32>,
    pub maneuvers: Vec<FlightPlanManeuver>,
}

impl FlightPlan {
    pub fn new(ship: &PilotedShip, current_system: SolarSystemId) -> Self {
        Self {
            made_for: ship.action.clone(),
            initial_system: current_system,
            initial_position: ship.physics.location,
            initial_velocity: ship.physics.linear_velocity,
            initial_orientation: ship.physics.rotation,
            elapsed_in_current_maneuver: 0.,
            maneuvers: Default::default(),
        }
    }

    pub fn last_system(&self) -> SolarSystemId {
        if let Some(maneuver) = self.maneuvers.last() {
            maneuver.system
        } else {
            self.initial_system
        }
    }

    pub fn last_location_for(&self, ship: &PilotedShip) -> Point2D<f32, Solar> {
        if let Some(maneuver) = self.maneuvers.last() {
            maneuver.target
        } else {
            ship.physics.location
        }
    }

    pub fn last_velocity_for(&self, ship: &PilotedShip) -> Vector2D<f32, Solar> {
        if let Some(maneuver) = self.maneuvers.last() {
            maneuver.target_velocity
        } else {
            ship.physics.linear_velocity
        }
    }

    pub fn last_rotation_for(&self, ship: &PilotedShip) -> Angle<f32> {
        if let Some(maneuver) = self.maneuvers.last() {
            maneuver.target_rotation
        } else {
            ship.physics.rotation
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct FlightPlanManeuver {
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
