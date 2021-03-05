use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::permissions::Permission;

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub permissions: AccountPermissions,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AccountPermissions {
    SuperUser,
    PermissionSet(HashSet<Permission>),
}

impl AccountPermissions {
    pub fn has_permission(&self, permission: &Permission) -> bool {
        match self {
            Self::SuperUser => true,
            Self::PermissionSet(permissions) => permissions.contains(permission),
        }
    }
    pub fn has_permissions(&self, permissions_to_check: &[Permission]) -> bool {
        match self {
            Self::SuperUser => true,
            Self::PermissionSet(permissions) => permissions_to_check
                .iter()
                .map(|p| permissions.contains(p))
                .all(|c| c),
        }
    }
}
