use chrono::{DateTime, Utc};
use cli_table::Table;
use cosmicverge_shared::permissions::{Permission, Service};

#[derive(Debug, Table)]
pub struct PermissionGroup {
    pub id: i32,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl PermissionGroup {
    pub async fn create<
        'e,
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
        S: Into<String> + Send,
    >(
        name: S,
        executor: E,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "INSERT INTO permission_groups (name) VALUES ($1) RETURNING id, name, created_at",
            name.into()
        )
        .fetch_one(executor)
        .await
    }

    pub async fn find_by_name<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        name: &str,
        executor: E,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "SELECT id, name, created_at FROM permission_groups WHERE name = $1",
            name
        )
        .fetch_one(executor)
        .await
    }

    pub async fn add_permission<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &self,
        permission: Permission,
        executor: E,
    ) -> Result<Statement, sqlx::Error> {
        sqlx::query_as!(
            Statement,
            "INSERT INTO permission_group_statements (permission_group_id, service, permission) VALUES ($1, $2, $3) RETURNING id, service, permission, created_at",
            self.id, permission.service().to_string(), permission.to_string()
        )
        .fetch_one(executor)
        .await
    }

    pub async fn add_all_service_permissions<
        'e,
        A: sqlx::Acquire<'e, Database = sqlx::Postgres> + Send,
    >(
        &self,
        service: Service,
        executor: A,
    ) -> Result<Statement, sqlx::Error> {
        let mut conn = executor.acquire().await?;
        sqlx::query!(
            "DELETE FROM permission_group_statements WHERE permission_group_id = $1 AND service = $2",
            self.id, service.to_string()
        )
        .execute(&mut *conn)
        .await?;
        sqlx::query_as!(
            Statement,
            "INSERT INTO permission_group_statements (permission_group_id, service) VALUES ($1, $2) RETURNING id, service, permission, created_at",
            self.id, service.to_string()
        )
        .fetch_one(&mut *conn)
        .await
    }

    pub async fn remove_permission<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &self,
        permission: Permission,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM permission_group_statements WHERE permission_group_id = $1 AND permission = $2",
            self.id, permission.to_string()
        )
        .execute(executor)
        .await.map(|_| ())
    }

    pub async fn remove_all_service_permissions<
        'e,
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    >(
        &self,
        service: Service,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
                "DELETE FROM permission_group_statements WHERE permission_group_id = $1 AND service = $2",
                self.id, service.to_string()
            )
            .execute(executor)
            .await.map(|_| ())
    }

    pub async fn delete<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &self,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM permission_groups WHERE id = $1", self.id)
            .execute(executor)
            .await
            .map(|_| ())
    }
}

#[derive(Debug, Table)]
pub struct Statement {
    pub id: i32,
    pub service: String,
    #[table(display_fn = "display_permission")]
    pub permission: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Helper used when formatting a Permission for viewing on the command line
///
/// Returns "*" if None is passed, otherwise a clone of the contained string.
/// The signature of &Option<String> is required due to how the `cli_table::Table`
/// macro works
fn display_permission(value: &Option<String>) -> String {
    value.as_deref().unwrap_or("*").to_owned()
}

impl Statement {
    pub async fn list_for_group_id<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        group_id: i32,
        executor: E,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            "SELECT id, service, permission, created_at FROM permission_group_statements WHERE permission_group_id = $1",
            group_id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn delete<'e, E: sqlx::Executor<'e, Database = sqlx::Postgres>>(
        &self,
        executor: E,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM permission_group_statements WHERE id = $1",
            self.id
        )
        .execute(executor)
        .await
        .map(|_| ())
    }
}

#[tokio::test]
async fn create_lookup_delete_test() -> anyhow::Result<()> {
    crate::test_util::initialize_safe_test().await;

    let mut tx = migrations::pool().begin().await?;
    let group = PermissionGroup::create("create_and_lookup_test", &mut tx).await?;

    assert_eq!(
        group.id,
        PermissionGroup::find_by_name("create_and_lookup_test", &mut tx)
            .await?
            .id
    );

    group.delete(&mut tx).await?;

    assert!(
        PermissionGroup::find_by_name("create_and_lookup_test", &mut tx)
            .await
            .is_err()
    );

    Ok(())
}

#[test]
fn display_permission_test() {
    assert_eq!("*", &display_permission(&None));
    assert_eq!("value", &display_permission(&Some(String::from("value"))));
}
