use persy::{ByteVec, IndexType, PRes, Persy, ValueMode};
use serde::{de::DeserializeOwned, Serialize};

use super::{read::PersyReadable, Index};

pub struct KeyValueIndex<'a, K> {
    index: Index<'a, K, ByteVec>,
}

impl<'a, K> KeyValueIndex<'a, K>
where
    K: IndexType,
{
    pub fn named(name: &'a str, value_mode: ValueMode) -> Self {
        Self {
            index: Index::named(name, value_mode),
        }
    }

    pub fn set<V: Serialize>(&self, key: K, value: &V, db: &Persy) -> PRes<()> {
        self.index
            .set(key, ByteVec::from(serde_cbor::to_vec(value).unwrap()), db)
    }

    pub fn get<D: DeserializeOwned, DB: Into<PersyReadable<'a>>>(
        &self,
        key: &K,
        db: DB,
    ) -> Option<D> {
        self.index
            .get(key, db)
            .map(|value| serde_cbor::from_slice(&value.0).ok())
            .flatten()
    }
}
