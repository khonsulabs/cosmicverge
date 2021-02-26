use persy::{ByteVec, Value, ValueMode};
use serde::{de::DeserializeOwned, Serialize};

use super::client_db;

pub struct Index<'a, K, V> {
    name: &'a str,
    value_mode: persy::ValueMode,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<'a, K, V> Index<'a, K, V>
where
    K: persy::IndexType,
    V: persy::IndexType,
{
    pub fn named(name: &'a str, value_mode: ValueMode) -> Self {
        Self {
            name,
            value_mode,
            _phantom: Default::default(),
        }
    }

    pub fn ensure_index_exists(&self) -> persy::PRes<()> {
        if !client_db().exists_index(self.name)? {
            let mut tx = client_db().begin()?;
            // Check that the segment wasn't created by another caller before creating it
            if !tx.exists_index(self.name)? {
                // TODO ValueMode feels like it should be clone
                tx.create_index::<K, V>(self.name, self.value_mode.clone())?;
                let prepared = tx.prepare()?;
                return prepared.commit();
            }
        }

        Ok(())
    }

    pub fn set(&self, key: K, value: V) -> persy::PRes<()> {
        self.ensure_index_exists()?;

        let mut tx = client_db().begin()?;
        tx.put(self.name, key, value)?;
        let prepared = tx.prepare()?;
        prepared.commit()
    }

    pub fn get(&self, key: &K) -> Option<V> {
        if let Ok(Some(Value::SINGLE(value))) = client_db().get::<K, V>(self.name, &key) {
            Some(value)
        } else {
            None
        }
    }
}

pub struct KeyValueIndex<'a, K> {
    index: Index<'a, K, ByteVec>,
}

impl<'a, K> KeyValueIndex<'a, K>
where
    K: persy::IndexType,
{
    pub fn named(name: &'a str, value_mode: ValueMode) -> Self {
        Self {
            index: Index::named(name, value_mode),
        }
    }

    pub fn set<V: Serialize>(&self, key: K, value: &V) -> persy::PRes<()> {
        self.index
            .set(key, ByteVec::from(serde_cbor::to_vec(value).unwrap()))
    }

    pub fn get<D: DeserializeOwned>(&self, key: &K) -> Option<D> {
        self.index
            .get(key)
            .map(|value| serde_cbor::from_slice(&value.0).ok())
            .flatten()
    }
}
