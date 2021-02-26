use persy::{IndexType, PRes, Persy, Value, ValueMode};

use super::read::PersyReadable;

pub struct Index<'a, K, V> {
    name: &'a str,
    value_mode: ValueMode,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<'a, K, V> Index<'a, K, V>
where
    K: IndexType,
    V: IndexType,
{
    pub fn named(name: &'a str, value_mode: ValueMode) -> Self {
        Self {
            name,
            value_mode,
            _phantom: Default::default(),
        }
    }

    pub fn ensure_index_exists(&'a self, db: &Persy) -> PRes<()> {
        if !db.exists_index(self.name)? {
            let mut tx = db.begin()?;
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

    pub fn set(&self, key: K, value: V, db: &Persy) -> PRes<()> {
        self.ensure_index_exists(db)?;

        let mut tx = db.begin()?;
        tx.put(self.name, key, value)?;
        let prepared = tx.prepare()?;
        prepared.commit()
    }

    pub fn get<DB: Into<PersyReadable<'a>>>(&self, key: &K, db: DB) -> Option<V> {
        let mut db = db.into();
        if let Ok(Some(Value::SINGLE(value))) = db.get::<K, V>(self.name, &key) {
            Some(value)
        } else {
            None
        }
    }
}
