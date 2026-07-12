use super::puller::{pull_category, UsenetCategory, UsenetPullConfig};
use anyhow::{anyhow, Context, Result};
use chrono::{SecondsFormat, Utc};
use log::{error, info};
use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::task;
use tokio::time::sleep;

const JOB_COLUMNS: &str =
    "id, category, status, attempts, requested_at, started_at, finished_at, next_attempt_at, last_error";
const DLQ_COLUMNS: &str =
    "id, category, job_name, reason, attempts, first_seen_at, last_failed_at, local_path, remote_path, resolved_at, resolution";
const WORKER_IDLE_DELAY: Duration = Duration::from_secs(5);
const WORKER_ERROR_DELAY: Duration = Duration::from_secs(15);
const RETRY_DELAY: chrono::Duration = chrono::Duration::minutes(5);
const MAX_ATTEMPTS: i64 = 5;

#[derive(Clone, Debug)]
pub struct UsenetQueue {
    db_path: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueueJob {
    pub id: i64,
    pub category: UsenetCategory,
    pub status: JobStatus,
    pub attempts: i64,
    pub requested_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub next_attempt_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EnqueueOutcome {
    pub job: QueueJob,
    pub coalesced: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueueSnapshot {
    pub db_path: PathBuf,
    pub queued: i64,
    pub running: i64,
    pub succeeded: i64,
    pub failed: i64,
    pub recent_jobs: Vec<QueueJob>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeadLetterJob {
    pub id: i64,
    pub category: UsenetCategory,
    pub job_name: String,
    pub reason: String,
    pub attempts: i64,
    pub first_seen_at: String,
    pub last_failed_at: String,
    pub local_path: Option<PathBuf>,
    pub remote_path: Option<String>,
    pub resolved_at: Option<String>,
    pub resolution: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeadLetterSnapshot {
    pub db_path: PathBuf,
    pub open: i64,
    pub recent_jobs: Vec<DeadLetterJob>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
}

impl JobStatus {
    fn as_db_value(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    fn from_db_value(value: &str) -> Result<Self> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(anyhow!("unknown usenet queue job status: {value}")),
        }
    }
}

impl UsenetQueue {
    pub async fn open(db_path: PathBuf) -> Result<Self> {
        let queue = Self { db_path };
        queue.initialize().await?;
        queue.requeue_interrupted_jobs().await?;
        queue.coalesce_pending_jobs().await?;
        Ok(queue)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub async fn enqueue(&self, category: UsenetCategory) -> Result<EnqueueOutcome> {
        let db_path = self.db_path.clone();
        task::spawn_blocking(move || {
            let mut conn = open_connection(&db_path)?;
            enqueue_pending_job(&mut conn, category, &now_string())
        })
        .await
        .context("joining sqlite enqueue task")?
    }

    pub async fn claim_next(&self) -> Result<Option<QueueJob>> {
        let db_path = self.db_path.clone();
        task::spawn_blocking(move || {
            let mut conn = open_connection(&db_path)?;
            let now = now_string();
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .context("starting usenet queue claim transaction")?;

            let job_id = tx
                .query_row(
                    &format!(
                        "SELECT candidate.id FROM usenet_pull_jobs AS candidate
                         WHERE candidate.status = ?1
                           AND (candidate.next_attempt_at IS NULL OR candidate.next_attempt_at <= ?2)
                           AND NOT EXISTS (
                               SELECT 1
                               FROM usenet_pull_jobs AS active
                               WHERE active.status = ?3
                                 AND active.category = candidate.category
                           )
                         ORDER BY candidate.id ASC
                         LIMIT 1"
                    ),
                    params![
                        JobStatus::Queued.as_db_value(),
                        now,
                        JobStatus::Running.as_db_value()
                    ],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .context("selecting next usenet queue job")?;

            let Some(job_id) = job_id else {
                tx.commit()
                    .context("committing empty usenet queue claim transaction")?;
                return Ok(None);
            };

            tx.execute(
                "UPDATE usenet_pull_jobs
                 SET status = ?1,
                     attempts = attempts + 1,
                     started_at = ?2,
                     finished_at = NULL,
                     next_attempt_at = NULL,
                     last_error = NULL
                 WHERE id = ?3",
                params![JobStatus::Running.as_db_value(), now, job_id],
            )
            .context("marking usenet queue job running")?;

            let job = fetch_job_from_tx(&tx, job_id)?;
            tx.commit()
                .context("committing usenet queue claim transaction")?;
            Ok(Some(job))
        })
        .await
        .context("joining sqlite claim task")?
    }

    pub async fn succeed(&self, job_id: i64) -> Result<()> {
        let db_path = self.db_path.clone();
        task::spawn_blocking(move || {
            let conn = open_connection(&db_path)?;
            conn.execute(
                "UPDATE usenet_pull_jobs
                 SET status = ?1,
                     finished_at = ?2,
                     next_attempt_at = NULL,
                     last_error = NULL
                 WHERE id = ?3",
                params![JobStatus::Succeeded.as_db_value(), now_string(), job_id],
            )
            .with_context(|| format!("marking usenet queue job {job_id} succeeded"))?;
            Ok(())
        })
        .await
        .context("joining sqlite success task")?
    }

    pub async fn fail_or_retry(&self, job: &QueueJob, error_message: &str) -> Result<JobStatus> {
        let db_path = self.db_path.clone();
        let job_id = job.id;
        let should_retry = job.attempts < MAX_ATTEMPTS;
        let error_message = truncate_error(error_message);

        task::spawn_blocking(move || {
            let conn = open_connection(&db_path)?;
            let now = now_string();

            if should_retry {
                let retry_at = timestamp_after(RETRY_DELAY);
                conn.execute(
                    "UPDATE usenet_pull_jobs
                     SET status = ?1,
                         started_at = NULL,
                         finished_at = ?2,
                         next_attempt_at = ?3,
                         last_error = ?4
                     WHERE id = ?5",
                    params![
                        JobStatus::Queued.as_db_value(),
                        now,
                        retry_at,
                        error_message,
                        job_id
                    ],
                )
                .with_context(|| format!("requeueing failed usenet queue job {job_id}"))?;
                Ok(JobStatus::Queued)
            } else {
                conn.execute(
                    "UPDATE usenet_pull_jobs
                     SET status = ?1,
                         finished_at = ?2,
                         next_attempt_at = NULL,
                         last_error = ?3
                     WHERE id = ?4",
                    params![JobStatus::Failed.as_db_value(), now, error_message, job_id],
                )
                .with_context(|| format!("marking usenet queue job {job_id} failed"))?;
                Ok(JobStatus::Failed)
            }
        })
        .await
        .context("joining sqlite failure task")?
    }

    pub async fn is_dead_lettered(&self, category: UsenetCategory, job_name: &str) -> Result<bool> {
        let db_path = self.db_path.clone();
        let job_name = job_name.to_string();
        task::spawn_blocking(move || {
            let conn = open_connection(&db_path)?;
            let exists = conn
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1
                        FROM usenet_dead_letter_jobs
                        WHERE category = ?1
                          AND job_name = ?2
                          AND resolved_at IS NULL
                    )",
                    params![category.as_remote_name(), job_name],
                    |row| row.get::<_, i64>(0),
                )
                .context("checking usenet dead-letter state")?;
            Ok(exists != 0)
        })
        .await
        .context("joining sqlite dead-letter check task")?
    }

    pub async fn dead_letter_release(
        &self,
        category: UsenetCategory,
        job_name: &str,
        reason: &str,
        local_path: Option<&Path>,
        remote_path: &str,
    ) -> Result<DeadLetterJob> {
        let db_path = self.db_path.clone();
        let job_name = job_name.to_string();
        let reason = truncate_error(reason);
        let local_path = local_path.map(|path| path.display().to_string());
        let remote_path = remote_path.to_string();

        task::spawn_blocking(move || {
            let conn = open_connection(&db_path)?;
            let now = now_string();
            conn.execute(
                "INSERT INTO usenet_dead_letter_jobs
                    (category, job_name, reason, attempts, first_seen_at, last_failed_at, local_path, remote_path)
                 VALUES (?1, ?2, ?3, 1, ?4, ?4, ?5, ?6)
                 ON CONFLICT(category, job_name) WHERE resolved_at IS NULL DO UPDATE SET
                    reason = excluded.reason,
                    attempts = usenet_dead_letter_jobs.attempts + 1,
                    last_failed_at = excluded.last_failed_at,
                    local_path = excluded.local_path,
                    remote_path = excluded.remote_path",
                params![
                    category.as_remote_name(),
                    job_name,
                    reason,
                    now,
                    local_path,
                    remote_path
                ],
            )
            .context("upserting usenet dead-letter job")?;

            fetch_open_dead_letter(&conn, category, &job_name)
        })
        .await
        .context("joining sqlite dead-letter upsert task")?
    }

    pub async fn dlq_snapshot(&self, limit: usize) -> Result<DeadLetterSnapshot> {
        let db_path = self.db_path.clone();
        let limit = limit.min(250) as i64;
        task::spawn_blocking(move || {
            let conn = open_connection(&db_path)?;
            let open = count_open_dead_letters(&conn)?;
            let recent_jobs = recent_dead_letters(&conn, limit)?;

            Ok(DeadLetterSnapshot {
                db_path,
                open,
                recent_jobs,
            })
        })
        .await
        .context("joining sqlite dead-letter snapshot task")?
    }

    pub async fn retry_dead_letter(
        &self,
        category: UsenetCategory,
        job_name: &str,
    ) -> Result<Option<QueueJob>> {
        let db_path = self.db_path.clone();
        let job_name = job_name.to_string();

        task::spawn_blocking(move || {
            let conn = open_connection(&db_path)?;
            let now = now_string();
            let changed = conn
                .execute(
                    "UPDATE usenet_dead_letter_jobs
                     SET resolved_at = ?1,
                         resolution = 'retry_requested'
                     WHERE category = ?2
                       AND job_name = ?3
                       AND resolved_at IS NULL",
                    params![now, category.as_remote_name(), job_name],
                )
                .context("marking usenet dead-letter job for retry")?;

            if changed == 0 {
                return Ok(None);
            }

            drop(conn);
            let mut conn = open_connection(&db_path)?;
            enqueue_pending_job(&mut conn, category, &now).map(|outcome| Some(outcome.job))
        })
        .await
        .context("joining sqlite dead-letter retry task")?
    }

    pub async fn snapshot(&self, limit: usize) -> Result<QueueSnapshot> {
        let db_path = self.db_path.clone();
        let limit = limit.min(100) as i64;
        task::spawn_blocking(move || {
            let conn = open_connection(&db_path)?;
            let queued = count_status(&conn, JobStatus::Queued)?;
            let running = count_status(&conn, JobStatus::Running)?;
            let succeeded = count_status(&conn, JobStatus::Succeeded)?;
            let failed = count_status(&conn, JobStatus::Failed)?;
            let recent_jobs = recent_jobs(&conn, limit)?;

            Ok(QueueSnapshot {
                db_path,
                queued,
                running,
                succeeded,
                failed,
                recent_jobs,
            })
        })
        .await
        .context("joining sqlite snapshot task")?
    }

    async fn initialize(&self) -> Result<()> {
        let db_path = self.db_path.clone();
        task::spawn_blocking(move || {
            if let Some(parent) = db_path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("creating usenet queue db parent {}", parent.display()))?;
            }

            let conn = open_connection(&db_path)?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS usenet_pull_jobs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    category TEXT NOT NULL CHECK (category IN ('tv', 'movies', 'audio')),
                    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
                    attempts INTEGER NOT NULL DEFAULT 0,
                    requested_at TEXT NOT NULL,
                    started_at TEXT,
                    finished_at TEXT,
                    next_attempt_at TEXT,
                    last_error TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_usenet_pull_jobs_ready
                    ON usenet_pull_jobs(status, next_attempt_at, id);
                CREATE INDEX IF NOT EXISTS idx_usenet_pull_jobs_category
                    ON usenet_pull_jobs(category, status, id);

                CREATE TABLE IF NOT EXISTS usenet_dead_letter_jobs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    category TEXT NOT NULL CHECK (category IN ('tv', 'movies', 'audio')),
                    job_name TEXT NOT NULL,
                    reason TEXT NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 1,
                    first_seen_at TEXT NOT NULL,
                    last_failed_at TEXT NOT NULL,
                    local_path TEXT,
                    remote_path TEXT,
                    resolved_at TEXT,
                    resolution TEXT
                );
                CREATE UNIQUE INDEX IF NOT EXISTS idx_usenet_dead_letter_jobs_open_unique
                    ON usenet_dead_letter_jobs(category, job_name)
                    WHERE resolved_at IS NULL;
                CREATE INDEX IF NOT EXISTS idx_usenet_dead_letter_jobs_open
                    ON usenet_dead_letter_jobs(resolved_at, last_failed_at DESC, id DESC);",
            )
            .context("initializing usenet queue schema")?;
            Ok(())
        })
        .await
        .context("joining sqlite initialization task")?
    }

    async fn requeue_interrupted_jobs(&self) -> Result<()> {
        let db_path = self.db_path.clone();
        task::spawn_blocking(move || {
            let conn = open_connection(&db_path)?;
            let changed = conn
                .execute(
                    "UPDATE usenet_pull_jobs
                     SET status = ?1,
                         started_at = NULL,
                         finished_at = ?2,
                         next_attempt_at = NULL,
                         last_error = 'requeued after rusty-hooks startup'
                     WHERE status = ?3",
                    params![
                        JobStatus::Queued.as_db_value(),
                        now_string(),
                        JobStatus::Running.as_db_value()
                    ],
                )
                .context("requeueing interrupted usenet queue jobs")?;

            if changed > 0 {
                info!("requeued {changed} interrupted usenet queue jobs after startup");
            }
            Ok(())
        })
        .await
        .context("joining sqlite interrupted-job requeue task")?
    }

    async fn coalesce_pending_jobs(&self) -> Result<()> {
        let db_path = self.db_path.clone();
        task::spawn_blocking(move || {
            let conn = open_connection(&db_path)?;
            let now = now_string();
            let changed = conn
                .execute(
                    "UPDATE usenet_pull_jobs AS duplicate
                     SET status = ?1,
                         finished_at = ?2,
                         next_attempt_at = NULL,
                         last_error = 'coalesced into earlier pending category job'
                     WHERE duplicate.status = ?3
                       AND EXISTS (
                           SELECT 1
                           FROM usenet_pull_jobs AS earlier
                           WHERE earlier.category = duplicate.category
                             AND earlier.status IN (?3, ?4)
                             AND earlier.id < duplicate.id
                       )",
                    params![
                        JobStatus::Succeeded.as_db_value(),
                        now,
                        JobStatus::Queued.as_db_value(),
                        JobStatus::Running.as_db_value()
                    ],
                )
                .context("coalescing duplicate pending usenet jobs")?;
            if changed > 0 {
                info!("coalesced {changed} duplicate pending usenet jobs");
            }
            Ok(())
        })
        .await
        .context("joining sqlite pending-job coalescing task")?
    }
}

pub async fn run_worker(worker_id: usize, queue: UsenetQueue, pull_config: UsenetPullConfig) {
    info!(
        "starting usenet queue worker worker_id={} db={}",
        worker_id,
        queue.db_path().display()
    );

    loop {
        let job = match queue.claim_next().await {
            Ok(Some(job)) => job,
            Ok(None) => {
                sleep(WORKER_IDLE_DELAY).await;
                continue;
            }
            Err(error) => {
                error!("unable to claim next usenet queue job: {error:#}");
                sleep(WORKER_ERROR_DELAY).await;
                continue;
            }
        };

        info!(
            "usenet queue job started worker_id={} id={} category={:?} attempt={}",
            worker_id, job.id, job.category, job.attempts
        );

        match pull_category(job.category, pull_config.clone(), queue.clone()).await {
            Ok(summary) => {
                if let Err(error) = queue.succeed(job.id).await {
                    error!(
                        "usenet queue job completed but could not be marked succeeded id={}: {error:#}",
                        job.id
                    );
                    continue;
                }
                info!(
                    "usenet queue job succeeded id={} category={:?} remote_jobs_seen={} pulled={} skipped_ready={} skipped_dead_letter={} skipped_empty_remote={} dead_lettered={} iso_remuxed={} remote_cleaned={}",
                    job.id,
                    summary.category,
                    summary.remote_jobs_seen,
                    summary.pulled,
                    summary.skipped_ready,
                    summary.skipped_dead_letter,
                    summary.skipped_empty_remote,
                    summary.dead_lettered,
                    summary.iso_remuxed,
                    summary.remote_cleaned
                );
            }
            Err(error) => {
                let error_message = format!("{error:#}");
                match queue.fail_or_retry(&job, &error_message).await {
                    Ok(JobStatus::Queued) => error!(
                        "usenet queue job failed id={} category={:?}; retrying later: {}",
                        job.id, job.category, error_message
                    ),
                    Ok(JobStatus::Failed) => error!(
                        "usenet queue job failed permanently id={} category={:?}: {}",
                        job.id, job.category, error_message
                    ),
                    Ok(status) => error!(
                        "usenet queue job id={} ended with unexpected status {:?}: {}",
                        job.id, status, error_message
                    ),
                    Err(queue_error) => error!(
                        "usenet queue job failed and could not be recorded id={} category={:?}: job_error={}; queue_error={:#}",
                        job.id, job.category, error_message, queue_error
                    ),
                }
            }
        }
    }
}

fn open_connection(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("opening usenet queue db {}", db_path.display()))?;
    conn.busy_timeout(Duration::from_secs(30))
        .context("setting usenet queue sqlite busy timeout")?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .context("setting usenet queue sqlite WAL mode")?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .context("enabling usenet queue sqlite foreign keys")?;
    Ok(conn)
}

fn enqueue_pending_job(
    conn: &mut Connection,
    category: UsenetCategory,
    requested_at: &str,
) -> Result<EnqueueOutcome> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("starting usenet enqueue transaction")?;

    let existing_id = tx
        .query_row(
            "SELECT id
             FROM usenet_pull_jobs
             WHERE category = ?1
               AND status IN (?2, ?3)
             ORDER BY CASE status WHEN 'running' THEN 0 ELSE 1 END, id
             LIMIT 1",
            params![
                category.as_remote_name(),
                JobStatus::Queued.as_db_value(),
                JobStatus::Running.as_db_value()
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .context("checking for pending usenet category job")?;

    if let Some(job_id) = existing_id {
        tx.execute(
            "UPDATE usenet_pull_jobs
             SET next_attempt_at = NULL
             WHERE id = ?1 AND status = ?2",
            params![job_id, JobStatus::Queued.as_db_value()],
        )
        .context("making coalesced usenet queue job immediately eligible")?;
        let job = fetch_job_from_tx(&tx, job_id)?;
        tx.commit()
            .context("committing coalesced usenet enqueue transaction")?;
        return Ok(EnqueueOutcome {
            job,
            coalesced: true,
        });
    }

    tx.execute(
        "INSERT INTO usenet_pull_jobs
            (category, status, attempts, requested_at)
         VALUES (?1, ?2, 0, ?3)",
        params![
            category.as_remote_name(),
            JobStatus::Queued.as_db_value(),
            requested_at
        ],
    )
    .context("inserting usenet queue job")?;

    let job_id = tx.last_insert_rowid();
    let job = fetch_job_from_tx(&tx, job_id)?;
    tx.commit()
        .context("committing usenet enqueue transaction")?;
    Ok(EnqueueOutcome {
        job,
        coalesced: false,
    })
}

fn fetch_job_from_tx(tx: &rusqlite::Transaction<'_>, job_id: i64) -> Result<QueueJob> {
    tx.query_row(
        &format!("SELECT {JOB_COLUMNS} FROM usenet_pull_jobs WHERE id = ?1"),
        params![job_id],
        row_to_job,
    )
    .with_context(|| format!("fetching claimed usenet queue job {job_id}"))
}

fn recent_jobs(conn: &Connection, limit: i64) -> Result<Vec<QueueJob>> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT {JOB_COLUMNS}
             FROM usenet_pull_jobs
             ORDER BY id DESC
             LIMIT ?1"
        ))
        .context("preparing usenet queue recent jobs query")?;

    let rows = statement
        .query_map(params![limit], row_to_job)
        .context("querying usenet queue recent jobs")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("reading usenet queue recent jobs")
}

fn fetch_open_dead_letter(
    conn: &Connection,
    category: UsenetCategory,
    job_name: &str,
) -> Result<DeadLetterJob> {
    conn.query_row(
        &format!(
            "SELECT {DLQ_COLUMNS}
             FROM usenet_dead_letter_jobs
             WHERE category = ?1
               AND job_name = ?2
               AND resolved_at IS NULL"
        ),
        params![category.as_remote_name(), job_name],
        row_to_dead_letter,
    )
    .with_context(|| format!("fetching usenet dead-letter job {category:?}/{job_name}"))
}

fn recent_dead_letters(conn: &Connection, limit: i64) -> Result<Vec<DeadLetterJob>> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT {DLQ_COLUMNS}
             FROM usenet_dead_letter_jobs
             WHERE resolved_at IS NULL
             ORDER BY last_failed_at DESC, id DESC
             LIMIT ?1"
        ))
        .context("preparing usenet dead-letter recent jobs query")?;

    let rows = statement
        .query_map(params![limit], row_to_dead_letter)
        .context("querying usenet dead-letter recent jobs")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("reading usenet dead-letter recent jobs")
}

fn count_open_dead_letters(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM usenet_dead_letter_jobs WHERE resolved_at IS NULL",
        [],
        |row| row.get(0),
    )
    .context("counting open usenet dead-letter jobs")
}

fn count_status(conn: &Connection, status: JobStatus) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM usenet_pull_jobs WHERE status = ?1",
        params![status.as_db_value()],
        |row| row.get(0),
    )
    .with_context(|| format!("counting usenet queue jobs status={:?}", status))
}

fn row_to_dead_letter(row: &Row<'_>) -> rusqlite::Result<DeadLetterJob> {
    let category: String = row.get(1)?;
    let local_path: Option<String> = row.get(7)?;

    Ok(DeadLetterJob {
        id: row.get(0)?,
        category: category_from_db(&category)
            .map_err(|error| conversion_error(1, category, error))?,
        job_name: row.get(2)?,
        reason: row.get(3)?,
        attempts: row.get(4)?,
        first_seen_at: row.get(5)?,
        last_failed_at: row.get(6)?,
        local_path: local_path.map(PathBuf::from),
        remote_path: row.get(8)?,
        resolved_at: row.get(9)?,
        resolution: row.get(10)?,
    })
}

fn row_to_job(row: &Row<'_>) -> rusqlite::Result<QueueJob> {
    let category: String = row.get(1)?;
    let status: String = row.get(2)?;

    Ok(QueueJob {
        id: row.get(0)?,
        category: category_from_db(&category)
            .map_err(|error| conversion_error(1, category, error))?,
        status: JobStatus::from_db_value(&status)
            .map_err(|error| conversion_error(2, status, error))?,
        attempts: row.get(3)?,
        requested_at: row.get(4)?,
        started_at: row.get(5)?,
        finished_at: row.get(6)?,
        next_attempt_at: row.get(7)?,
        last_error: row.get(8)?,
    })
}

fn category_from_db(value: &str) -> Result<UsenetCategory> {
    match value {
        "tv" => Ok(UsenetCategory::Tv),
        "movies" => Ok(UsenetCategory::Movies),
        "audio" => Ok(UsenetCategory::Audio),
        _ => Err(anyhow!("unknown usenet queue category: {value}")),
    }
}

fn conversion_error(index: usize, value: String, error: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        Type::Text,
        anyhow!("{error}; value={value}").into(),
    )
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn timestamp_after(duration: chrono::Duration) -> String {
    Utc::now()
        .checked_add_signed(duration)
        .unwrap_or_else(Utc::now)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn truncate_error(error_message: &str) -> String {
    const MAX_ERROR_LEN: usize = 8_000;
    if error_message.len() <= MAX_ERROR_LEN {
        return error_message.to_string();
    }

    let truncated = error_message
        .char_indices()
        .take_while(|(index, _)| *index < MAX_ERROR_LEN)
        .map(|(_, value)| value)
        .collect::<String>();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;

    fn test_db(name: &str) -> (TempDir, PathBuf) {
        let directory = TempDir::new(name).expect("create temp directory");
        let path = directory.path().join("queue.sqlite3");
        (directory, path)
    }

    #[tokio::test]
    async fn simultaneous_webhooks_coalesce_to_one_pending_category_job() {
        let (_directory, path) = test_db("usenet-coalesce");
        let queue = UsenetQueue::open(path).await.expect("open queue");

        let (first, second) = tokio::join!(
            queue.enqueue(UsenetCategory::Audio),
            queue.enqueue(UsenetCategory::Audio)
        );
        let first = first.expect("first enqueue");
        let second = second.expect("second enqueue");

        assert_eq!(first.job.id, second.job.id);
        assert_ne!(first.coalesced, second.coalesced);
        assert_eq!(queue.snapshot(10).await.expect("snapshot").queued, 1);
    }

    #[tokio::test]
    async fn claims_different_categories_in_parallel_but_blocks_same_category() {
        let (_directory, path) = test_db("usenet-category-lock");
        let queue = UsenetQueue::open(path.clone()).await.expect("open queue");

        queue
            .enqueue(UsenetCategory::Movies)
            .await
            .expect("enqueue movies");
        let movies = queue
            .claim_next()
            .await
            .expect("claim movies")
            .expect("movies job");
        assert_eq!(movies.category, UsenetCategory::Movies);

        let conn = open_connection(&path).expect("open test connection");
        conn.execute(
            "INSERT INTO usenet_pull_jobs
                (category, status, attempts, requested_at)
             VALUES (?1, ?2, 0, ?3)",
            params![
                UsenetCategory::Movies.as_remote_name(),
                JobStatus::Queued.as_db_value(),
                now_string()
            ],
        )
        .expect("insert second movie job");
        drop(conn);

        queue
            .enqueue(UsenetCategory::Audio)
            .await
            .expect("enqueue audio");
        let audio = queue
            .claim_next()
            .await
            .expect("claim audio")
            .expect("audio job");
        assert_eq!(audio.category, UsenetCategory::Audio);
        assert!(queue.claim_next().await.expect("third claim").is_none());
    }

    #[tokio::test]
    async fn startup_coalesces_legacy_duplicate_jobs() {
        let (_directory, path) = test_db("usenet-startup-coalesce");
        let queue = UsenetQueue::open(path.clone()).await.expect("open queue");
        queue
            .enqueue(UsenetCategory::Audio)
            .await
            .expect("enqueue audio");

        let conn = open_connection(&path).expect("open test connection");
        conn.execute(
            "INSERT INTO usenet_pull_jobs
                (category, status, attempts, requested_at)
             VALUES (?1, ?2, 0, ?3)",
            params![
                UsenetCategory::Audio.as_remote_name(),
                JobStatus::Queued.as_db_value(),
                now_string()
            ],
        )
        .expect("insert duplicate audio job");
        drop(conn);
        drop(queue);

        let reopened = UsenetQueue::open(path).await.expect("reopen queue");
        let snapshot = reopened.snapshot(10).await.expect("snapshot");
        assert_eq!(snapshot.queued, 1);
        assert_eq!(snapshot.succeeded, 1);
    }
}
