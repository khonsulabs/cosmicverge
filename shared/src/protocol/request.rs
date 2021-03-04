use serde::{Deserialize, Serialize};

use crate::protocol::{navigation::Action, Id, OAuthProvider};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Request {
    AuthenticationUrl(OAuthProvider),
    SelectPilot(Id),
    CreatePilot { name: String },

    Fly(Action),
    GetPilotInformation(Id),
}
