use cli_table::WithTitle as _;
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

impl Command {
    pub async fn execute(self, database_url: Option<String>) -> anyhow::Result<()> {
        database::initialize(database_url).await;
        let account = if let Some(id) = self.id {
            Account::load(id, database::pool()).await?
        } else if let Some(twitch) = &self.twitch {
            Account::find_by_twitch_username(twitch, database::pool()).await?
        } else {
            anyhow::bail!("Either id or twitch parameters must be specified for account commands")
        };

        match account {
            Some(mut account) => {
                match self.operation {
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

                println!("Account:");
                cli_table::print_stdout(vec![account].with_title())?;

                Ok(())
            }
            None => anyhow::bail!("Account not found"),
        }
    }
}
