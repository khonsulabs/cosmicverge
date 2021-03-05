use std::collections::{HashMap, HashSet};

use async_channel::Receiver;
use basws_client::prelude::*;
use cosmicverge_shared::protocol::{
    cosmic_verge_protocol_version, navigation, pilot, OAuthProvider, Pilot, Request, Response,
};
use kludgine::runtime::Runtime;

use crate::{database::Database, CosmicVergeClient};

mod broadcast;

pub fn initialize(server_url: Url) -> CosmicVergeClient {
    let client = basws_client::Client::new(Client {
        server_url,
        pilot_information_cache: Handle::default(),
        event_emitter: broadcast::Channel::default(),
    });

    let thread_client = client.clone();
    Runtime::spawn(thread_client.run());

    client
}

#[derive(Debug)]
pub struct Client {
    pub server_url: Url,
    pub pilot_information_cache: Handle<PilotInformationCache>,
    event_emitter: broadcast::Channel<Event>,
}

#[derive(Debug, Default)]
pub struct PilotInformationCache {
    info: HashMap<pilot::Id, Pilot>,
    requested: HashSet<pilot::Id>,
}

#[derive(Debug, Clone)]
pub enum Event {
    ConnectedPilotsCountUpdated(usize),
    PilotChanged(navigation::ActivePilot),
    SpaceUpdate {
        timestamp: f64,
        location: navigation::Pilot,
        action: navigation::Action,
        ships: Vec<navigation::Ship>,
    },
}

impl Client {
    pub async fn event_receiver(&self) -> Receiver<Event> {
        self.event_emitter.receiver().await
    }
}

#[async_trait]
impl ClientLogic for Client {
    type Request = Request;
    type Response = Response;

    fn server_url(&self) -> Url {
        self.server_url.clone()
    }

    fn protocol_version(&self) -> Version {
        cosmic_verge_protocol_version()
    }

    async fn state_changed(
        &self,
        state: &LoginState,
        _client: basws_client::Client<Self>,
    ) -> anyhow::Result<()> {
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
        Database::installation_config()
    }

    async fn store_installation_config(&self, config: InstallationConfig) -> anyhow::Result<()> {
        Database::set_installation_config(&config)?;

        Ok(())
    }

    async fn response_received(
        &self,
        response: Self::Response,
        _original_request_id: Option<u64>,
        client: basws_client::Client<Self>,
    ) -> anyhow::Result<()> {
        match response {
            Response::ServerStatus { connected_pilots } => {
                drop(
                    self.event_emitter
                        .send(Event::ConnectedPilotsCountUpdated(connected_pilots))
                        .await,
                );
            }
            Response::AuthenticateAtUrl { url } => {
                if webbrowser::open(&url).is_err() {
                    error!("Could not open a browser for you. Please open this URL to proceed with authentication: {}", url);
                }
            }
            Response::Authenticated { pilots, .. } => {
                if let Some(pilot) = pilots.first() {
                    info!("Authenticated! Picking the first pilot because avoiding UI for now");
                    client.request(Request::SelectPilot(pilot.id)).await?;
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
                drop(self.event_emitter.send(Event::PilotChanged(pilot)).await);
            }
            Response::SpaceUpdate {
                timestamp,
                location,
                action,
                ships,
            } => {
                drop(
                    self.event_emitter
                        .send(Event::SpaceUpdate {
                            timestamp,
                            location,
                            action,
                            ships,
                        })
                        .await,
                );
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

    async fn handle_error(
        &self,
        error: Error,
        _client: basws_client::Client<Self>,
    ) -> anyhow::Result<()> {
        error!("Api Error: {:?}", error);

        Ok(())
    }
}

impl Client {
    pub async fn pilot_information(
        &self,
        pilot_id: pilot::Id,
        client: &basws_client::Client<Self>,
    ) -> Option<Pilot> {
        {
            let cache = self.pilot_information_cache.read().await;
            if let Some(info) = cache.info.get(&pilot_id) {
                return Some(info.clone());
            } else if cache.requested.contains(&pilot_id) {
                // Already requested, don't spam the server with more requests
                return None;
            }
        }

        let mut cache = self.pilot_information_cache.write().await;
        if !cache.requested.contains(&pilot_id) {
            cache.requested.insert(pilot_id);
            drop(client.request(Request::GetPilotInformation(pilot_id)).await);
        }
        None
    }
}
