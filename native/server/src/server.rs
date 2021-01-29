use super::{database_refactor, twitch};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use cosmicverge_shared::{
    cosmic_verge_protocol_version_requirements,
    AuthenticatedUser, CosmicVergeRequest, CosmicVergeResponse, OAuthProvider,
};
use basws_server::prelude::*;


#[derive(Debug)]
pub struct ConnectedAccount {
    pub user: AuthenticatedUser,
}

impl ConnectedAccount {
    pub async fn lookup(installation_id: Uuid) -> anyhow::Result<Self> {
        let profile = database_refactor::get_profile_by_installation_id(database::pool(), installation_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("no profile found"))?;
        Ok(Self {
            user: AuthenticatedUser {
                profile,
            },
        })
    }
}

impl Identifiable for ConnectedAccount {
    type Id = i64;
    fn id(&self) -> Self::Id {
        self.user.profile.id
    }
}

pub struct CosmicVergeServer;

pub fn initialize() -> Server<CosmicVergeServer> {
    Server::new(CosmicVergeServer)
}

#[async_trait]
impl ServerLogic for CosmicVergeServer {
    type Request = CosmicVergeRequest;
    type Response = CosmicVergeResponse;
    type Client = ();
    type Account = ConnectedAccount;
    type AccountId = i64;

    async fn handle_request(
        &self,
        client: &ConnectedClient<Self>,
        request: Self::Request,
        _server: &Server<Self>,
    ) -> anyhow::Result<RequestHandling<Self::Response>> {
        match request {
            CosmicVergeRequest::AuthenticationUrl(provider) => match provider {
                OAuthProvider::Twitch => {
                    if let Some(installation) = client.installation().await {
                        Ok(RequestHandling::Respond(CosmicVergeResponse::AuthenticateAtUrl {
                            url: twitch::authorization_url(installation.id),
                        }))
                    } else {
                        anyhow::bail!("Requested authentication URL without being connected")
                    }
                }
            }
        }
    }

    async fn lookup_account_from_installation_id(
        &self,
        installation_id: Uuid,
    ) -> anyhow::Result<Option<Handle<Self::Account>>> {
        Ok(ConnectedAccount::lookup(installation_id)
            .await
            .ok()
            .map(Handle::new))
    }

    fn protocol_version_requirements(&self) -> VersionReq {
        cosmic_verge_protocol_version_requirements()
    }

    async fn lookup_or_create_installation(
        &self,
        _client: &ConnectedClient<Self>,
        installation_id: Option<Uuid>,
    ) -> anyhow::Result<InstallationConfig> {
        let installation = database_refactor::lookup_or_create_installation(installation_id).await?;
        Ok(InstallationConfig::from_vec(
            installation.id,
            installation.private_key.unwrap(),
        )?)
    }

    async fn client_reconnected(
        &self,
        client: &ConnectedClient<Self>,
    ) -> anyhow::Result<RequestHandling<Self::Response>> {
        if let Some(account) = client.account().await {
            let account = account.read().await;

            Ok(RequestHandling::Respond(CosmicVergeResponse::Authenticated(
                account.user.clone(),
            )))
        } else {
            Ok(RequestHandling::Respond(CosmicVergeResponse::Unauthenticated))
        }
    }

    async fn client_disconnected(&self, _client: &ConnectedClient<Self>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn new_client_connected(
        &self,
        _client: &ConnectedClient<Self>,
    ) -> anyhow::Result<RequestHandling<Self::Response>> {
        Ok(RequestHandling::Respond(CosmicVergeResponse::Unauthenticated))
    }

    async fn account_associated(&self, client: &ConnectedClient<Self>) -> anyhow::Result<()> {
        if let Some(installation) = client.installation().await {
            if let Some(account) = client.account().await {
                let account_id = {
                    let account = account.read().await;
                    account.id()
                };
                database_refactor::set_installation_account_id(database::pool(), installation.id, Some(account_id))
                    .await?;
                return Ok(());
            }
        }
        anyhow::bail!("account_associated called with either no installation or account")
    }

    async fn handle_websocket_error(&self, _err: warp::Error) -> ErrorHandling {
        println!("Error: {:?}", _err);
        ErrorHandling::Disconnect
    }
}
