use basws_client::prelude::InstallationConfig;
use once_cell::sync::OnceCell;
use persy::{ByteVec, ValueMode};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

use self::index::{Index, KeyValueIndex};

mod index;

static CLIENTDB: OnceCell<persy::Persy> = OnceCell::new();

fn client_db() -> &'static persy::Persy {
    CLIENTDB.get().unwrap()
}

#[derive(Clone)]
pub struct ClientDatabase {
    db: persy::Persy,
}

impl ClientDatabase {
    pub fn initialize<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
        CLIENTDB
            .set(persy::Persy::open_or_create_with(
                path,
                persy::Config::default(),
                |_persy| Ok(()), // This can be used to initialize the database
            )?)
            .map_err(|_| anyhow::anyhow!("ClientDatabase already initialized"))?;

        Ok(())
    }

    fn set_configuration_by_key<V: Serialize>(key: &str, value: &V) -> persy::PRes<()> {
        KeyValueIndex::named("configuration", ValueMode::REPLACE).set(key.to_string(), value)
    }

    fn configuration_by_key<D: DeserializeOwned>(key: &str) -> Option<D> {
        // TODO accessing by &key.to_string() because of the generic usage
        // within Persey. We should find proof that the rust compiler isn't
        // causing an allocation here, or submit a PR to solve this a better way
        // within Persey -- IndexType should be implemented for &str, but it's
        // invalid if its used outside of get methods, so potentially a second
        // trait type needs to be used for the get methods
        KeyValueIndex::named("configuration", ValueMode::REPLACE).get(&key.to_string())
    }

    pub fn installation_config() -> Option<InstallationConfig> {
        Self::configuration_by_key("installation_config")
    }

    pub fn set_installation_config(config: &InstallationConfig) -> persy::PRes<()> {
        Self::set_configuration_by_key("installation_config", config)
    }

    pub fn load_cached_resource(source_url: &str) -> Option<Vec<u8>> {
        Index::named("cached_resources", ValueMode::REPLACE)
            .get(&source_url.to_string())
            .map(|value: ByteVec| value.0.to_vec())
    }

    pub async fn store_cached_resource(source_url: &str, data: Vec<u8>) -> persy::PRes<()> {
        Index::named("cached_resources", ValueMode::REPLACE)
            .set(source_url.to_string(), ByteVec::from(data))
    }

    // pub fn last_tiles_timestamp() -> Option<DateTime<Utc>> {
    //     let last_tiles_timestamp = client_db().get(b"last_tiles_timestamp").ok().flatten()?;
    //     let last_tiles_timestamp = std::str::from_utf8(&last_tiles_timestamp).ok()?;
    //     DateTime::parse_from_rfc3339(last_tiles_timestamp)
    //         .ok()
    //         .map(DateTime::from)
    // }

    // pub fn set_last_tiles_timestamp(last_tiles_timestamp: DateTime<Utc>) -> sled::Result<()> {
    //     client_db()
    //         .insert(
    //             b"last_tiles_timestamp",
    //             last_tiles_timestamp.to_rfc3339().as_str(),
    //         )
    //         .map(|_| ())
    // }

    // pub async fn save_tiles(tiles: &[MapTile]) -> sled::Result<()> {
    //     let mut latest_timestamp = None;

    //     for tile in tiles {
    //         if latest_timestamp.is_none() || latest_timestamp.unwrap() < tile.last_changed {
    //             latest_timestamp = Some(tile.last_changed);
    //         }

    //         Self::save_tile(tile)?;
    //     }

    //     if let Some(timestamp) = latest_timestamp {
    //         Self::set_last_tiles_timestamp(timestamp)?;
    //     }

    //     client_db().flush_async().await?;

    //     Ok(())
    // }

    // fn save_tile(tile: &MapTile) -> sled::Result<()> {
    //     let tree = client_db().open_tree(b"map_tiles")?;
    //     tree.insert(
    //         MapTileKey {
    //             map: I32::new(tile.map_id),
    //             x: I32::new(tile.location.x),
    //             y: I32::new(tile.location.y),
    //         }
    //         .as_bytes(),
    //         serde_cbor::to_vec(tile).unwrap(),
    //     )?;
    //     Ok(())
    // }

    // pub fn load_tile(map: i32, x: i32, y: i32) -> Option<MapTile> {
    //     let tree = client_db().open_tree(b"map_tiles").ok()?;
    //     let ivec = tree
    //         .get(
    //             MapTileKey {
    //                 map: I32::new(map),
    //                 x: I32::new(x),
    //                 y: I32::new(y),
    //             }
    //             .as_bytes(),
    //         )
    //         .ok()
    //         .flatten()?;

    //     serde_cbor::from_slice(&ivec).ok()
    // }
}

// use byteorder::NetworkEndian;
// use zerocopy::{byteorder::I32, Unaligned};
// #[derive(AsBytes, Unaligned)]
// #[repr(C)]
// struct MapTileKey {
//     map: I32<NetworkEndian>,
//     x: I32<NetworkEndian>,
//     y: I32<NetworkEndian>,
// }
