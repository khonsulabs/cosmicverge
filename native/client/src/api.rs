use std::collections::HashMap;

use async_channel::Receiver;
use basws_client::prelude::*;
use cosmicverge_shared::protocol::{
    cosmic_verge_protocol_version, ActivePilot, CosmicVergeRequest, CosmicVergeResponse,
    OAuthProvider, Pilot, PilotId,
};

use crate::{database::ClientDatabase, CosmicVergeClient};

use self::broadcast::BroadcastChannel;

mod broadcast;

pub fn initialize(server_url: Url) -> CosmicVergeClient {
    let client = Client::new(ApiClient {
        server_url,
        connected_pilots_count: Default::default(),
        pilot_information_cache: Default::default(),
        event_emitter: BroadcastChannel::default(),
    });
    let thread_client = client.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(thread_client.run()).unwrap()
    });

    client
}

#[derive(Debug)]
pub struct ApiClient {
    pub server_url: Url,
    connected_pilots_count: Handle<Option<usize>>,
    pilot_information_cache: Handle<HashMap<PilotId, Pilot>>,
    event_emitter: BroadcastChannel<ApiEvent>,
}

#[derive(Debug, Clone)]
pub enum ApiEvent {
    PilotChanged(ActivePilot),
}

impl ApiClient {
    pub async fn connected_pilots_count(&self) -> Option<usize> {
        let connected_pilots_count = self.connected_pilots_count.read().await;
        *connected_pilots_count
    }

    pub async fn event_receiver(&self) -> Receiver<ApiEvent> {
        self.event_emitter.receiver().await
    }
}

#[async_trait]
impl ClientLogic for ApiClient {
    type Request = CosmicVergeRequest;
    type Response = CosmicVergeResponse;

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
            CosmicVergeResponse::ServerStatus { connected_pilots } => {
                let mut connected_pilots_count = self.connected_pilots_count.write().await;
                *connected_pilots_count = Some(connected_pilots);
            }
            CosmicVergeResponse::AuthenticateAtUrl { url } => {
                if webbrowser::open(&url).is_err() {
                    error!("Could not open a browser for you. Please open this URL to proceed with authentication: {}", url);
                }
            }
            CosmicVergeResponse::Authenticated { pilots, .. } => {
                if let Some(pilot) = pilots.first() {
                    info!("Authenticated! Picking the first pilot because avoiding UI for now");
                    client
                        .request(CosmicVergeRequest::SelectPilot(pilot.id))
                        .await?;
                } else {
                    info!("Authenticated! But, you have no pilots. Create one in the browser at https://cosmicverge.com/ and come back");
                }
            }
            CosmicVergeResponse::Unauthenticated => {
                info!("Not authenticated, forcing you to try authenticating at twitch!");
                client
                    .request(CosmicVergeRequest::AuthenticationUrl(OAuthProvider::Twitch))
                    .await?;
            }
            CosmicVergeResponse::PilotChanged(pilot) => {
                let _ = self.event_emitter.send(ApiEvent::PilotChanged(pilot)).await;
            }
            CosmicVergeResponse::SpaceUpdate { .. } => {
                info!("TODO: SpaceUpdate ignored");
            }
            CosmicVergeResponse::PilotInformation(pilot) => {
                let mut cache = self.pilot_information_cache.write().await;
                cache.insert(pilot.id, pilot);
            }
            CosmicVergeResponse::Error { message } => {
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
