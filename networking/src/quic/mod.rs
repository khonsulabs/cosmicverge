//! QUIC enabled socket implementation.

mod connection;
mod endpoint;
mod task;
mod util;

// TODO: fix lint or allow it, this is horrible
#[allow(unreachable_pub)]
pub use connection::{Connection, Incoming, Receiver, Sender};
#[allow(unreachable_pub)]
pub use endpoint::Endpoint;
use task::Task;
use util::StreamExtExt;
