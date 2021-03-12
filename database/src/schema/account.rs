use std::{collections::HashSet, str::FromStr};

use basws_server::prelude::Uuid;
use chrono::{DateTime, Utc};
use cli_table::Table;
use cosmicverge_shared::{
    permissions::{Permission, Service},
    protocol::Permissions,
};
use migrations::sqlx;

#[derive(Debug, Clone, Copy, Table)]
pub struct Account {
    pub id: i64,
    pub superuser: bool,
    pub created_at: DateTime<Utc>,
}

impl Account {
    pub async fn find_by_installation_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        installation_id: Uuid,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error> {
        match sqlx::query_as!(
            Self,
            "SELECT accounts.id, accounts.superuser, accounts.created_at FROM accounts INNER JOIN installations ON installations.account_id = accounts.id WHERE installations.id = $1",
            installation_id,
        )
            .fetch_one(executor)
            .await {
            Ok(result) => Ok(Some(result)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub async fn find_by_twitch_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        twitch_id: &str,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error> {
        match sqlx::query_as!(
                Self,
                "SELECT accounts.id, accounts.superuser, accounts.created_at FROM accounts INNER JOIN twitch_profiles ON twitch_profiles.account_id = accounts.id WHERE twitch_profiles.id = $1",
                twitch_id
            )
            .fetch_one(executor)
            .await {
            Ok(result) => Ok(Some(result)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub async fn find_by_twitch_username<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        username: &str,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error> {
        match sqlx::query_as!(
                Self,
                "SELECT accounts.id, accounts.superuser, accounts.created_at FROM accounts INNER JOIN twitch_profiles ON twitch_profiles.account_id = accounts.id WHERE LOWER(twitch_profiles.username) = LOWER($1)",
                username
            )
            .fetch_one(executor)
            .await {
            Ok(result) => Ok(Some(result)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(err)
        }
    }

    pub async fn load<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        account_id: i64,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error> {
        match sqlx::query_as!(
            Self,
            "SELECT accounts.id, accounts.superuser, accounts.created_at FROM accounts WHERE accounts.id = $1",
            account_id,
        )
        .fetch_one(executor)
        .await
        {
            Ok(result) => Ok(Some(result)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub async fn create<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        executor: E,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "INSERT INTO accounts DEFAULT VALUES RETURNING id, superuser, created_at"
        )
        .fetch_one(executor)
        .await
    }

    pub async fn save<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &self,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE accounts SET superuser = $2 WHERE id = $1",
            self.id,
            self.superuser
        )
        .execute(executor)
        .await
        .map(|_| ())
    }

    pub async fn permissions<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &self,
        executor: E,
    ) -> Result<Permissions, sqlx::Error> {
        if self.superuser {
            Ok(Permissions::SuperUser)
        } else {
            let mut permissions = HashSet::new();
            for row in sqlx::query!(
                r#"SELECT 
                    permission_group_statements.service,
                    permission_group_statements.permission
                FROM permission_group_statements
                INNER JOIN permission_groups ON permission_groups.id = permission_group_statements.permission_group_id
                INNER JOIN account_permission_groups ON account_permission_groups.permission_group_id = permission_groups.id
                WHERE account_permission_groups.account_id = $1"#,
                self.id,
            )
            .fetch_all(executor)
            .await? {
                // Purposely ignoring any errors parsing, this means that we deleted a permission
                if let Some(permission) = &row.permission {
                    if let Ok(permission)= Permission::from_str(permission) {
                        permissions.insert(permission);
                    }
                } else if let Ok(service) = Service::from_str(&row.service) {
                    for permission in service.permissions() {
                        permissions.insert(permission);
                    }
                }
            }

            Ok(Permissions::PermissionSet(permissions))
        }
    }

    pub async fn assign_permission_group<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &self,
        permission_group_id: i32,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO account_permission_groups (account_id, permission_group_id) VALUES ($1, $2)", 
            self.id, permission_group_id
        )
        .execute(executor).await.map(|_|())
    }
}

#[cfg(test)]
mod tests {
    use cosmicverge_shared::{
        permissions::{AccountPermission, GenericPermission},
        protocol::Permissions,
    };

    use super::*;
    use crate::{
        schema::{PermissionGroup, TwitchProfile},
        test_util::pool,
    };

    #[tokio::test]
    async fn account_permissions() -> sqlx::Result<()> {
        let mut tx = pool().await.begin().await?;
        let account = Account::create(&mut tx).await?;
        let permissions = account.permissions(&mut tx).await?;
        assert_eq!(permissions, Permissions::PermissionSet(HashSet::new()));

        let group =
            PermissionGroup::create(String::from("account-permissions-test-group"), &mut tx)
                .await?;
        account.assign_permission_group(group.id, &mut tx).await?;

        group
            .add_permission(Permission::Account(AccountPermission::View), &mut tx)
            .await?;
        group
            .add_all_service_permissions(Service::Universe, &mut tx)
            .await?;

        let permissions = dbg!(account.permissions(&mut tx).await?);
        assert!(permissions.has_permissions(&[
            Permission::Universe(GenericPermission::View),
            Permission::Universe(GenericPermission::List)
        ]));
        assert!(permissions.has_permission(Permission::Account(AccountPermission::View)));
        assert!(!permissions.has_permission(Permission::Account(AccountPermission::List)));

        Ok(())
    }

    #[tokio::test]
    async fn account_lookup() -> sqlx::Result<()> {
        let mut tx = pool().await.begin().await?;
        let account = Account::create(&mut tx).await?;
        assert_eq!(
            account.id,
            Account::load(account.id, &mut tx).await?.unwrap().id
        );

        TwitchProfile::associate(
            "account_lookup_twitch_id",
            account.id,
            "account_lookup_twitch_username",
            &mut tx,
        )
        .await?;

        assert_eq!(
            account.id,
            Account::find_by_twitch_id("account_lookup_twitch_id", &mut tx)
                .await?
                .unwrap()
                .id
        );

        assert_eq!(
            account.id,
            Account::find_by_twitch_username("account_lookup_twitch_username", &mut tx)
                .await?
                .unwrap()
                .id
        );

        Ok(())
    }
}
