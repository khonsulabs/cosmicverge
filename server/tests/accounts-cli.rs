use assert_cmd::prelude::*;
use database::schema::{Account, TwitchProfile};
use std::{env, process::Command};

#[tokio::test]
async fn account_superuser_command() -> anyhow::Result<()> {
    database::test_util::initialize_exclusive_test().await;

    let account = Account::create(database::pool()).await?;
    TwitchProfile::associate(
        "fake_twitch_id",
        account.id,
        "twitch_username",
        database::pool(),
    )
    .await?;

    Command::cargo_bin("cosmicverge-server")?
        .arg("--database-url")
        .arg(env::var("TEST_DATABASE_URL")?)
        .arg("account")
        .arg("--id")
        .arg(account.id.to_string())
        .arg("set-super-user")
        .assert()
        .success();
    assert!(
        Account::load(account.id, database::pool())
            .await?
            .unwrap()
            .superuser
    );

    Command::cargo_bin("cosmicverge-server")?
        .arg("--database-url")
        .arg(env::var("TEST_DATABASE_URL")?)
        .arg("account")
        .arg("--id")
        .arg(account.id.to_string())
        .arg("set-normal-user")
        .assert()
        .success();
    assert!(
        !Account::load(account.id, database::pool())
            .await?
            .unwrap()
            .superuser
    );

    Command::cargo_bin("cosmicverge-server")?
        .arg("--database-url")
        .arg(env::var("TEST_DATABASE_URL")?)
        .arg("account")
        .arg("--twitch")
        .arg("Twitch_Username") // testing case insensitivity
        .arg("set-super-user")
        .assert()
        .success();

    assert!(
        Account::load(account.id, database::pool())
            .await?
            .unwrap()
            .superuser
    );

    Command::cargo_bin("cosmicverge-server")?
        .arg("--database-url")
        .arg(env::var("TEST_DATABASE_URL")?)
        .arg("account")
        .arg("--twitch")
        .arg("invalid_username") // testing case insensitivity
        .arg("set-super-user")
        .assert()
        .failure();

    TwitchProfile::delete("fake_twitch_id", database::pool()).await?;
    account.delete(database::pool()).await?;

    Ok(())
}
