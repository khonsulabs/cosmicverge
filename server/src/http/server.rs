use async_trait::async_trait;
use cosmicverge_shared::protocol::{self, navigation::ActivePilot};
use database::{
    basws_server::{self, prelude::*},
    cosmicverge_shared::protocol::{
        cosmic_verge_protocol_version_requirements, OAuthProvider, Permissions, Request, Response,
    },
    schema::{convert_db_pilots, pilot, Account, Installation, Pilot},
};

use crate::{
    http::twitch,
    orchestrator::{connected_pilots, location_store::LocationStore},
    pubsub::connected_pilots_count,
};

#[derive(Debug)]
pub struct ConnectedAccount {
    pub account: Account,
    pub permissions: Permissions,
}

impl ConnectedAccount {
    pub async fn lookup(installation_id: Uuid) -> anyhow::Result<Self> {
        let account = Account::find_by_installation_id(installation_id, database::pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("no profile found"))?;
        let permissions = account.permissions(database::pool()).await?;

        Ok(Self {
            account,
            permissions,
        })
    }
}

impl Identifiable for ConnectedAccount {
    type Id = i64;
    fn id(&self) -> Self::Id {
        self.account.id
    }
}

pub struct Server;

#[derive(Default, Debug)]
pub struct ClientData {
    pub pilot: Option<Pilot>,
}

pub fn initialize() -> basws_server::Server<Server> {
    basws_server::Server::new(Server)
}

#[async_trait]
impl ServerLogic for Server {
    type Request = Request;
    type Response = Response;
    type Client = ClientData;
    type Account = ConnectedAccount;
    type AccountId = i64;

    async fn handle_request(
        &self,
        client: &ConnectedClient<Self>,
        request: Self::Request,
        _server: &basws_server::Server<Self>,
    ) -> anyhow::Result<RequestHandling<Self::Response>> {
        match request {
            Request::Fly(action) => {
                if let Some(pilot_id) = client.map_client(|c| c.pilot.as_ref().map(Pilot::id)).await
                {
                    LocationStore::set_piloting_action(pilot_id, &action).await?;
                    Ok(RequestHandling::NoResponse)
                } else {
                    anyhow::bail!("attempted to fly without having a pilot selected")
                }
            }
            Request::AuthenticationUrl(provider) => match provider {
                OAuthProvider::Twitch => {
                    if let Some(installation) = client.installation().await {
                        Ok(RequestHandling::Respond(Response::AuthenticateAtUrl {
                            url: twitch::authorization_url(installation.id),
                        }))
                    } else {
                        anyhow::bail!("Requested authentication URL without being connected")
                    }
                }
            },
            Request::SelectPilot(pilot_id) => {
                if let Some(pilot) = Pilot::load(pilot_id, database::pool()).await? {
                    self.select_pilot(pilot, client).await
                } else {
                    Ok(RequestHandling::Respond(Response::error("not-found")))
                }
            }
            Request::CreatePilot { name } => {
                if let Some(connected_account) = client.account().await {
                    let connected_account = connected_account.read().await;
                    match Pilot::create(connected_account.account.id, &name, database::pool()).await
                    {
                        Ok(pilot) => self.select_pilot(pilot, client).await,
                        Err(pilot::Error::NameAlreadyTaken) => Ok(RequestHandling::Respond(
                            Response::error("pilot-error-name-already-taken"),
                        )),
                        Err(pilot::Error::InvalidName) => Ok(RequestHandling::Respond(
                            Response::error("pilot-error-invalid-name"),
                        )),
                        Err(pilot::Error::TooManyPilots) => Ok(RequestHandling::Respond(
                            Response::error("pilot-error-too-many-pilots"),
                        )),
                        Err(pilot::Error::Database(db)) => Err(db.into()),
                    }
                } else {
                    Ok(RequestHandling::Respond(Response::error("unauthenticated")))
                }
            }
            // TODO this should use a cache
            Request::GetPilotInformation(pilot_id) => {
                match Pilot::load(pilot_id, database::pool()).await? {
                    Some(pilot) => Ok(RequestHandling::Respond(Response::PilotInformation(
                        pilot.into(),
                    ))),
                    None => Ok(RequestHandling::Respond(Response::error("pilot not found"))),
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

            let account = protocol::Account {
                id: connected_account.account.id,
                permissions: connected_account
                    .account
                    .permissions(database::pool())
                    .await?,
            };

            Ok(RequestHandling::Respond(Response::Authenticated {
                account,
                pilots,
            }))
        } else {
            Ok(RequestHandling::Respond(Response::Unauthenticated))
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
            Response::Unauthenticated,
            Response::ServerStatus {
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
            .map_client(|client| client.pilot.as_ref().map(Pilot::id))
            .await
        {
            connected_pilots::note(pilot_id).await;
        }

        Ok(RequestHandling::NoResponse)
    }
}

impl Server {
    async fn select_pilot(
        &self,
        pilot: Pilot,
        client: &ConnectedClient<Self>,
    ) -> anyhow::Result<RequestHandling<Response>> {
        let api_pilot = pilot.clone().into();
        let info = LocationStore::lookup(pilot.id()).await;
        client
            .map_client_mut(|client| {
                client.pilot = Some(pilot);
            })
            .await;
        Ok(RequestHandling::Respond(Response::PilotChanged(
            ActivePilot {
                pilot: api_pilot,
                location: info.location,
                action: info.action,
            },
        )))
    }
}
