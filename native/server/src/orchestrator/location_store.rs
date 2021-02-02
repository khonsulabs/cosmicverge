use std::{collections::HashMap, sync::Arc};

use cosmicverge_shared::{
    protocol::{PilotLocation, PilotingAction},
    solar_systems::SolarSystemId,
    strum::EnumCount,
};
use once_cell::sync::OnceCell;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use tokio::sync::RwLock;

use crate::orchestrator::connected_pilots::PilotId;

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
        let (locations, actions) = redis::pipe()
            .cmd("HGETALL")
            .arg("pilot_locations")
            .cmd("HGETALL")
            .arg("pilot_actions")
            .query_async(&mut redis)
            .await?;
        let locations: HashMap<PilotId, String> = locations;
        let actions: HashMap<PilotId, String> = actions;

        let mut pilot_cache = HashMap::new();
        for (pilot_id, location) in locations {
            let location = serde_json::from_str::<PilotLocation>(&location).unwrap_or_default();
            let action = match actions.get(&pilot_id) {
                Some(action) => serde_json::from_str(action).unwrap_or_default(),
                None => PilotingAction::default(),
            };

            pilot_cache.insert(pilot_id, PilotCache { location, action });
        }

        // It's possible for pilots to not have an initialized position, but have an action set
        for (pilot_id, action) in actions {
            if pilot_cache.contains_key(&pilot_id) {
                continue;
            }

            let action = serde_json::from_str(&action).unwrap_or_default();

            pilot_cache.insert(
                pilot_id,
                PilotCache {
                    action,
                    location: Default::default(),
                },
            );
        }

        let mut system_pilots: HashMap<SolarSystemId, Vec<i64>> =
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
}

#[derive(Default)]
struct LocationCache {
    system_pilots: HashMap<SolarSystemId, Vec<PilotId>>,
    pilot_cache: HashMap<PilotId, PilotCache>,
}
