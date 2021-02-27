use std::sync::Arc;

use basws_client::Handle;
use kludgine::runtime::Runtime;

use crate::database::ClientDatabase;
use async_channel::{Receiver, Sender};
use once_cell::sync::OnceCell;

mod image;
mod tracker;

pub use image::CachedImage;

static CACHE_WORKERS_INITIALIZED: OnceCell<CacheData> = OnceCell::new();

struct CacheData {
    tracker: Handle<tracker::CacheTracker<CachedResource>>,
    sender: Sender<Arc<CachedResource>>,
}

#[derive(Debug)]
pub struct CachedResource {
    pub source_url: String,
    pub data: Handle<Option<Vec<u8>>>,
}

impl CachedResource {
    // TODO this isn't the best design for minimizing over-caching. It is fine for now, but if two areas of code
    // create cached resources independently pointing to the same location, it's possible for two copies of the data to end up in memory.
    // Whatever solution we do here should also be used for the other cache types
    pub async fn new<S: ToString>(source_url: S) -> persy::PRes<Arc<Self>> {
        let cache = Self::cache();
        let source_url = source_url.to_string();
        {
            let tracker = cache.tracker.read().await;
            if let Some(existing_entry) = tracker.lookup(&source_url) {
                return Ok(existing_entry);
            }
        }

        let mut tracker = cache.tracker.write().await;
        let mut queue = false;
        let entry = tracker.track(source_url.clone(), || {
            let data = ClientDatabase::load_cached_resource(&source_url);
            queue = data.is_none();
            let data = Handle::new(data);
            Self { source_url, data }
        });

        if queue {
            let _ = cache.sender.send(entry.clone()).await;
        }

        Ok(entry)
    }

    fn cache() -> &'static CacheData {
        CACHE_WORKERS_INITIALIZED.get_or_init(|| {
            let (sender, receiver) = async_channel::unbounded();
            Runtime::spawn(cache_loop(receiver));
            CacheData {
                sender,
                tracker: Handle::new(Default::default()),
            }
        })
    }

    pub async fn data(&self) -> Option<Vec<u8>> {
        let data = self.data.read().await;
        data.clone()
    }
}

async fn cache_loop(receiver: Receiver<Arc<CachedResource>>) {
    let client = reqwest::Client::new();
    while let Ok(resource) = receiver.recv().await {
        if let Err(err) = load_resource(&client, resource).await {
            error!("Error writing to cache: {:?}", err);
        }
    }
}

async fn load_resource(
    client: &reqwest::Client,
    resource: Arc<CachedResource>,
) -> anyhow::Result<()> {
    let source_url = if resource.source_url.starts_with('/') {
        "http://localhost:7879".to_string() + resource.source_url.as_str()
    } else {
        resource.source_url.to_string()
    };
    let response = client.get(&source_url).send().await?;
    let data = response.bytes().await?;
    ClientDatabase::store_cached_resource(&source_url, data.to_vec()).await?;

    let mut cache_data = resource.data.write().await;
    *cache_data = Some(data.to_vec());

    Ok(())
}
