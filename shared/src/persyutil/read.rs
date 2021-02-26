use persy::{IndexType, PRes, Persy, Transaction, Value};

pub enum PersyReadable<'a> {
    Persy(&'a Persy),
    Transaction(&'a mut Transaction),
}

impl<'a> Into<PersyReadable<'a>> for &'a Persy {
    fn into(self) -> PersyReadable<'a> {
        PersyReadable::Persy(self)
    }
}

impl<'a> Into<PersyReadable<'a>> for &'a mut Transaction {
    fn into(self) -> PersyReadable<'a> {
        PersyReadable::Transaction(self)
    }
}

impl<'a> PersyReadable<'a> {
    pub fn get<K, V>(&mut self, index_name: &str, k: &K) -> PRes<Option<Value<V>>>
    where
        K: IndexType,
        V: IndexType,
    {
        match self {
            PersyReadable::Persy(db) => db.get(index_name, k),
            PersyReadable::Transaction(db) => db.get(index_name, k),
        }
    }
}
