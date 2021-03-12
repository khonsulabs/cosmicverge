use cli_table::WithTitle as _;
use cosmicverge_shared::permissions::{Permission, Service};
use database::schema::{PermissionGroup, Statement};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(about = "commands to execute")]
/// commands that the server can execute
pub struct Command {
    /// The name of the permission group
    group_name: String,

    #[structopt(subcommand)]
    operation: Operation,
}

#[derive(StructOpt, Debug)]
enum Operation {
    /// Creates a new permission group
    Create,
    /// Views a permission group
    View,
    /// Adds a permission
    Add { permission: Permission },
    /// Removes a permission
    Remove { permission: Permission },
    /// Grants all permissions to a service
    AddService { service: Service },
    /// Removes all permissions from a service
    RemoveService { service: Service },
}

impl Command {
    pub async fn execute(self) -> anyhow::Result<()> {
        database::initialize().await;

        let group = match self.operation {
            Operation::Create => PermissionGroup::create(self.group_name, database::pool()).await?,
            other => {
                let group =
                    PermissionGroup::find_by_name(&self.group_name, database::pool()).await?;
                match other {
                    Operation::Add { permission } => {
                        group.add_permission(permission, database::pool()).await?;
                    }
                    Operation::Remove { permission } => {
                        group
                            .remove_permission(permission, database::pool())
                            .await?;
                    }
                    Operation::AddService { service } => {
                        group
                            .add_all_service_permissions(service, database::pool())
                            .await?;
                    }
                    Operation::RemoveService { service } => {
                        group
                            .remove_all_service_permissions(service, database::pool())
                            .await?;
                    }
                    Operation::View => {}
                    Operation::Create => unreachable!(),
                }
                group
            }
        };

        let permissions = Statement::list_for_group_id(group.id, database::pool()).await?;

        println!("Permission Group:");
        cli_table::print_stdout(vec![group].with_title())?;
        println!("Current Permissions:");
        cli_table::print_stdout(permissions.with_title())?;

        Ok(())
    }
}
