use persy::{PRes, PersyId};

use super::{Index, PersyConnection};

/// This is a hypothetical structure that's an idea of how to solve a Table-like structure with multiple indexes that need to be kept in sync.
///
/// It would expose methods to help with insert/update/delete that would take care of:
///   - serializing the structure
///   - beginning a transaction
///   - inserting it into a segment, getting the PersyId
///   - invoking the update_indexes/remove_from_indexes methods appropriately
///   - committing the transaction
pub trait Table {
    type Row;

    fn update_indexes<'c>(
        &self,
        id: PersyId,
        row: &Self::Row,
        db: PersyConnection<'c>,
    ) -> PRes<PersyConnection<'c>>;

    fn remove_from_indexes<'c>(
        &self,
        id: PersyId,
        db: PersyConnection<'c>,
    ) -> PRes<PersyConnection<'c>>;
}

/// This is a hypothetical replacement of database::schema::Pilot
pub struct Pilot {
    pub id: u64,
    pub account_id: u64,
    pub name: String,
}

pub struct PilotTable<'a> {
    by_id: Index<'a, PersyId, u64>,
    by_account_id: Index<'a, PersyId, u64>,
}

impl<'a> Table for PilotTable<'a> {
    type Row = Pilot;

    fn update_indexes<'c>(
        &self,
        id: PersyId,
        row: &Self::Row,
        mut db: PersyConnection<'c>,
    ) -> PRes<PersyConnection<'c>> {
        db = self.by_id.set(id.clone(), row.id, db)?;
        // TODO This needs to be a list, which persy supports, but
        // my Index type currently only has API calls that work with
        // singular instances of data.
        // I'm of two minds when I revisit this code: First would be
        // to have two separate base Index types for the unique styles
        // of Indexes. This would make usage more safe and explicit.
        // The other would be to keep the Index type as-is and add
        // the needed methods.
        db = self.by_account_id.set(id, row.account_id, db)?;
        Ok(db)
    }

    fn remove_from_indexes<'c>(
        &self,
        id: PersyId,
        db: PersyConnection<'c>,
    ) -> PRes<PersyConnection<'c>> {
        todo!()
    }
}
