use std::{collections::HashMap, sync::RwLock};

use basws_yew::{prelude::*, ClientLogic, ClientState, Error};
use cosmicverge_shared::protocol::{
    cosmic_verge_protocol_version, CosmicVergeRequest, CosmicVergeResponse, Pilot, PilotId,
};
use once_cell::sync::OnceCell;
use url::Url;

pub type AgentMessage = basws_yew::AgentMessage<CosmicVergeRequest>;
pub type AgentResponse = basws_yew::AgentResponse<CosmicVergeResponse>;
pub type ApiAgent = basws_yew::ApiAgent<CosmicVergeApiClient>;
pub type ApiBridge = basws_yew::ApiBridge<CosmicVergeApiClient>;

static PILOT_CACHE: OnceCell<RwLock<HashMap<PilotId, Pilot>>> = OnceCell::new();

fn cache_pilot_information(pilot: Pilot) {
    let mut cache = PILOT_CACHE
        .get_or_init(|| RwLock::new(Default::default()))
        .write()
        .unwrap();
    cache.insert(pilot.id, pilot);
}

pub fn pilot_information(pilot_id: PilotId, api: &mut ApiBridge) -> Option<Pilot> {
    if let Some(cache) = PILOT_CACHE.get() {
        let cache = cache.read().unwrap();
        if let Some(pilot) = cache.get(&pilot_id) {
            return Some(pilot.clone());
        }
    }

    api.send(AgentMessage::Request(
        CosmicVergeRequest::GetPilotInformation(pilot_id),
    ));
    None
}

#[derive(Debug, Default)]
pub struct CosmicVergeApiClient;

impl ClientLogic for CosmicVergeApiClient {
    type Request = CosmicVergeRequest;
    type Response = CosmicVergeResponse;

    #[cfg(debug_assertions)]
    fn server_url(&self) -> Url {
        Url::parse("ws://10.0.0.130:7879/v1/ws").unwrap()
    }

    #[cfg(not(debug_assertions))]
    fn server_url(&self) -> Url {
        Url::parse("wss://cosmicverge.com/v1/ws").unwrap()
    }

    fn protocol_version(&self) -> Version {
        cosmic_verge_protocol_version()
    }

    fn state_changed(&self, _state: &ClientState) -> anyhow::Result<()> {
        Ok(())
    }

    fn response_received(
        &mut self,
        response: Self::Response,
        _original_request_id: Option<u64>,
    ) -> anyhow::Result<()> {
        match response {
            CosmicVergeResponse::AuthenticateAtUrl { url } => {
                let window = web_sys::window().expect("Need a window");
                window
                    .location()
                    .set_href(&url)
                    .expect("Error setting location for redirect");
            }
            CosmicVergeResponse::PilotInformation(pilot) => {
                cache_pilot_information(pilot);
            }
            CosmicVergeResponse::Error { message } => error!("Error from server: {:?}", message),
            _ => {}
        }

        Ok(())
    }

    fn handle_error(&self, error: Error) -> anyhow::Result<()> {
        error!("Received error: {:?}", error);
        Ok(())
    }
}
