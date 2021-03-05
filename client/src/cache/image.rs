use std::{ops::Deref, sync::Arc};

use kludgine::prelude::*;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;

use super::{tracker::CacheTracker, CachedResource};

static CACHE_WORKERS_INITIALIZED: OnceCell<Handle<CacheTracker<CachedImage>>> = OnceCell::new();

pub struct CachedImage {
    resource: RwLock<CacheContents>,
}

enum CacheContents {
    Resource(Arc<CachedResource>),
    Loaded(Texture),
}

impl CachedImage {
    pub async fn new<S: ToString>(source_url: S) -> persy::PRes<Arc<Self>> {
        let source_url = source_url.to_string();
        {
            let tracker = Self::cache().read().await;
            if let Some(entry) = tracker.lookup(&source_url) {
                return Ok(entry);
            }
        }

        let mut tracker = Self::cache().write().await;
        match tracker.lookup(&source_url) {
            Some(entry) => Ok(entry),
            None => {
                let resource = CachedResource::new(&source_url).await?;

                Ok(tracker.track(source_url, move || CachedImage::from(resource)))
            }
        }
    }

    fn cache() -> &'static Handle<CacheTracker<CachedImage>> {
        CACHE_WORKERS_INITIALIZED.get_or_init(|| Handle::new(Default::default()))
    }

    pub async fn texture(&self) -> KludgineResult<Option<Texture>> {
        let loaded_data = {
            let resource = self.resource.read().await;
            match resource.deref() {
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

impl From<Arc<CachedResource>> for CachedImage {
    fn from(resource: Arc<CachedResource>) -> Self {
        Self {
            resource: RwLock::new(CacheContents::Resource(resource)),
        }
    }
}
