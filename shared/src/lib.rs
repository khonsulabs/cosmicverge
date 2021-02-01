mod version;

use basws_shared::{Version, VersionReq};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_PILOTS_PER_ACCOUNT: usize = 2;

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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum OAuthProvider {
    Twitch,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CosmicVergeResponse {
    ServerStatus { connected_pilots: usize },
    AuthenticateAtUrl { url: String },
    Authenticated { user_id: i64, pilots: Vec<Pilot> },
    Unauthenticated,
    PilotChanged(Pilot),

    Error { message: Option<String> },
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