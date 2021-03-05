use std::{collections::HashMap, sync::RwLock};

use basws_yew::{prelude::*, ClientLogic, ClientState, Error};
use cosmicverge_shared::protocol::{
    cosmic_verge_protocol_version, pilot, Pilot, Request, Response,
};
use once_cell::sync::OnceCell;
use url::Url;

pub type AgentMessage = basws_yew::AgentMessage<Request>;
pub type AgentResponse = basws_yew::AgentResponse<Response>;
pub type ApiAgent = basws_yew::ApiAgent<CosmicVergeApiClient>;
pub type ApiBridge = basws_yew::ApiBridge<CosmicVergeApiClient>;

static PILOT_CACHE: OnceCell<RwLock<HashMap<pilot::Id, Pilot>>> = OnceCell::new();

fn cache_pilot_information(pilot: Pilot) {
    let mut cache = PILOT_CACHE
        .get_or_init(|| RwLock::new(Default::default()))
        .write()
        .unwrap();
    cache.insert(pilot.id, pilot);
}

pub fn pilot_information(pilot_id: pilot::Id, api: &mut ApiBridge) -> Option<Pilot> {
    if let Some(cache) = PILOT_CACHE.get() {
        let cache = cache.read().unwrap();
        if let Some(pilot) = cache.get(&pilot_id) {
            return Some(pilot.clone());
        }
    }

    api.send(AgentMessage::Request(Request::GetPilotInformation(
        pilot_id,
    )));
    None
}

#[derive(Debug, Default)]
pub struct CosmicVergeApiClient;

impl ClientLogic for CosmicVergeApiClient {
    type Request = Request;
    type Response = Response;

    #[cfg(debug_assertions)]
    fn server_url(&self) -> Url {
        Url::parse("ws://localhost:7879/v1/ws").unwrap()
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
            Response::AuthenticateAtUrl { url } => {
                let window = web_sys::window().expect("Need a window");
                window
                    .location()
                    .set_href(&url)
                    .expect("Error setting location for redirect");
            }
            Response::PilotInformation(pilot) => {
                cache_pilot_information(pilot);
            }
            Response::Error { message } => error!("Error from server: {:?}", message),
            _ => {}
        }

        Ok(())
    }

    fn handle_error(&self, error: Error) -> anyhow::Result<()> {
        error!("Received error: {:?}", error);
        Ok(())
    }
}
