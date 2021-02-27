use basws_client::prelude::InstallationConfig;
use once_cell::sync::OnceCell;
use persy::{ByteVec, ValueMode};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

use cosmicverge_shared::persyutil::{Index, KeyValueIndex};

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
        KeyValueIndex::named("configuration", ValueMode::REPLACE)
            .set(key.to_string(), value, client_db().into())
            .map(|_| ())
    }

    fn configuration_by_key<D: DeserializeOwned>(key: &str) -> Option<D> {
        // TODO accessing by &key.to_string() because of the generic usage
        // within Persey. We should find proof that the rust compiler isn't
        // causing an allocation here, or submit a PR to solve this a better way
        // within Persey -- IndexType should be implemented for &str, but it's
        // invalid if its used outside of get methods, so potentially a second
        // trait type needs to be used for the get methods
        KeyValueIndex::named("configuration", ValueMode::REPLACE)
            .get(&key.to_string(), &mut client_db().into())
    }

    pub fn installation_config() -> Option<InstallationConfig> {
        Self::configuration_by_key("installation_config")
    }

    pub fn set_installation_config(config: &InstallationConfig) -> persy::PRes<()> {
        Self::set_configuration_by_key("installation_config", config)
    }

    pub fn load_cached_resource(source_url: &str) -> Option<Vec<u8>> {
        Index::named("cached_resources", ValueMode::REPLACE)
            .get(&source_url.to_string(), &mut client_db().into())
            .map(|value: ByteVec| value.0.to_vec())
    }

    pub async fn store_cached_resource(source_url: &str, data: Vec<u8>) -> persy::PRes<()> {
        Index::named("cached_resources", ValueMode::REPLACE)
            .set(
                source_url.to_string(),
                ByteVec::from(data),
                client_db().into(),
            )
            .map(|_| ())
    }
}
