use cli_table::{Cell as _, Style as _, Table as _};
use database::schema::Account;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Command {
    /// The ID of the account
    #[structopt(long)]
    id: Option<i64>,

    /// The Twitch handle to look up to find the account
    #[structopt(long)]
    twitch: Option<String>,

    /// The command to execute
    #[structopt(subcommand)]
    operation: Operation,
}

#[derive(StructOpt, Debug)]
#[structopt(about = "commands to execute")]
/// commands that the server can execute
enum Operation {
    /// Views the account
    View,
    /// Sets the superuser flag on an account
    SetSuperUser,
    /// Clears the superuser flag on an account
    SetNormalUser,
}

pub async fn handle_command(command: Command) -> anyhow::Result<()> {
    database::initialize().await;
    let account = if let Some(id) = command.id {
        Account::load(id, database::pool()).await?
    } else if let Some(twitch) = &command.twitch {
        Account::find_by_twitch_username(twitch, database::pool()).await?
    } else {
        anyhow::bail!("Either id or twitch parameters must be specified for account commands")
    };

    match account {
        Some(mut account) => {
            match command.operation {
                Operation::SetSuperUser => {
                    account.superuser = true;
                    account.save(database::pool()).await?;
                }
                Operation::SetNormalUser => {
                    account.superuser = false;
                    account.save(database::pool()).await?;
                }
                Operation::View => {}
            }

            print_account(account)?;

            Ok(())
        }
        None => anyhow::bail!("Account not found"),
    }
}

fn print_account(account: Account) -> std::io::Result<()> {
    println!("Account:");
    cli_table::print_stdout(
        vec![
            vec!["ID".cell().bold(true), account.id.cell()],
            vec!["Superuser".cell().bold(true), account.superuser.cell()],
            vec!["Created At".cell().bold(true), account.created_at.cell()],
        ]
        .table(),
    )
}
