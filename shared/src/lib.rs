#[macro_use]
extern crate log;

pub use euclid;
pub use num_traits;
pub use strum;
pub use strum_macros;
#[cfg(feature="persyutil")]
pub mod persyutil;
pub mod protocol;
pub mod ships;
pub mod solar_system_simulation;
pub mod solar_systems;
mod version;

pub const MAX_PILOTS_PER_ACCOUNT: usize = 2;
