use std::collections::{HashMap, HashSet};

use async_channel::Receiver;
use basws_client::prelude::*;
use cosmicverge_shared::protocol::{
    cosmic_verge_protocol_version, ActivePilot, Request, Response,
    OAuthProvider, Pilot, Id, PilotLocation, PilotedShip, Action,
};
use kludgine::runtime::Runtime;

use crate::{database::ClientDatabase, CosmicVergeClient};

use self::broadcast::BroadcastChannel;

mod broadcast;

pub fn initialize(server_url: Url) -> CosmicVergeClient {
    let client = Client::new(ApiClient {
        server_url,
        pilot_information_cache: Default::default(),
        event_emitter: BroadcastChannel::default(),
    });

    let thread_client = client.clone();
    Runtime::spawn(thread_client.run());

    client
}

#[derive(Debug)]
pub struct ApiClient {
    pub server_url: Url,
    pub pilot_information_cache: Handle<PilotInformationCache>,
    event_emitter: BroadcastChannel<ApiEvent>,
}

#[derive(Debug, Default)]
pub struct PilotInformationCache {
    info: HashMap<Id, Pilot>,
    requested: HashSet<Id>,
}

#[derive(Debug, Clone)]
pub enum ApiEvent {
    ConnectedPilotsCountUpdated(usize),
    PilotChanged(ActivePilot),
    SpaceUpdate {
        timestamp: f64,
        location: PilotLocation,
        action: Action,
        ships: Vec<PilotedShip>,
    },
}

impl ApiClient {
    pub async fn event_receiver(&self) -> Receiver<ApiEvent> {
        self.event_emitter.receiver().await
    }
}

#[async_trait]
impl ClientLogic for ApiClient {
    type Request = Request;
    type Response = Response;

    fn server_url(&self) -> Url {
        self.server_url.clone()
    }

    fn protocol_version(&self) -> Version {
        cosmic_verge_protocol_version()
    }

    async fn state_changed(&self, state: &LoginState, _client: Client<Self>) -> anyhow::Result<()> {
        match state {
            LoginState::Disconnected => {
                info!("Disconnected from API server");
            }
            LoginState::Handshaking { .. } => {}
            LoginState::Connected { .. } => {
                info!("Connected to API server");
            }
            LoginState::Error { message } => {
                if let Some(message) = message {
                    error!("Error from API server: {}", message);
                } else {
                    error!("Error received from server");
                }
            }
        }
        Ok(())
    }

    async fn stored_installation_config(&self) -> Option<InstallationConfig> {
        ClientDatabase::installation_config()
    }

    async fn store_installation_config(&self, config: InstallationConfig) -> anyhow::Result<()> {
        ClientDatabase::set_installation_config(&config)?;

        Ok(())
    }

    async fn response_received(
        &self,
        response: Self::Response,
        _original_request_id: Option<u64>,
        client: Client<Self>,
    ) -> anyhow::Result<()> {
        match response {
            Response::ServerStatus { connected_pilots } => {
                let _ = self
                    .event_emitter
                    .send(ApiEvent::ConnectedPilotsCountUpdated(connected_pilots))
                    .await;
            }
            Response::AuthenticateAtUrl { url } => {
                if webbrowser::open(&url).is_err() {
                    error!("Could not open a browser for you. Please open this URL to proceed with authentication: {}", url);
                }
            }
            Response::Authenticated { pilots, .. } => {
                if let Some(pilot) = pilots.first() {
                    info!("Authenticated! Picking the first pilot because avoiding UI for now");
                    client
                        .request(Request::SelectPilot(pilot.id))
                        .await?;
                } else {
                    info!("Authenticated! But, you have no pilots. Create one in the browser at https://cosmicverge.com/ and come back");
                }
            }
            Response::Unauthenticated => {
                info!("Not authenticated, forcing you to try authenticating at twitch!");
                client
                    .request(Request::AuthenticationUrl(OAuthProvider::Twitch))
                    .await?;
            }
            Response::PilotChanged(pilot) => {
                let _ = self.event_emitter.send(ApiEvent::PilotChanged(pilot)).await;
            }
            Response::SpaceUpdate {
                timestamp,
                location,
                action,
                ships,
            } => {
                let _ = self
                    .event_emitter
                    .send(ApiEvent::SpaceUpdate {
                        timestamp,
                        location,
                        action,
                        ships,
                    })
                    .await;
            }
            Response::PilotInformation(pilot) => {
                let mut cache = self.pilot_information_cache.write().await;
                cache.info.insert(pilot.id, pilot);
            }
            Response::Error { message } => {
                error!("Error from API: {:?}", message);
            }
        }
        Ok(())
    }

    async fn handle_error(&self, error: Error, _client: Client<Self>) -> anyhow::Result<()> {
        error!("Api Error: {:?}", error);

        Ok(())
    }
}

impl ApiClient {
    pub async fn pilot_information(
        &self,
        pilot_id: &Id,
        client: &Client<Self>,
    ) -> Option<Pilot> {
        {
            let cache = self.pilot_information_cache.read().await;
            if let Some(info) = cache.info.get(pilot_id) {
                return Some(info.clone());
            } else if cache.requested.contains(pilot_id) {
                // Already requested, don't spam the server with more requests
                return None;
            }
        }

        let mut cache = self.pilot_information_cache.write().await;
        if !cache.requested.contains(pilot_id) {
            cache.requested.insert(*pilot_id);
            let _ = client
                .request(Request::GetPilotInformation(*pilot_id))
                .await;
        }
        None
    }
}
