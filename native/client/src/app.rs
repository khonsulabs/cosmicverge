use kludgine::prelude::*;

use crate::CosmicVergeClient;

pub fn run(api_client: CosmicVergeClient) -> ! {
    SingleWindowApplication::run(CosmicVerge::default());
}

#[derive(Default)]
struct CosmicVerge {}

impl WindowCreator for CosmicVerge {
    fn window_title() -> String {
        "Cosmic Verge".to_owned()
    }
}

impl Window for CosmicVerge {}

impl StandaloneComponent for CosmicVerge {}

#[async_trait]
impl Component for CosmicVerge {
    async fn initialize(&mut self, _context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        Ok(())
    }

    async fn render(&mut self, context: &mut StyledContext, layout: &Layout) -> KludgineResult<()> {
        Ok(())
    }
}
