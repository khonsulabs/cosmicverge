use basws_shared::{Version, VersionReq};

#[must_use]
pub fn cosmic_verge_protocol_version() -> Version {
    Version::parse("0.0.1").unwrap()
}

#[must_use]
pub fn cosmic_verge_protocol_version_requirements() -> VersionReq {
    VersionReq::parse("=0.0.1").unwrap()
}

mod account;
mod installation;
mod navigation;
mod oauth_provider;
mod pilot;
mod request;
mod response;

pub use self::{
    account::*, installation::*, navigation::*, oauth_provider::*, pilot::*, request::*,
    response::*,
};
