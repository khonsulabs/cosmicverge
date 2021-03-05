use persy::{ByteVec, IndexType, PRes, ValueMode};
use serde::{de::DeserializeOwned, Serialize};

use super::{Connection, Index};

pub struct KvIndex<'a, K> {
    index: Index<'a, K, ByteVec>,
}

impl<'a, K> KvIndex<'a, K>
where
    K: IndexType,
{
    #[must_use]
    pub fn named(name: &'a str, value_mode: ValueMode) -> Self {
        Self {
            index: Index::named(name, value_mode),
        }
    }

    pub fn set<'c, V: Serialize>(
        &self,
        key: K,
        value: &V,
        db: Connection<'c>,
    ) -> PRes<Connection<'c>> {
        self.index
            .set(key, ByteVec::from(serde_cbor::to_vec(value).unwrap()), db)
    }

    pub fn get<'c, D: DeserializeOwned>(&self, key: &K, db: &mut Connection<'c>) -> Option<D> {
        self.index
            .get(key, db)
            .and_then(|value| serde_cbor::from_slice(&value.0).ok())
    }
}
