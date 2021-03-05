use std::collections::HashMap;

use cosmicverge_shared::solar_systems::{universe, SystemId};

pub type NodeId = u32;
pub type SolarSystemServerId = u32;

#[derive(Default, Debug, Clone)]
pub struct Cluster {
    pub nodes: HashMap<NodeId, Node>,
    pub servers: HashMap<SolarSystemServerId, SolarSystemServer>,
}

impl Cluster {
    pub fn fake_cluster() -> Self {
        let mut cluster = Self::default();

        let node_ids = std::iter::repeat_with(|| cluster.add_node())
            .take(3)
            .collect::<Vec<_>>();
        let mut node_ids = node_ids.into_iter().cycle();

        for system in universe().systems() {
            cluster.add_server_for_system(node_ids.next().unwrap(), system.id, true);
            cluster.add_server_for_system(node_ids.next().unwrap(), system.id, false);
            cluster.add_server_for_system(node_ids.next().unwrap(), system.id, false);
        }

        cluster
    }

    pub fn add_node(&mut self) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.insert(
            id,
            Node {
                id,
                load_1m: 0.1,
                load_5m: 0.2,
                load_15m: 0.1,
                ram_used: 768000,
                ram_free: 128000,
            },
        );
        id
    }

    pub fn add_server_for_system(
        &mut self,
        node_id: NodeId,
        system: SystemId,
        is_leader: bool,
    ) {
        let id = self.servers.len() as SolarSystemServerId + 100;
        self.servers.insert(
            id,
            SolarSystemServer {
                id,
                node_id,
                system,
                is_leader,
            },
        );
    }
}

#[derive(Debug, Clone)]
pub struct SolarSystemServer {
    pub id: SolarSystemServerId,
    pub node_id: NodeId,
    pub system: SystemId,
    pub is_leader: bool,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub load_1m: f32,
    pub load_5m: f32,
    pub load_15m: f32,
    pub ram_used: usize,
    pub ram_free: usize,
}
