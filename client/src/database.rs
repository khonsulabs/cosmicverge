use basws_client::prelude::InstallationConfig;
use once_cell::sync::OnceCell;
use std::path::Path;
use zerocopy::AsBytes;

static CLIENTDB: OnceCell<sled::Db> = OnceCell::new();

fn client_db() -> &'static sled::Db {
    CLIENTDB.get().unwrap()
}

#[derive(Clone)]
pub struct Database {
    db: sled::Db,
}

impl Database {
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
}
