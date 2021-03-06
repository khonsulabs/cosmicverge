use std::marker::PhantomData;

use persy::{IndexType, PRes, Value, ValueMode};

use super::Connection;

pub struct Index<'a, K, V> {
    name: &'a str,
    value_mode: ValueMode,
    _phantom: PhantomData<(K, V)>,
}

impl<'a, K, V> Index<'a, K, V>
where
    K: IndexType,
    V: IndexType,
{
    #[must_use]
    pub fn named(name: &'a str, value_mode: ValueMode) -> Self {
        Self {
            name,
            value_mode,
            _phantom: PhantomData::default(),
        }
    }

    pub fn ensure_index_exists<'c>(&self, db: Connection<'c>) -> PRes<Connection<'c>> {
        if !db.exists_index(self.name)? {
            let mut tx = db.begin()?;
            // Check that the segment wasn't created by another caller before creating it
            if !tx.exists_index(self.name)? {
                // TODO ValueMode feels like it should be clone
                tx.create_index::<K, V>(self.name, self.value_mode.clone())?;
                tx = tx.commit()?;
            }

            return Ok(tx);
        }

        Ok(db)
    }

    pub fn get<'c>(&self, key: &K, db: &mut Connection<'c>) -> Option<V> {
        if let Ok(Some(Value::SINGLE(value))) = db.get::<K, V>(self.name, key) {
            Some(value)
        } else {
            None
        }
    }

    pub fn set<'c>(&self, key: K, value: V, mut db: Connection<'c>) -> PRes<Connection<'c>> {
        db = self.ensure_index_exists(db)?;

        let mut tx = db.begin()?;
        tx.put(self.name, key, value)?;
        tx.commit()
    }
}
