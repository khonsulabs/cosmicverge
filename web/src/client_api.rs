use basws_yew::{prelude::*, ClientLogic, ClientState, Error};
use cosmicverge_shared::protocol::{
    cosmic_verge_protocol_version, CosmicVergeRequest, CosmicVergeResponse,
};
use url::Url;

pub type AgentMessage = basws_yew::AgentMessage<CosmicVergeRequest>;
pub type AgentResponse = basws_yew::AgentResponse<CosmicVergeResponse>;
pub type ApiAgent = basws_yew::ApiAgent<CosmicVergeApiClient>;
pub type ApiBridge = basws_yew::ApiBridge<CosmicVergeApiClient>;

#[derive(Debug, Default)]
pub struct CosmicVergeApiClient;

impl ClientLogic for CosmicVergeApiClient {
    type Request = CosmicVergeRequest;
    type Response = CosmicVergeResponse;

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
            CosmicVergeResponse::AuthenticateAtUrl { url } => {
                let window = web_sys::window().expect("Need a window");
                window
                    .location()
                    .set_href(&url)
                    .expect("Error setting location for redirect");
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
