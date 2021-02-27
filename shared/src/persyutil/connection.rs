use persy::{IndexType, PRes, Persy, Transaction, Value, ValueMode};

#[allow(clippy::large_enum_variant)]
/// Abstracts an API above Persy that supports passing references into common
/// APIs regardless of whether you're inside of a transaction
///
/// For example, if you want to have a method that gets a value from Persy, but
/// you want it to support both reading from an &Persy reference as well as
/// inside of a Transaction, you currently must write two separate methods. This
/// Type offers a solution to that pattern, and also adds rudimentary support
/// for nested transactions (they become one large transaction).
pub enum PersyConnection<'a> {
    Persy(&'a Persy),
    Transaction {
        tx: Transaction,
        begin_count: usize,
        original_reference: &'a Persy,
    },
}

impl<'a> Into<PersyConnection<'a>> for &'a Persy {
    fn into(self) -> PersyConnection<'a> {
        PersyConnection::Persy(self)
    }
}

impl<'a> PersyConnection<'a> {
    pub fn exists_index(&self, index_name: &str) -> PRes<bool> {
        match self {
            PersyConnection::Persy(db) => db.exists_index(index_name),
            PersyConnection::Transaction { tx, .. } => tx.exists_index(index_name),
        }
    }

    pub fn get<K, V>(&mut self, index_name: &str, k: &K) -> PRes<Option<Value<V>>>
    where
        K: IndexType,
        V: IndexType,
    {
        match self {
            PersyConnection::Persy(db) => db.get(index_name, k),
            PersyConnection::Transaction { tx, .. } => tx.get(index_name, k),
        }
    }

    pub fn put<K, V>(&mut self, index_name: &str, k: K, v: V) -> PRes<()>
    where
        K: IndexType,
        V: IndexType,
    {
        match self {
            PersyConnection::Persy(_) => {
                panic!("put must always be called inside of a transaction")
            }
            PersyConnection::Transaction { tx, .. } => tx.put(index_name, k, v),
        }
    }

    pub fn create_index<K, V>(&mut self, index_name: &str, value_mode: ValueMode) -> PRes<()>
    where
        K: IndexType,
        V: IndexType,
    {
        match self {
            PersyConnection::Transaction { tx, .. } => {
                tx.create_index::<K, V>(index_name, value_mode)
            }
            PersyConnection::Persy(_) => {
                panic!("create_index must always be called inside of a transaction")
            }
        }
    }

    /// Begins a transaction
    ///
    /// If the connection is currently a Persy reference a new transaction is
    /// started. If the connection is already inside of a Transaction, the
    /// number of required calls to commit() to successfully commit the
    /// transaction is incremented.
    pub fn begin(self) -> PRes<PersyConnection<'a>> {
        match self {
            PersyConnection::Persy(db) => Ok(PersyConnection::Transaction {
                tx: db.begin()?,
                begin_count: 1,
                original_reference: db,
            }),
            PersyConnection::Transaction {
                tx,
                begin_count,
                original_reference,
            } => Ok(PersyConnection::Transaction {
                tx,
                begin_count: begin_count + 1,
                original_reference,
            }),
        }
    }

    /// Commits a transaction with nested transaction support
    ///
    /// commit() decrements the counter required to commit. Once an equal number
    /// of commit() and begin() calls has been reached, the transaction is
    /// committed and the connection reverts to a plain Persy connection.
    pub fn commit(self) -> PRes<PersyConnection<'a>> {
        match self {
            PersyConnection::Transaction {
                tx,
                begin_count,
                original_reference,
            } => {
                assert!(begin_count > 0);
                let begin_count = begin_count - 1;
                if begin_count == 0 {
                    let prepared = tx.prepare()?;
                    prepared.commit()?;
                    Ok(PersyConnection::Persy(original_reference))
                } else {
                    Ok(PersyConnection::Transaction {
                        tx,
                        begin_count,
                        original_reference,
                    })
                }
            }
            PersyConnection::Persy(_) => panic!("commit called before begin()"),
        }
    }

    /// Rollsback the current transaction
    ///
    /// Aborts the current transaction regardless of how many times begin() was
    /// called. The connection returned will be the original Persy reference.
    pub fn rollback(self) -> PRes<PersyConnection<'a>> {
        match self {
            PersyConnection::Transaction {
                tx,
                original_reference,
                ..
            } => {
                let prepared = tx.prepare()?;
                prepared.rollback()?;
                Ok(PersyConnection::Persy(original_reference))
            }
            PersyConnection::Persy(_) => panic!("rollback called outside of a transaction"),
        }
    }
}
