use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};

use crate::database;

const URL_VARIABLE: &str = "PRESSWORK_TEST_DATABASE_URL";

pub(crate) struct TestDatabase {
    pub(crate) pool: PgPool,
}

pub(crate) async fn test_database() -> Option<TestDatabase> {
    let Ok(url) = std::env::var(URL_VARIABLE) else {
        eprintln!("{URL_VARIABLE} is not set, so this database test did not run.");
        return None;
    };
    let server = PgPool::connect(&url).await.unwrap();
    let name = unique_name();
    sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&server)
        .await
        .unwrap();
    let options: PgConnectOptions = url.parse().unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect_with(options.database(&name))
        .await
        .unwrap();
    database::migrate(&pool).await.unwrap();
    Some(TestDatabase { pool })
}

fn unique_name() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    format!(
        "presswork_test_{}_{nanos}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}
