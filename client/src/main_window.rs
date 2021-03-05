use basws_client::Url;
use cosmicverge_shared::protocol::navigation;
use kludgine::prelude::*;

use self::solar_system_canvas::SolarSystemCanvas;
use crate::{
    api::{self, ApiEvent},
    CosmicVergeClient,
};

mod solar_system_canvas;

pub fn run(server_url: &str) -> ! {
    SingleWindowApplication::run(CosmicVerge::new(server_url));
}

struct CosmicVerge {
    server_url: String,
    api_client: Option<CosmicVergeClient>,
    pilot: Option<navigation::ActivePilot>,
    solar_system: Entity<SolarSystemCanvas>,
    connected_pilots_count: Option<usize>,
}

impl WindowCreator for CosmicVerge {
    fn window_title() -> String {
        "Cosmic Verge".to_owned()
    }
}

impl Window for CosmicVerge {}

#[async_trait]
impl InteractiveComponent for CosmicVerge {
    type Message = ();

    type Command = Command;

    type Event = ();

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        let Command::HandleApiEvent(event) = command;
        match event {
            ApiEvent::ConnectedPilotsCountUpdated(count) => {
                self.connected_pilots_count = Some(count);
                // TODO set up a label for this
            }
            ApiEvent::PilotChanged(pilot) => {
                let _ = self
                    .solar_system
                    .send(solar_system_canvas::Command::ViewSolarSystem(
                        pilot.location.system,
                    ))
                    .await;

                self.pilot = Some(pilot);
                context.set_needs_redraw().await;
            }
            ApiEvent::SpaceUpdate {
                timestamp,
                location,
                action,
                ships,
            } => {
                let _ = self
                    .solar_system
                    .send(solar_system_canvas::Command::SpaceUpdate {
                        timestamp,
                        location,
                        action,
                        ships,
                    })
                    .await;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    HandleApiEvent(ApiEvent),
}

#[async_trait]
impl Component for CosmicVerge {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        self.api_client = Some(api::initialize(Url::parse(&self.server_url).unwrap()));
        self.spawn_api_event_receiver(context).await;

        self.solar_system = self
            .new_entity(
                context,
                SolarSystemCanvas::new(self.api_client.clone().unwrap()),
            )
            .await?
            .insert()
            .await?;

        self.register_fonts(context).await?;

        Ok(())
    }
}

impl CosmicVerge {
    pub fn new(server_url: impl ToString) -> Self {
        Self {
            server_url: server_url.to_string(),
            api_client: None,
            pilot: None,
            solar_system: Default::default(),
            connected_pilots_count: None,
        }
    }

    fn api_client(&self) -> &CosmicVergeClient {
        self.api_client.as_ref().unwrap()
    }

    async fn spawn_api_event_receiver(&self, context: &mut Context) {
        let event_receiver = self.api_client().event_receiver().await;
        let callback_entity = self.entity(context);

        Runtime::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                if callback_entity
                    .send(Command::HandleApiEvent(event))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
    }

    async fn register_fonts(&self, context: &mut Context) -> KludgineResult<()> {
        context
            .scene()
            .register_font(&include_font!("fonts/orbitron/Orbitron-Regular.ttf"))
            .await;
        context
            .scene()
            .register_font(&include_font!("fonts/orbitron/Orbitron-Bold.ttf"))
            .await;

        Ok(())
    }
}
