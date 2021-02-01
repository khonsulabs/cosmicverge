use async_trait::async_trait;
use database::{
    basws_server::prelude::*,
    cosmicverge_shared::{
        cosmic_verge_protocol_version_requirements, CosmicVergeRequest, CosmicVergeResponse,
        OAuthProvider,
    },
    schema::{convert_db_pilots, Account, Installation, Pilot, PilotError},
};

use super::twitch;
use crate::pubsub::connected_pilots_count;

#[derive(Debug)]
pub struct ConnectedAccount {
    pub account: Account,
}

impl ConnectedAccount {
    pub async fn lookup(installation_id: Uuid) -> anyhow::Result<Self> {
        let account = Account::find_by_installation_id(installation_id, database::pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("no profile found"))?;
        Ok(Self { account })
    }
}

impl Identifiable for ConnectedAccount {
    type Id = i64;
    fn id(&self) -> Self::Id {
        self.account.id
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
    type Client = Option<Pilot>;
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
                        Ok(RequestHandling::Respond(
                            CosmicVergeResponse::AuthenticateAtUrl {
                                url: twitch::authorization_url(installation.id),
                            },
                        ))
                    } else {
                        anyhow::bail!("Requested authentication URL without being connected")
                    }
                }
            },
            CosmicVergeRequest::SelectPilot(pilot_id) => {
                if let Some(pilot) = Pilot::load(pilot_id, database::pool()).await? {
                    self.select_pilot(pilot, client).await
                } else {
                    Ok(RequestHandling::Respond(CosmicVergeResponse::error(
                        "not-found",
                    )))
                }
            }
            CosmicVergeRequest::CreatePilot { name } => {
                if let Some(connected_account) = client.account().await {
                    let connected_account = connected_account.read().await;
                    match Pilot::create(connected_account.account.id, &name, database::pool()).await
                    {
                        Ok(pilot) => self.select_pilot(pilot, client).await,
                        Err(PilotError::NameAlreadyTaken) => Ok(RequestHandling::Respond(
                            CosmicVergeResponse::error("pilot-error-name-already-taken"),
                        )),
                        Err(PilotError::InvalidName) => Ok(RequestHandling::Respond(
                            CosmicVergeResponse::error("pilot-error-invalid-name"),
                        )),
                        Err(PilotError::TooManyPilots) => Ok(RequestHandling::Respond(
                            CosmicVergeResponse::error("pilot-error-too-many-pilots"),
                        )),
                        Err(PilotError::Database(db)) => Err(db.into()),
                    }
                } else {
                    Ok(RequestHandling::Respond(CosmicVergeResponse::error(
                        "unauthenticated",
                    )))
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
        let installation = Installation::load_or_create(installation_id).await?;
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
            let connected_account = account.read().await;
            let pilots = convert_db_pilots(
                Pilot::list_by_account_id(connected_account.account.id, database::pool()).await?,
            );

            Ok(RequestHandling::Respond(
                CosmicVergeResponse::Authenticated {
                    user_id: connected_account.account.id,
                    pilots,
                },
            ))
        } else {
            Ok(RequestHandling::Respond(
                CosmicVergeResponse::Unauthenticated,
            ))
        }
    }

    async fn client_disconnected(&self, _client: &ConnectedClient<Self>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn new_client_connected(
        &self,
        _client: &ConnectedClient<Self>,
    ) -> anyhow::Result<RequestHandling<Self::Response>> {
        Ok(RequestHandling::Batch(vec![
            CosmicVergeResponse::Unauthenticated,
            CosmicVergeResponse::ServerStatus {
                connected_pilots: connected_pilots_count(),
            },
        ]))
    }

    async fn account_associated(&self, client: &ConnectedClient<Self>) -> anyhow::Result<()> {
        if let Some(installation) = client.installation().await {
            if let Some(account) = client.account().await {
                let account_id = {
                    let account = account.read().await;
                    account.id()
                };
                Installation::set_account_id_for_installation_id(
                    installation.id,
                    Some(account_id),
                    database::pool(),
                )
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

    async fn client_timings_updated(
        &self,
        client: &ConnectedClient<Self>,
    ) -> anyhow::Result<RequestHandling<Self::Response>> {
        if let Some(pilot_id) = client
            .map_client(|client| client.as_ref().map(|p| p.id))
            .await
        {
            orchestrator::connected_pilots::note(pilot_id).await;
        }

        Ok(RequestHandling::NoResponse)
    }
}

impl CosmicVergeServer {
    async fn select_pilot(
        &self,
        pilot: Pilot,
        client: &ConnectedClient<Self>,
    ) -> anyhow::Result<RequestHandling<CosmicVergeResponse>> {
        let api_pilot = pilot.clone().into();
        client
            .map_client_mut(|client| {
                *client = Some(pilot);
            })
            .await;
        Ok(RequestHandling::Respond(CosmicVergeResponse::PilotChanged(
            api_pilot,
        )))
    }
}
