use serde::{Deserialize, Serialize};

use crate::protocol::{ActivePilot, Pilot, PilotLocation, PilotedShip, PilotingAction};

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
