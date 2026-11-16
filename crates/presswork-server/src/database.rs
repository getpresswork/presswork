use std::time::Duration;

use sqlx::PgPool;
use sqlx::migrate::MigrateError;
use sqlx::postgres::PgPoolOptions;

const MAX_CONNECTIONS: u32 = 16;
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) async fn connect(url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect(url)
        .await
}

pub(crate) async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub(crate) async fn is_ready(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").execute(pool).await.is_ok()
}

#[cfg(test)]
pub(crate) fn unreachable() -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy("postgres://presswork@127.0.0.1:1/presswork")
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::test_database;

    #[tokio::test]
    async fn an_unreachable_database_is_not_ready() {
        let pool = unreachable();

        assert!(!is_ready(&pool).await);
    }

    #[tokio::test]
    async fn the_migrations_make_the_organizations_table() {
        let Some(database) = test_database().await else {
            return;
        };

        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM organizations")
            .fetch_one(&database.pool)
            .await
            .unwrap();

        assert_eq!(count, 0);
        assert!(is_ready(&database.pool).await);
    }
}
