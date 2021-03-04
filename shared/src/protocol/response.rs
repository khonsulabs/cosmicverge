use serde::{Deserialize, Serialize};

use crate::protocol::{Action, ActivePilot, Pilot, PilotLocation, PilotedShip};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Response {
    ServerStatus {
        connected_pilots: usize,
    },
    AuthenticateAtUrl {
        url: String,
    },
    Authenticated {
        account: Account,
        pilots: Vec<Pilot>,
    },
    Unauthenticated,
    PilotChanged(ActivePilot),
    SpaceUpdate {
        timestamp: f64,
        location: PilotLocation,
        action: Action,
        ships: Vec<PilotedShip>,
    },
    PilotInformation(Pilot),

    Error {
        message: Option<String>,
    },
}

impl Response {
    #[must_use]
    pub fn error(key: &str) -> Self {
        Self::Error {
            message: Some(key.to_string()),
        }
    }
}
