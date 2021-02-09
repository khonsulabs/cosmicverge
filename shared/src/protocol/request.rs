use serde::{Deserialize, Serialize};

use crate::protocol::{navigation::PilotingAction, OAuthProvider, PilotId};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CosmicVergeRequest {
    AuthenticationUrl(OAuthProvider),
    SelectPilot(PilotId),
    CreatePilot { name: String },

    Fly(PilotingAction),
    GetPilotInformation(PilotId),
}
