use std::time::Duration;

use data_model::Cluster;
use kludgine::prelude::*;

use self::cluster_map::ClusterMap;

mod cluster_map;
mod data_model;

pub fn run() -> ! {
    SingleWindowApplication::run(ClusterAdmin::fake_cluster());
}

#[derive(Default, Debug)]
struct ClusterAdmin {
    map: Entity<Scroll<ClusterMap>>,
    cluster: Handle<Cluster>,
}

#[derive(Debug, Clone)]
pub enum Command {
    ClusterUpdated,
}

impl Window for ClusterAdmin {}

impl WindowCreator for ClusterAdmin {
    fn window_title() -> String {
        "Cosmic Verge Cluster Admin".to_owned()
    }
}

#[async_trait]
impl Component for ClusterAdmin {
    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        Runtime::spawn(cluster_faker(self.entity(context), self.cluster.clone()));

        self.map = self
            .new_entity(context, Scroll::new(ClusterMap::new(self.cluster.clone())))
            .await?
            .bounds(Surround::uniform(Points::new(0.)).into())
            .insert()
            .await?;

        Ok(())
    }
}

#[async_trait]
impl InteractiveComponent for ClusterAdmin {
    type Message = ();
    type Command = Command;
    type Event = ();

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        let Command::ClusterUpdated = command;

        context.set_needs_redraw().await;

        Ok(())
    }
}

impl ClusterAdmin {
    fn fake_cluster() -> Self {
        Self {
            cluster: Handle::new(Cluster::fake_cluster()),
            map: Entity::default(),
        }
    }
}

async fn cluster_faker(
    admin: Entity<ClusterAdmin>,
    _cluster: Handle<Cluster>,
) -> anyhow::Result<()> {
    loop {
        // TODO do something to alter the cluster
        // cluster.rebalance();
        admin.send(Command::ClusterUpdated).await?;

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
