use std::collections::HashMap;

use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

#[derive(
    Clone,
    Copy,
    Hash,
    Eq,
    PartialEq,
    Debug,
    Serialize,
    Deserialize,
    strum_macros::EnumString,
    strum_macros::ToString,
    strum_macros::EnumIter,
)]
pub enum Permission {
    AccountPermanentBan,
    AccountTemporaryBan,
    AccountList,
    AccountView,
    UniverseList,
    UniverseEdit,
}

#[derive(
    Hash,
    Eq,
    PartialEq,
    Debug,
    Serialize,
    Deserialize,
    strum_macros::EnumString,
    strum_macros::ToString,
)]
pub enum Service {
    Account,
    Universe,
}

impl Service {
    pub fn permissions(&self) -> Vec<Permission> {
        static SERVICE_MAP: OnceCell<HashMap<Service, Vec<Permission>>> = OnceCell::new();
        SERVICE_MAP.get_or_init(|| {
            let mut permissions = HashMap::<Service, Vec<Permission>>::new();
            for permission in Permission::iter() {
                let service_permissions = permissions.entry(permission.service()).or_default();
                service_permissions.push(permission);
            }

            permissions
        })[self]
            .clone()
    }
}

impl Permission {
    pub fn service(&self) -> Service {
        match self {
            Permission::AccountPermanentBan
            | Permission::AccountTemporaryBan
            | Permission::AccountList
            | Permission::AccountView => Service::Account,
            Permission::UniverseList | Permission::UniverseEdit => Service::Universe,
        }
    }
}
