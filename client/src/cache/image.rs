use std::sync::Arc;

use kludgine::prelude::*;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;

use super::{CachedResource, Tracker};

static CACHE_WORKERS_INITIALIZED: OnceCell<Handle<Tracker<Image>>> = OnceCell::new();

pub struct Image {
    resource: RwLock<CacheContents>,
}

enum CacheContents {
    Resource(Arc<CachedResource>),
    Loaded(Texture),
}

impl Image {
    pub async fn new<S: Into<String> + Send>(source_url: S) -> sled::Result<Arc<Self>> {
        let source_url = source_url.into();
        {
            let tracker = Self::cache().read().await;
            if let Some(entry) = tracker.lookup(&source_url) {
                return Ok(entry);
            }
        }

        let mut tracker = Self::cache().write().await;
        if let Some(entry) = tracker.lookup(&source_url) {
            Ok(entry)
        } else {
            let resource = CachedResource::new(&source_url).await?;

            Ok(tracker.track(source_url, move || Self::from(resource)))
        }
    }

    fn cache() -> &'static Handle<Tracker<Self>> {
        CACHE_WORKERS_INITIALIZED.get_or_init(|| Handle::new(Tracker::default()))
    }

    pub async fn texture(&self) -> KludgineResult<Option<Texture>> {
        let loaded_data = {
            let resource = self.resource.read().await;
            match &*resource {
                CacheContents::Loaded(texture) => return Ok(Some(texture.clone())),
                CacheContents::Resource(resource) => {
                    if let Some(data) = resource.data().await {
                        data.to_vec()
                    } else {
                        return Ok(None);
                    }
                }
            }
        };

        let mut resource = self.resource.write().await;
        let texture = Texture::from_bytes(&loaded_data)?;
        *resource = CacheContents::Loaded(texture.clone());
        Ok(Some(texture))
    }
}

impl From<Arc<CachedResource>> for Image {
    fn from(resource: Arc<CachedResource>) -> Self {
        Self {
            resource: RwLock::new(CacheContents::Resource(resource)),
        }
    }
}
