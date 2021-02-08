use basws_shared::{Version, VersionReq};
use chrono::{DateTime, Utc};
use euclid::{Point2D, Vector2D, Angle, approxeq::ApproxEq};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ships::ShipId,
    solar_systems::{Solar, SolarSystemId},
};

pub fn cosmic_verge_protocol_version() -> Version {
    Version::parse("0.0.1").unwrap()
}

pub fn cosmic_verge_protocol_version_requirements() -> VersionReq {
    VersionReq::parse("=0.0.1").unwrap()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CosmicVergeRequest {
    AuthenticationUrl(OAuthProvider),
    SelectPilot(i64),
    CreatePilot { name: String },

    Fly(PilotingAction),
    GetPilotInformation(i64),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum OAuthProvider {
    Twitch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CosmicVergeResponse {
    ServerStatus {
        connected_pilots: usize,
    },
    AuthenticateAtUrl {
        url: String,
    },
    Authenticated {
        user_id: i64,
        pilots: Vec<Pilot>,
    },
    Unauthenticated,
    PilotChanged(ActivePilot),
    SpaceUpdate {
        timestamp: i64,
        location: PilotLocation,
        action: PilotingAction,
        ships: Vec<PilotedShip>,
    },
    PilotInformation(Pilot),

    Error {
        message: Option<String>,
    },
}

impl CosmicVergeResponse {
    pub fn error(key: &str) -> Self {
        Self::Error {
            message: Some(key.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct Pilot {
    pub id: i64,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Installation {
    pub id: Uuid,
    pub account_id: Option<i64>,
    pub nonce: Option<Vec<u8>>,
    pub private_key: Option<Vec<u8>>,
}

pub type SolarSystemLocationId = i64;

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
                _ => false
            },
            Self::Docked(self_location) => match other {
                Self::Docked(other_location) => self_location == other_location,
                _ => false
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
    pub pilot_id: i64,
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

