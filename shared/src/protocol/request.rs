use serde::{Deserialize, Serialize};

use crate::protocol::{navigation::Action, pilot, OAuthProvider};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Request {
    AuthenticationUrl(OAuthProvider),
    SelectPilot(pilot::Id),
    CreatePilot { name: String },

    Fly(Action),
    GetPilotInformation(pilot::Id),
}
