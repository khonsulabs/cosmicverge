use cosmicverge_shared::protocol::ActivePilot;
use kludgine::prelude::*;

use crate::{api::ApiEvent, CosmicVergeClient};

pub fn run(api_client: CosmicVergeClient) -> ! {
    SingleWindowApplication::run(CosmicVerge {
        api_client,
        pilot: None,
    });
}

struct CosmicVerge {
    api_client: CosmicVergeClient,
    pilot: Option<ActivePilot>,
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
        let Command::PilotSelected(pilot) = command;
        self.pilot = Some(pilot);
        context.set_needs_redraw().await;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    PilotSelected(ActivePilot),
}

#[async_trait]
impl Component for CosmicVerge {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        let event_receiver = self.api_client.event_receiver().await;
        let callback_entity = self.entity(context);

        Runtime::spawn(async move {
            while let Ok(event) = event_receiver.recv().await {
                let ApiEvent::PilotChanged(pilot) = event;
                if callback_entity
                    .send(Command::PilotSelected(pilot))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

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
