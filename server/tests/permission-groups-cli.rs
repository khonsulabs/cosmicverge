use assert_cmd::prelude::*;
use cosmicverge_shared::permissions::{AccountPermission, Permission, Service};
use database::schema::{PermissionGroup, Statement};
use std::{env, process::Command, str::FromStr};

#[tokio::test]
async fn permission_groups() -> anyhow::Result<()> {
    database::test_util::initialize_exclusive_test().await;

    Command::cargo_bin("cosmicverge-server")?
        .arg("--database-url")
        .arg(env::var("TEST_DATABASE_URL")?)
        .arg("permission-group")
        .arg("test-group")
        .arg("create")
        .assert()
        .success();

    let group = PermissionGroup::find_by_name("test-group", database::pool()).await?;

    Command::cargo_bin("cosmicverge-server")?
        .arg("--database-url")
        .arg(env::var("TEST_DATABASE_URL")?)
        .arg("permission-group")
        .arg("test-group")
        .arg("add")
        .arg(Permission::Account(AccountPermission::TemporaryBan).to_string())
        .assert()
        .success();

    Command::cargo_bin("cosmicverge-server")?
        .arg("--database-url")
        .arg(env::var("TEST_DATABASE_URL")?)
        .arg("permission-group")
        .arg("test-group")
        .arg("add-service")
        .arg(Service::Universe.to_string())
        .assert()
        .success();

    let statements = Statement::list_for_group_id(group.id, database::pool()).await?;
    assert_eq!(2, statements.len());
    assert!(statements.iter().any(|s| matches!(
        s.permission
            .as_ref()
            .map(|p| Permission::from_str(p).ok())
            .flatten(),
        Some(Permission::Account(AccountPermission::TemporaryBan))
    )));
    assert!(statements.iter().any(|s| s.permission.is_none()
        && matches!(Service::from_str(&s.service), Ok(Service::Universe))));

    Command::cargo_bin("cosmicverge-server")?
        .arg("--database-url")
        .arg(env::var("TEST_DATABASE_URL")?)
        .arg("permission-group")
        .arg("test-group")
        .arg("remove")
        .arg(Permission::Account(AccountPermission::TemporaryBan).to_string())
        .assert()
        .success();

    Command::cargo_bin("cosmicverge-server")?
        .arg("--database-url")
        .arg(env::var("TEST_DATABASE_URL")?)
        .arg("permission-group")
        .arg("test-group")
        .arg("remove-service")
        .arg(Service::Universe.to_string())
        .assert()
        .success();

    assert!(Statement::list_for_group_id(group.id, database::pool())
        .await?
        .is_empty());

    group.delete(database::pool()).await?;

    Ok(())
}
