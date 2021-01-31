use sqlx::{postgres::PgListener, Executor};

use database::{
    basws_server::{prelude::Uuid, Handle, Server},
    cosmicverge_shared::CosmicVergeResponse,
    pool, sqlx,
};

use crate::server::{ConnectedAccount, CosmicVergeServer};
use database::schema::{convert_db_pilots, Pilot};

pub async fn pg_notify_loop(websockets: Server<CosmicVergeServer>) -> Result<(), anyhow::Error> {
    let pool = pool();
    let mut listener = PgListener::connect_with(&pool).await?;
    listener.listen_all(vec!["installation_login"]).await?;
    while let Ok(notification) = listener.recv().await {
        info!(
            "Got notification: {} {}",
            notification.channel(),
            notification.payload()
        );
        if notification.channel() == "installation_login" {
            // The payload is the installation_id that logged in.
            let installation_id = Uuid::parse_str(notification.payload())?;
            if let Ok(account) = ConnectedAccount::lookup(installation_id).await {
                let user_id = account.account.id;
                websockets
                    .associate_installation_with_account(installation_id, Handle::new(account))
                    .await?;

                let pilots = convert_db_pilots(Pilot::list_by_account_id(user_id, pool).await?);
                websockets
                    .send_to_installation_id(
                        installation_id,
                        CosmicVergeResponse::Authenticated { user_id, pilots },
                    )
                    .await;
            }
        }
    }
    panic!("Error on postgres listening");
}

pub async fn notify<S: ToString>(channel: &'static str, payload: S) -> Result<(), sqlx::Error> {
    let mut connection = pool().acquire().await?;
    connection
        .execute(&*format!("NOTIFY {}, '{}'", channel, payload.to_string()))
        .await?;
    Ok(())
}
