use basws_client::prelude::InstallationConfig;
use once_cell::sync::OnceCell;
use std::path::Path;
use zerocopy::AsBytes;

static CLIENTDB: OnceCell<sled::Db> = OnceCell::new();

fn client_db() -> &'static sled::Db {
    CLIENTDB.get().unwrap()
}

#[derive(Clone)]
pub struct ClientDatabase {
    db: sled::Db,
}

impl ClientDatabase {
    pub fn initialize<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
        CLIENTDB.set(sled::open(path)?).unwrap();

        Ok(())
    }

    pub fn installation_config() -> Option<InstallationConfig> {
        if let Ok(Some(config_bytes)) = client_db().get(b"installation_config") {
            serde_cbor::from_slice(config_bytes.as_bytes()).ok()
        } else {
            None
        }
    }

    pub fn set_installation_config(config: &InstallationConfig) -> sled::Result<()> {
        client_db()
            .insert(b"installation_config", serde_cbor::to_vec(&config).unwrap())
            .map(|_| ())
    }

    pub fn load_cached_resource(source_url: &str) -> sled::Result<Option<Vec<u8>>> {
        let tree = client_db().open_tree(b"cached_resources")?;
        let ivec = tree.get(source_url.as_bytes())?;

        Ok(ivec.map(|vec| vec.to_vec()))
    }

    pub async fn store_cached_resource(source_url: &str, data: &[u8]) -> sled::Result<()> {
        let db = client_db();
        let tree = db.open_tree(b"cached_resources")?;
        tree.insert(source_url.as_bytes(), data)?;
        db.flush_async().await?;
        Ok(())
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
