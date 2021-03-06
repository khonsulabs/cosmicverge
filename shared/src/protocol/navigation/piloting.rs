use euclid::{Angle, Point2D, Vector2D};
use serde::{Deserialize, Serialize};

use crate::{
    protocol::{navigation, pilot, Pilot},
    ships,
    solar_systems::{Solar, SolarSystemId},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Idle,
    NavigateTo(navigation::Universe),
}

impl Default for Action {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ship {
    pub pilot_id: pilot::Id,
    pub ship: ShipInformation,
    pub physics: Physics,
    pub action: Action,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ActivePilot {
    pub pilot: Pilot,
    pub location: navigation::Universe,
    pub action: Action,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Physics {
    pub system: SolarSystemId,
    pub location: Point2D<f32, Solar>,
    pub rotation: Angle<f32>,
    pub linear_velocity: Vector2D<f32, Solar>,
    pub flight_plan: Option<navigation::Plan>,
    pub effect: Option<ShipEffect>,
}

#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub enum ShipEffect {
    Thrusting,
    Jumping,
}

impl Default for Physics {
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
    pub ship: ships::Id,
    pub mass_of_cargo: f32,
}

impl Default for ShipInformation {
    fn default() -> Self {
        Self {
            ship: ships::Id::Shuttle,
            mass_of_cargo: 0.,
        }
    }
}
