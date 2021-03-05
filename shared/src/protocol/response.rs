use serde::{Deserialize, Serialize};

use crate::protocol::{navigation, Pilot};

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
    PilotChanged(navigation::ActivePilot),
    SpaceUpdate {
        timestamp: f64,
        location: navigation::Pilot,
        action: navigation::Action,
        ships: Vec<navigation::Ship>,
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
