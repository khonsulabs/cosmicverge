use cli_table::{Cell as _, Style as _, Table as _};
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

pub async fn handle_command(group_command: Command) -> anyhow::Result<()> {
    database::initialize().await;

    let group = match group_command.operation {
        Operation::Create => {
            PermissionGroup::create(group_command.group_name, database::pool()).await?
        }
        other => {
            let group =
                PermissionGroup::find_by_name(&group_command.group_name, database::pool()).await?;
            match other {
                Operation::Add { permission } => {
                    group.add_permission(permission, database::pool()).await?;
                }
                Operation::Remove { permission } => {
                    group.add_permission(permission, database::pool()).await?;
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

    print_group(group)?;
    print_permissions(permissions)?;

    Ok(())
}

fn print_group(group: PermissionGroup) -> std::io::Result<()> {
    println!("Permission Group:");
    cli_table::print_stdout(
        vec![
            vec!["ID".cell().bold(true), group.id.cell()],
            vec!["Name".cell().bold(true), group.name.cell()],
            vec!["Created At".cell().bold(true), group.created_at.cell()],
        ]
        .table(),
    )
}

fn print_permissions(permissions: Vec<Statement>) -> std::io::Result<()> {
    let permissions = permissions
        .into_iter()
        .map(|permission| {
            vec![
                permission.service.cell(),
                permission.permission.as_deref().unwrap_or("*").cell(),
            ]
        })
        .table()
        .title(vec![
            "Service".cell().bold(true),
            "Permission".cell().bold(true),
        ]);

    println!("Current Permissions:");
    cli_table::print_stdout(permissions)
}
