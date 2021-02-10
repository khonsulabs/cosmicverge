use euclid::{Angle, Point2D, Vector2D};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::{FlightPlan, Pilot, PilotId, PilotLocation},
    ships::ShipId,
    solar_systems::{Solar, SolarSystemId},
};

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
            location: Default::default(),
            rotation: Default::default(),
            linear_velocity: Default::default(),
            flight_plan: Default::default(),
            effect: Default::default(),
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
