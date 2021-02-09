use std::{collections::HashMap, sync::Arc};

use cosmicverge_shared::{
    protocol::{PilotId, PilotLocation, PilotPhysics, PilotingAction, ShipInformation},
    solar_systems::SolarSystemId,
    strum::EnumCount,
};
use once_cell::sync::OnceCell;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use serde::Deserialize;
use tokio::sync::RwLock;

pub struct LocationStore {
    redis: MultiplexedConnection,
    cache: Arc<RwLock<LocationCache>>,
}

static SHARED_STORE: OnceCell<LocationStore> = OnceCell::new();

impl LocationStore {
    pub async fn initialize(redis: MultiplexedConnection) {
        let store = LocationStore {
            redis,
            cache: Arc::new(RwLock::new(Default::default())),
        };

        store.reload_cache().await.unwrap();

        let _ = SHARED_STORE.set(store);
    }

    pub async fn refresh() -> Result<(), redis::RedisError> {
        let store = SHARED_STORE.get().unwrap();
        store.reload_cache().await
    }

    async fn reload_cache(&self) -> Result<(), redis::RedisError> {
        match self.fetch_cache_from_redis().await {
            Ok(new_cache) => {
                let mut cache = self.cache.write().await;
                *cache = new_cache;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    async fn fetch_cache_from_redis(&self) -> Result<LocationCache, redis::RedisError> {
        let mut redis = self.redis.clone();
        let (pilots, locations, actions, physics, ships) = redis::pipe()
            .cmd("HKEYS")
            .arg("connected_pilots")
            .cmd("HGETALL")
            .arg("pilot_locations")
            .cmd("HGETALL")
            .arg("pilot_actions")
            .cmd("HGETALL")
            .arg("pilot_physics")
            .cmd("HGETALL")
            .arg("pilot_ships")
            .query_async(&mut redis)
            .await?;
        let pilots: Vec<PilotId> = pilots;
        let locations: HashMap<PilotId, String> = locations;
        let actions: HashMap<PilotId, String> = actions;
        let physics: HashMap<PilotId, String> = physics;
        let ships: HashMap<PilotId, String> = ships;

        let mut pilot_cache = HashMap::new();
        for pilot_id in pilots {
            let location = parse_value_from_pilot_json_map(pilot_id, &locations);
            let action = parse_value_from_pilot_json_map(pilot_id, &actions);
            let physics = parse_value_from_pilot_json_map(pilot_id, &physics);
            let ship = parse_value_from_pilot_json_map(pilot_id, &ships);
            pilot_cache.insert(
                pilot_id,
                PilotCache {
                    location,
                    action,
                    physics,
                    ship,
                },
            );
        }

        let mut system_pilots: HashMap<SolarSystemId, Vec<PilotId>> =
            HashMap::with_capacity(SolarSystemId::COUNT);
        for (pilot_id, cache) in pilot_cache.iter() {
            system_pilots
                .entry(cache.location.system)
                .and_modify(|pilots| pilots.push(*pilot_id))
                .or_insert_with(|| vec![*pilot_id]);
        }

        Ok(LocationCache {
            system_pilots,
            pilot_cache,
        })
    }

    pub async fn lookup(pilot_id: PilotId) -> PilotCache {
        let store = SHARED_STORE.get().expect("Uninitialized cache access");
        let cache = store.cache.read().await;
        cache
            .pilot_cache
            .get(&pilot_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn set_piloting_action(
        pilot_id: PilotId,
        action: &PilotingAction,
    ) -> Result<(), redis::RedisError> {
        let store = SHARED_STORE.get().expect("Uninitialized cache access");
        let mut redis = store.redis.clone();
        redis
            .hset(
                "pilot_actions",
                pilot_id,
                serde_json::to_string(&action).unwrap(),
            )
            .await
    }

    pub async fn pilots_in_system(system: SolarSystemId) -> Vec<PilotId> {
        let store = SHARED_STORE.get().expect("Uninitialized cache access");
        let cache = store.cache.read().await;
        cache
            .system_pilots
            .get(&system)
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Default, Clone)]
pub struct PilotCache {
    pub location: PilotLocation,
    pub action: PilotingAction,
    pub ship: ShipInformation,
    pub physics: PilotPhysics,
}

#[derive(Default)]
struct LocationCache {
    system_pilots: HashMap<SolarSystemId, Vec<PilotId>>,
    pilot_cache: HashMap<PilotId, PilotCache>,
}

fn parse_value_from_pilot_json_map<'de, T: Deserialize<'de> + Default>(
    pilot_id: PilotId,
    json_map: &'de HashMap<PilotId, String>,
) -> T {
    match json_map.get(&pilot_id) {
        Some(json) => serde_json::from_str(json).unwrap_or_default(),
        None => T::default(),
    }
}
