use basws_client::Url;
use cosmicverge_shared::protocol::ActivePilot;
use kludgine::prelude::*;

use crate::{
    api::{self, ApiEvent},
    CosmicVergeClient,
};

use self::solar_system_canvas::SolarSystemCanvas;

mod solar_system_canvas;

pub fn run() -> ! {
    SingleWindowApplication::run(CosmicVerge::default());
}

#[derive(Default)]
struct CosmicVerge {
    api_client: Option<CosmicVergeClient>,
    pilot: Option<ActivePilot>,
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
            ApiEvent::ConnectedPilotsCountUpdated(count) => {}
            ApiEvent::PilotChanged(pilot) => {
                info!("Received command PilotChanged {:?}", pilot);
                let _ = self
                    .solar_system
                    .send(solar_system_canvas::Command::ViewSolarSystem(
                        pilot.location.system,
                    ))
                    .await;

                self.pilot = Some(pilot);
                context.set_needs_redraw().await;
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
        info!("initializing");
        self.api_client = Some(api::initialize(
            Url::parse("ws://localhost:7879/v1/ws").unwrap(),
        ));
        self.spawn_api_event_receiver(context).await;

        self.solar_system = self
            .new_entity(context, SolarSystemCanvas::default())
            .await?
            .insert()
            .await?;

        Ok(())
    }

    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        if let Some(active_pilot) = &self.pilot {
            Text::span(&active_pilot.pilot.name, context.effective_style()?.clone())
                .render_at(context.scene(), Point::new(0.0, 120.0), TextWrap::NoWrap)
                .await?;
        }

        Ok(())
    }
}

impl CosmicVerge {
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
}
