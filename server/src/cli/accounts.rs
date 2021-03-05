use database::schema::Account;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(about = "commands to execute")]
/// commands that the server can execute
pub enum AccountCommand {
    /// Sets the superuser flag on an account
    SetSuperUser,
    /// Clears the superuser flag on an account
    SetNormalUser,
}

pub async fn handle_command(
    id: Option<i64>,
    twitch: Option<String>,
    command: AccountCommand,
) -> anyhow::Result<()> {
    database::initialize().await;
    let account = if let Some(id) = id {
        Account::load(id, database::pool()).await?
    } else if let Some(twitch) = twitch {
        Account::find_by_twitch_username(&twitch, database::pool()).await?
    } else {
        anyhow::bail!("Either id or twitch parameters must be specified for account commands")
    };

    match account {
        Some(mut account) => match command {
            AccountCommand::SetSuperUser => {
                account.superuser = true;
                account.save(database::pool()).await?;
                Ok(())
            }
            AccountCommand::SetNormalUser => {
                account.superuser = false;
                account.save(database::pool()).await?;
                Ok(())
            }
        },
        None => anyhow::bail!("Account not found"),
    }
}
