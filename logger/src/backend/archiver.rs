use std::convert::TryInto;

use async_trait::async_trait;
use flume::{bounded, Receiver, Sender};
use futures::Future;
use once_cell::sync::Lazy;
use serde::Serialize;
use sled::{transaction::TransactionError, IVec};

use crate::{backend::Backend, Log};

#[derive(Debug)]
pub struct Archiver {
    db: sled::Db,
    archive_signal: Option<Sender<()>>,
}

#[derive(Serialize)]
enum Key {
    CurrentLogId,
    LastArchivedId,
    Log(u64),
}

static CURRENT_ID_KEY: Lazy<Vec<u8>> =
    Lazy::new(|| serde_cbor::to_vec(&Key::CurrentLogId).unwrap());

static LAST_ARCHIVED_ID_KEY: Lazy<Vec<u8>> =
    Lazy::new(|| serde_cbor::to_vec(&Key::LastArchivedId).unwrap());

impl Archiver {
    #[must_use]
    pub const fn new(db: sled::Db) -> Self {
        Self {
            db,
            archive_signal: None,
        }
    }

    pub fn run(&mut self) -> impl Future<Output = anyhow::Result<()>> {
        let (sender, receiver) = bounded(1);
        self.archive_signal = Some(sender);
        archive_loop(receiver, self.db.clone())
    }
}

#[async_trait]
impl Backend for Archiver {
    async fn process_log(&mut self, log: &Log) -> anyhow::Result<()> {
        let tree = self.db.open_tree(b"logs")?;
        tree.transaction::<_, _, anyhow::Error>(|tx| {
            let current_log_id = tx
                .get(&*CURRENT_ID_KEY)?
                .map(convert_ivec_to_u64)
                .unwrap_or_default();

            let new_id = current_log_id.wrapping_add(1);
            tx.insert(CURRENT_ID_KEY.clone(), new_id.to_be_bytes().to_vec())?;
            tx.insert(
                serde_cbor::to_vec(&Key::Log(new_id)).unwrap(),
                serde_json::to_vec(log).unwrap(),
            )?;

            Ok(())
        })
        .map_transaction_error()?;

        if let Some(signal) = &self.archive_signal {
            let _ = signal.try_send(());
        }

        Ok(())
    }
}

async fn archive_loop(signal: Receiver<()>, db: sled::Db) -> anyhow::Result<()> {
    loop {
        archive_logs(&db).await?;
        signal.recv_async().await?;
    }
}

async fn archive_logs(db: &sled::Db) -> anyhow::Result<()> {
    let tree = db.open_tree(b"logs")?;

    while matches!(archive_one_batch(&tree).await?, ArchiveStatus::LogsPending) {}

    Ok(())
}

enum ArchiveStatus {
    Archived,
    LogsPending,
}

async fn archive_one_batch(tree: &sled::Tree) -> anyhow::Result<ArchiveStatus> {
    // Get the current log ID for the data stored in sled. If no data is
    // present, exit early.
    let current_log_id = match tree.get(&*CURRENT_ID_KEY)?.map(convert_ivec_to_u64) {
        Some(id) => id,
        None => return Ok(ArchiveStatus::Archived),
    };

    // Loop over a range of ids to archive. This logic takes care in the
    // event enough log messages are generated to wrap a usize. In the event
    // of a wrap, the batch will be forced to end at at usize::MAX regardless
    // of how small that batch ends up being. The remaining will be synced
    // up in a second batch
    let last_archived_id = tree
        .get(&*LAST_ARCHIVED_ID_KEY)?
        .map(convert_ivec_to_u64)
        .unwrap_or_default();
    let first_id_to_archive = last_archived_id.wrapping_add(1);

    let range = if last_archived_id > current_log_id {
        first_id_to_archive..=u64::MAX
    } else {
        first_id_to_archive..=current_log_id
    };
    if range.start() >= range.end() {
        return Ok(ArchiveStatus::Archived);
    }

    println!(
        "Range: {:?}, last archived {}, current {}",
        range, last_archived_id, current_log_id
    );
    let mut entries_to_archive = Vec::with_capacity((range.end() - range.start()) as usize);
    for id in range.clone() {
        if let Some(ivec) = tree.get(&log_key_for_id(id))? {
            match serde_json::from_slice::<Log>(&ivec) {
                Ok(log) => {
                    entries_to_archive.push(log);
                }
                Err(err) => eprintln!("Error serializing log from sled: {:?}", err),
            }
        }
    }

    database::schema::Log::insert_batch(
        &entries_to_archive
            .into_iter()
            .map(|l| l.into())
            .collect::<Vec<_>>(),
    )
    .await?;

    tree.transaction::<_, _, anyhow::Error>(|tx| {
        for id in range.clone() {
            tx.remove(log_key_for_id(id))?;
        }

        tx.insert(
            LAST_ARCHIVED_ID_KEY.clone(),
            range.end().to_be_bytes().to_vec(),
        )?;

        if current_log_id == last_archived_id {
            Ok(ArchiveStatus::Archived)
        } else {
            Ok(ArchiveStatus::LogsPending)
        }
    })
    .map_transaction_error()
}

trait TransactionResultExt<T> {
    fn map_transaction_error(self) -> anyhow::Result<T>;
}

impl<T> TransactionResultExt<T> for Result<T, TransactionError<anyhow::Error>> {
    fn map_transaction_error(self) -> anyhow::Result<T> {
        self.map_err(|err| match err {
            TransactionError::Abort(err) => err,
            TransactionError::Storage(err) => anyhow::Error::from(err),
        })
    }
}

fn log_key_for_id(id: u64) -> Vec<u8> {
    serde_cbor::to_vec(&Key::Log(id)).unwrap()
}

#[allow(clippy::clippy::needless_pass_by_value)] // for ergonomics of map()
fn convert_ivec_to_u64(vec: IVec) -> u64 {
    let array: [u8; 8] = vec.to_vec().try_into().unwrap_or_default();
    u64::from_be_bytes(array)
}

#[tokio::test]
async fn archiver_test() -> anyhow::Result<()> {
    use crate::{test_util::TestMessage, Manager};
    use std::{sync::Arc, time::Duration};

    // We want to ensure our query of logs will only be the ones we insert below
    database::test_util::initialize_exclusive_test().await;

    // Create/open the sled database, and drop the logs tree if it exists already
    let path = std::env::temp_dir().join("archiver_test.sled");
    let db = sled::open(&path)?;
    db.drop_tree(b"logs")?;
    let mut test_backend = Archiver::new(db);
    tokio::spawn(test_backend.run());

    let sender = Manager::default()
        .with_backend(test_backend)
        .launch(|task| {
            tokio::spawn(task);
        });

    sender.try_send(Arc::new(Log::info(TestMessage::A)))?;
    sender.try_send(Arc::new(Log::info(TestMessage::B)))?;
    sender.try_send(Arc::new(Log::info(TestMessage::A).with("key", "value")?))?;

    // TODO I don't love hard-coding 100ms, but this one is trickier than the
    // other tests because it involves latency to an external database. Granted,
    // if it's localhost, a couple of ms should be plenty. For the other tests,
    // I've been thinking of having a way to await a flush of the backends.
    // Unfortunately for the archiver, it buffers the backend, so we still have
    // to wait for that backend to also finish. Because of those complications,
    // we're just waiting 100ms for now. @ecton
    tokio::time::sleep(Duration::from_millis(100)).await;

    // This returns entries in reverse chronology
    let entries = database::schema::Log::list_recent(100).await?;
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].message, "A");
    assert_eq!(
        entries[0].payload,
        Some(serde_json::json!({"key": "value"}))
    );
    assert_eq!(entries[1].message, "B");
    assert_eq!(entries[2].message, "A");
    assert_eq!(entries[2].payload, None);

    Ok(())
}
