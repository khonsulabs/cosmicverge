use euclid::{Angle, Point2D, Vector2D};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::{FlightPlan, Id, Pilot, PilotLocation},
    ships::ShipId,
    solar_systems::{Solar, SolarSystemId},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Idle,
    NavigateTo(PilotLocation),
}

impl Default for Action {
    fn default() -> Self {
        Action::Idle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PilotedShip {
    pub pilot_id: Id,
    pub ship: ShipInformation,
    pub physics: PilotPhysics,
    pub action: Action,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ActivePilot {
    pub pilot: Pilot,
    pub location: PilotLocation,
    pub action: Action,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct PilotPhysics {
    pub system: SolarSystemId,
    pub location: Point2D<f32, Solar>,
    pub rotation: Angle<f32>,
    pub linear_velocity: Vector2D<f32, Solar>,
    pub flight_plan: Option<FlightPlan>,
    pub effect: Option<ShipEffect>,
}

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum ShipEffect {
    Thrusting,
    Jumping,
}

impl Default for PilotPhysics {
    fn default() -> Self {
        Self {
            system: SolarSystemId::SM0A9F4,
            location: Point2D::default(),
            rotation: Angle::default(),
            linear_velocity: Vector2D::default(),
            flight_plan: None,
            effect: None,
        }
    }
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
