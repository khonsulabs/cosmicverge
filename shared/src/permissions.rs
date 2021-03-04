use std::collections::HashMap;

use once_cell::sync::Lazy;
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
pub enum AccountPermission {
    PermanentBan,
    TemporaryBan,
    List,
    View,
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
        static SERVICE_MAP: Lazy<HashMap<Service, Vec<Permission>>> = Lazy::new(|| {
            let mut permissions = HashMap::<Service, Vec<Permission>>::new();
            for permission in Permission::iter() {
                let service_permissions = permissions.entry(permission.service()).or_default();
                service_permissions.push(permission);
            }

            permissions
        });
        SERVICE_MAP[self].clone()
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
