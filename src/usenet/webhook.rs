use super::puller::{UsenetCategory, UsenetPullConfig};
use super::queue::{run_worker, UsenetQueue};
use axum::extract::{rejection::JsonRejection, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use log::{error, info};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone)]
pub struct WebhookState {
    queue: UsenetQueue,
}

impl WebhookState {
    pub fn new(queue: UsenetQueue) -> Self {
        Self { queue }
    }
}

#[derive(Debug, Deserialize)]
struct ReadyRequest {
    category: UsenetCategory,
}

#[derive(Debug, Deserialize)]
struct RetryDeadLetterRequest {
    category: UsenetCategory,
    job_name: String,
}

pub async fn start(
    bind_address: SocketAddr,
    pull_config: UsenetPullConfig,
    queue_db_path: PathBuf,
    worker_count: usize,
) -> anyhow::Result<()> {
    let queue = UsenetQueue::open(queue_db_path).await?;
    let state = WebhookState::new(queue.clone());
    let worker_count = worker_count.clamp(1, 3);
    for worker_id in 1..=worker_count {
        tokio::spawn(run_worker(worker_id, queue.clone(), pull_config.clone()));
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/usenet/dlq", get(dlq_status))
        .route("/usenet/dlq/retry", post(retry_dlq))
        .route("/usenet/queue", get(queue_status))
        .route("/usenet/ready", post(ready))
        .route("/sabnzbd/ready", post(ready))
        .with_state(state);

    info!("starting usenet webhook server on {}", bind_address);
    axum::Server::bind(&bind_address)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(json!({"ok": true}))
}

async fn queue_status(State(state): State<WebhookState>) -> impl IntoResponse {
    match state.queue.snapshot(25).await {
        Ok(snapshot) => (StatusCode::OK, Json(json!(snapshot))),
        Err(error) => {
            error!("failed to read usenet queue status: {error:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "queue_status_failed",
                    "details": error.to_string(),
                })),
            )
        }
    }
}

async fn dlq_status(State(state): State<WebhookState>) -> impl IntoResponse {
    match state.queue.dlq_snapshot(100).await {
        Ok(snapshot) => (StatusCode::OK, Json(json!(snapshot))),
        Err(error) => {
            error!("failed to read usenet DLQ status: {error:#}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "dlq_status_failed",
                    "details": error.to_string(),
                })),
            )
        }
    }
}

async fn retry_dlq(
    State(state): State<WebhookState>,
    payload: Result<Json<RetryDeadLetterRequest>, JsonRejection>,
) -> impl IntoResponse {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid_json_or_category",
                    "details": error.to_string(),
                    "allowed": ["tv", "movies", "audio"],
                })),
            );
        }
    };

    let category = request.category;
    let job_name = request.job_name;
    match state.queue.retry_dead_letter(category, &job_name).await {
        Ok(Some(job)) => {
            info!(
                "retry requested for usenet DLQ job category={:?} job_name={} queued_job_id={}",
                category, job_name, job.id
            );
            (
                StatusCode::ACCEPTED,
                Json(json!({
                    "status": "retry_queued",
                    "job_id": job.id,
                    "category": job.category,
                    "job_name": job_name,
                    "requested_at": job.requested_at,
                })),
            )
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "dlq_entry_not_found",
                "category": category,
                "job_name": job_name,
            })),
        ),
        Err(error) => {
            error!(
                "failed to retry usenet DLQ job category={category:?} job_name={job_name}: {error:#}"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "dlq_retry_failed",
                    "category": category,
                    "job_name": job_name,
                    "details": error.to_string(),
                })),
            )
        }
    }
}

async fn ready(
    State(state): State<WebhookState>,
    payload: Result<Json<ReadyRequest>, JsonRejection>,
) -> impl IntoResponse {
    let Json(request) = match payload {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid_json_or_category",
                    "details": error.to_string(),
                    "allowed": ["tv", "movies", "audio"],
                })),
            );
        }
    };

    let category = request.category;
    let enqueue = match state.queue.enqueue(category).await {
        Ok(enqueue) => enqueue,
        Err(error) => {
            error!("failed to enqueue usenet pull request category={category:?}: {error:#}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "enqueue_failed",
                    "category": category,
                    "details": error.to_string(),
                })),
            );
        }
    };

    let job = enqueue.job;
    let status = if enqueue.coalesced {
        "already_pending"
    } else {
        "queued"
    };
    info!(
        "usenet pull request status={} id={} category={:?}",
        status, job.id, job.category
    );

    (
        StatusCode::ACCEPTED,
        Json(json!({
            "status": status,
            "job_id": job.id,
            "category": job.category,
            "requested_at": job.requested_at,
        })),
    )
}
