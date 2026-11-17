use std::sync::Arc;
use std::time::Duration;

use presswork_api::Work;
use presswork_api::live::ServerMessage;
use sqlx::{PgConnection, PgPool};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::library::LibraryError;
use crate::routes::AppState;

pub(crate) const READ_TEXT: &str = "read_text";
pub(crate) const OCR: &str = "ocr";
pub(crate) const WORKERS: usize = 2;
const MAX_ATTEMPTS: i32 = 3;
const RETRY_WAITS: [Duration; 2] = [Duration::from_secs(10), Duration::from_secs(60)];
const LEASE: Duration = Duration::from_secs(600);
const HEARTBEAT: Duration = Duration::from_secs(200);
const POLL: Duration = Duration::from_secs(5);
const UNREADABLE: &str = "Presswork cannot read the pages of the file. Upload the file again.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Job {
    pub(crate) id: Uuid,
    pub(crate) kind: String,
    pub(crate) subject: Uuid,
    pub(crate) attempts: i32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Jobs {
    wake: Arc<Notify>,
}

impl Jobs {
    pub(crate) fn wake(&self) {
        self.wake.notify_one();
    }
}

pub(crate) async fn add(
    connection: &mut PgConnection,
    kind: &str,
    subject: Uuid,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO jobs (id, kind, subject, state) VALUES ($1, $2, $3, 'queued')")
        .bind(id)
        .bind(kind)
        .bind(subject)
        .execute(connection)
        .await?;
    Ok(id)
}

pub(crate) async fn claim(pool: &PgPool) -> Result<Option<Job>, sqlx::Error> {
    let row: Option<(Uuid, String, Uuid, i32)> = sqlx::query_as(
        "UPDATE jobs SET state = 'running', attempts = attempts + 1, \
         locked_until = now() + make_interval(secs => $1), updated_at = now() \
         WHERE id = (SELECT id FROM jobs \
             WHERE (state = 'queued' AND run_after <= now()) \
                OR (state = 'running' AND locked_until < now()) \
             ORDER BY run_after, id FOR UPDATE SKIP LOCKED LIMIT 1) \
         RETURNING id, kind, subject, attempts",
    )
    .bind(LEASE.as_secs_f64())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id, kind, subject, attempts)| Job {
        id,
        kind,
        subject,
        attempts,
    }))
}

pub(crate) async fn extend(pool: &PgPool, job: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE jobs SET locked_until = now() + make_interval(secs => $2), updated_at = now() \
         WHERE id = $1 AND state = 'running'",
    )
    .bind(job)
    .bind(LEASE.as_secs_f64())
    .execute(pool)
    .await?;
    Ok(())
}

struct Lease(tokio::task::JoinHandle<()>);

impl Drop for Lease {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn keep_lease(pool: PgPool, job: Uuid) {
    loop {
        tokio::time::sleep(HEARTBEAT).await;
        if let Err(error) = extend(&pool, job).await {
            tracing::warn!(%error, %job, "A job did not extend its lease");
        }
    }
}

pub(crate) async fn finish(pool: &PgPool, job: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE jobs SET state = 'done', locked_until = NULL, updated_at = now() WHERE id = $1",
    )
    .bind(job)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn fail(
    pool: &PgPool,
    job: &Job,
    error: &str,
    permanent: bool,
) -> Result<bool, sqlx::Error> {
    let wait = if permanent {
        None
    } else {
        retry_wait(job.attempts)
    };
    if let Some(wait) = wait {
        sqlx::query(
            "UPDATE jobs SET state = 'queued', run_after = now() + make_interval(secs => $2), \
             locked_until = NULL, last_error = $3, updated_at = now() WHERE id = $1",
        )
        .bind(job.id)
        .bind(wait.as_secs_f64())
        .bind(error)
        .execute(pool)
        .await?;
        return Ok(false);
    }
    sqlx::query(
        "UPDATE jobs SET state = 'failed', locked_until = NULL, last_error = $2, \
         updated_at = now() WHERE id = $1",
    )
    .bind(job.id)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(true)
}

fn retry_wait(attempts: i32) -> Option<Duration> {
    if attempts >= MAX_ATTEMPTS {
        return None;
    }
    usize::try_from(attempts)
        .ok()
        .and_then(|attempts| attempts.checked_sub(1))
        .and_then(|index| RETRY_WAITS.get(index))
        .copied()
}

pub(crate) fn start(state: &AppState) {
    for _ in 0..WORKERS {
        let state = state.clone();
        tokio::spawn(async move {
            loop {
                match run_one(&state).await {
                    Ok(true) => {}
                    Ok(false) => {
                        tokio::select! {
                            () = state.jobs.wake.notified() => {}
                            () = tokio::time::sleep(POLL) => {}
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "A worker cannot take a job");
                        tokio::time::sleep(POLL).await;
                    }
                }
            }
        });
    }
}

pub(crate) async fn run_one(state: &AppState) -> Result<bool, sqlx::Error> {
    let Some(job) = claim(&state.database).await? else {
        return Ok(false);
    };
    if job.attempts > MAX_ATTEMPTS {
        fail(&state.database, &job, "the job stopped too often", true).await?;
        give_up(state, &job).await;
        return Ok(true);
    }
    let lease = Lease(tokio::spawn(keep_lease(state.database.clone(), job.id)));
    let result = match job.kind.as_str() {
        READ_TEXT => state.library.read_text(&state.database, job.subject).await,
        OCR => state.library.recognize(&state.database, job.subject).await,
        _ => Err(LibraryError::NotFound),
    };
    drop(lease);
    match result {
        Ok(work) => {
            finish(&state.database, job.id).await?;
            if let Some(work) = work {
                tell(state, work).await;
            }
        }
        Err(LibraryError::NotFound) => finish(&state.database, job.id).await?,
        Err(error) => {
            tracing::warn!(%error, job = %job.id, kind = %job.kind, "A job failed");
            if fail(
                &state.database,
                &job,
                &error.to_string(),
                error.is_permanent(),
            )
            .await?
            {
                give_up(state, &job).await;
            }
        }
    }
    Ok(true)
}

async fn give_up(state: &AppState, job: &Job) {
    let result = match job.kind.as_str() {
        READ_TEXT => crate::library::mark_failed(&state.database, job.subject, UNREADABLE).await,
        OCR => crate::library::end_ocr(&state.database, job.subject).await,
        _ => return,
    };
    match result {
        Ok(Some(work)) => tell(state, work).await,
        Ok(None) => {}
        Err(error) => tracing::error!(%error, "A file did not get its state after a failed job"),
    }
}

async fn tell(state: &AppState, work: Work) {
    let organization = work.organization_id;
    let changed = ServerMessage::WorkChanged {
        work: Box::new(work),
    };
    state.hub.tell(organization, &changed).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use crate::links::create_organization;
    use crate::testing::test_database;

    #[test]
    fn a_job_waits_longer_before_each_new_attempt() {
        assert_eq!(retry_wait(1), Some(Duration::from_secs(10)));
        assert_eq!(retry_wait(2), Some(Duration::from_secs(60)));
        assert_eq!(retry_wait(3), None);
        assert_eq!(retry_wait(0), None);
    }

    async fn queued(pool: &PgPool) -> Uuid {
        let mut connection = pool.acquire().await.unwrap();
        add(&mut connection, "test", Uuid::from_u128(5))
            .await
            .unwrap()
    }

    async fn state_of(pool: &PgPool, job: Uuid) -> String {
        sqlx::query_scalar("SELECT state FROM jobs WHERE id = $1")
            .bind(job)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn one_worker_takes_a_job_until_it_is_done() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let id = queued(pool).await;

        let job = claim(pool).await.unwrap().unwrap();
        assert_eq!(
            (job.id, job.subject, job.attempts),
            (id, Uuid::from_u128(5), 1)
        );
        assert_eq!(claim(pool).await.unwrap(), None);

        finish(pool, id).await.unwrap();
        assert_eq!(state_of(pool, id).await, "done");
        assert_eq!(claim(pool).await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_failed_job_waits_and_then_runs_again() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let id = queued(pool).await;
        let job = claim(pool).await.unwrap().unwrap();

        assert!(
            !fail(pool, &job, "the database stopped", false)
                .await
                .unwrap()
        );
        assert_eq!(state_of(pool, id).await, "queued");
        assert_eq!(claim(pool).await.unwrap(), None);

        sqlx::query("UPDATE jobs SET run_after = now() WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
        assert_eq!(claim(pool).await.unwrap().unwrap().attempts, 2);
    }

    #[tokio::test]
    async fn a_job_fails_after_its_last_attempt_or_a_permanent_error() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let id = queued(pool).await;
        let mut job = claim(pool).await.unwrap().unwrap();

        job.attempts = MAX_ATTEMPTS;
        assert!(
            fail(pool, &job, "the database stopped", false)
                .await
                .unwrap()
        );
        assert_eq!(state_of(pool, id).await, "failed");

        let other = queued(pool).await;
        let job = claim(pool).await.unwrap().unwrap();
        assert!(fail(pool, &job, "the file is broken", true).await.unwrap());
        assert_eq!(state_of(pool, other).await, "failed");
    }

    #[tokio::test]
    async fn a_job_of_a_stopped_worker_runs_again_after_its_lease() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let id = queued(pool).await;
        claim(pool).await.unwrap().unwrap();

        sqlx::query("UPDATE jobs SET locked_until = now() - interval '1 second' WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();

        let again = claim(pool).await.unwrap().unwrap();
        assert_eq!((again.id, again.attempts), (id, 2));
    }

    async fn seconds_of_lease(pool: &PgPool, job: Uuid) -> Option<f64> {
        sqlx::query_scalar(
            "SELECT extract(epoch FROM locked_until - now())::float8 FROM jobs WHERE id = $1",
        )
        .bind(job)
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_running_job_extends_its_lease() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        let id = queued(pool).await;
        claim(pool).await.unwrap().unwrap();
        sqlx::query("UPDATE jobs SET locked_until = now() + interval '1 second' WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();

        extend(pool, id).await.unwrap();

        assert!(seconds_of_lease(pool, id).await.unwrap() > 590.0);
        finish(pool, id).await.unwrap();
        extend(pool, id).await.unwrap();
        assert_eq!(state_of(pool, id).await, "done");
        assert_eq!(seconds_of_lease(pool, id).await, None);
    }

    #[test]
    fn a_worker_extends_the_lease_before_it_ends() {
        assert!(HEARTBEAT * 2 < LEASE);
    }

    #[tokio::test]
    async fn a_job_for_an_object_that_is_gone_is_done() {
        let Some(database) = test_database().await else {
            return;
        };
        let pool = &database.pool;
        create_organization(pool, "Field Press").await.unwrap();
        let state = AppState::new(pool.clone(), Library::in_memory().unwrap());
        let mut connection = pool.acquire().await.unwrap();
        let id = add(&mut connection, READ_TEXT, Uuid::from_u128(77))
            .await
            .unwrap();

        assert!(run_one(&state).await.unwrap());

        assert_eq!(state_of(pool, id).await, "done");
        assert!(!run_one(&state).await.unwrap());
    }
}
