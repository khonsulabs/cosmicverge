use std::{
    fmt::{Display, Write},
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

#[derive(Clone, Copy, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum Permission {
    Account(AccountPermission),
    Universe(GenericPermission),
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

impl Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (service, permission) = match self {
            Permission::Account(perm) => (Service::Account, perm.to_string()),
            Permission::Universe(perm) => (Service::Universe, perm.to_string()),
        };
        f.write_str(&service.to_string())?;
        f.write_char('(')?;
        f.write_str(&permission)?;
        f.write_char(')')
    }
}

impl FromStr for Permission {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut service = String::new();
        let mut permission = String::new();
        enum State {
            InService,
            InPermission,
            AtEnd,
        }
        let mut state = State::InService;

        for c in s.chars() {
            match c {
                '(' => {
                    state = State::InPermission;
                }
                ')' => {
                    state = State::AtEnd;
                }
                _ => match state {
                    State::InService => service.push(c),
                    State::InPermission => permission.push(c),
                    State::AtEnd => anyhow::bail!("junk found after close paren"),
                },
            }
        }

        let service = Service::from_str(&service)?;

        let permission = match service {
            Service::Account => Permission::Account(AccountPermission::from_str(&permission)?),
            Service::Universe => Permission::Universe(GenericPermission::from_str(&permission)?),
        };

        Ok(permission)
    }
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
pub enum GenericPermission {
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
    strum_macros::EnumIter,
)]
pub enum Service {
    Account,
    Universe,
}

impl Service {
    #[must_use]
    pub fn permissions(&self) -> Vec<Permission> {
        match self {
            Service::Account => AccountPermission::iter().map(Permission::Account).collect(),
            Service::Universe => GenericPermission::iter()
                .map(Permission::Universe)
                .collect(),
        }
    }
}

impl Permission {
    #[must_use]
    pub const fn service(self) -> Service {
        match self {
            Permission::Account(_) => Service::Account,
            Permission::Universe(_) => Service::Universe,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use strum::IntoEnumIterator;

    use super::{Permission, Service};

    #[test]
    fn test_serialization() -> anyhow::Result<()> {
        let all_permissions = Service::iter().flat_map(|service| service.permissions());
        for permission in all_permissions {
            let serialized = permission.to_string();
            let deserialized = Permission::from_str(&serialized)?;
            assert_eq!(permission, deserialized);
        }

        Ok(())
    }
}
