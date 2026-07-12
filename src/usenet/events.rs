use super::queue::DeadLetterJob;
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use log::warn;
use reqwest::Url;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

const DLQ_EVENT_TYPE: &str = "usenet.dlq.upserted";
const EVENT_VERSION: u8 = 1;

#[derive(Clone, Debug)]
pub struct UsenetEventSink {
    jsonl_path: Option<PathBuf>,
    webhook_url: Option<Url>,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct UsenetDlqEvent<'a> {
    event_type: &'static str,
    event_version: u8,
    occurred_at: String,
    dead_letter: &'a DeadLetterJob,
}

impl UsenetEventSink {
    pub fn new(jsonl_path: Option<PathBuf>, webhook_url: Option<String>) -> Result<Option<Self>> {
        if jsonl_path.is_none() && webhook_url.is_none() {
            return Ok(None);
        }

        let webhook_url = webhook_url
            .map(|url| {
                Url::parse(&url).with_context(|| format!("parsing usenet event webhook URL {url}"))
            })
            .transpose()?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("building usenet event HTTP client")?;

        Ok(Some(Self {
            jsonl_path,
            webhook_url,
            http,
        }))
    }

    pub async fn emit_dead_letter(&self, dead_letter: &DeadLetterJob) -> Result<()> {
        let event = UsenetDlqEvent {
            event_type: DLQ_EVENT_TYPE,
            event_version: EVENT_VERSION,
            occurred_at: now_string(),
            dead_letter,
        };

        if let Some(jsonl_path) = &self.jsonl_path {
            append_jsonl(jsonl_path, &event).await?;
        }

        if let Some(webhook_url) = &self.webhook_url {
            let event_body =
                serde_json::to_value(&event).context("serializing usenet DLQ event")?;
            let webhook_url = webhook_url.clone();
            let http = self.http.clone();
            tokio::spawn(async move {
                if let Err(error) = post_webhook(http, webhook_url, event_body).await {
                    warn!("failed to POST usenet DLQ event webhook: {error:#}");
                }
            });
        }

        Ok(())
    }
}

async fn append_jsonl(path: &PathBuf, event: &UsenetDlqEvent<'_>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("creating usenet event log parent {}", parent.display()))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .with_context(|| format!("opening usenet event log {}", path.display()))?;
    let mut line = serde_json::to_vec(event).context("serializing usenet DLQ event")?;
    line.push(b'\n');
    file.write_all(&line)
        .await
        .with_context(|| format!("writing usenet event log {}", path.display()))?;
    file.flush()
        .await
        .with_context(|| format!("flushing usenet event log {}", path.display()))?;
    Ok(())
}

async fn post_webhook(
    http: reqwest::Client,
    webhook_url: Url,
    event_body: serde_json::Value,
) -> Result<()> {
    let response = http
        .post(webhook_url.clone())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(event_body.to_string())
        .send()
        .await
        .with_context(|| format!("POSTing usenet event webhook {webhook_url}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("usenet event webhook {webhook_url} returned {status}: {body}");
    }
    Ok(())
}

fn now_string() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::super::puller::UsenetCategory;
    use super::*;
    use tempdir::TempDir;

    #[tokio::test]
    async fn appends_dead_letter_event_jsonl() {
        let temp = TempDir::new("rusty-hooks-usenet-events").expect("tempdir");
        let jsonl_path = temp.path().join("events.jsonl");
        let sink = UsenetEventSink::new(Some(jsonl_path.clone()), None)
            .expect("sink")
            .expect("configured sink");
        let dead_letter = DeadLetterJob {
            id: 42,
            category: UsenetCategory::Movies,
            job_name: "Example.Release".to_string(),
            reason: "unsupported media".to_string(),
            attempts: 1,
            first_seen_at: "2026-05-26T00:00:00Z".to_string(),
            last_failed_at: "2026-05-26T00:00:00Z".to_string(),
            local_path: Some(PathBuf::from("/tmp/quarantine/Example.Release")),
            remote_path: Some("movies/Example.Release".to_string()),
            resolved_at: None,
            resolution: None,
        };

        sink.emit_dead_letter(&dead_letter)
            .await
            .expect("emit event");

        let contents = fs::read_to_string(jsonl_path).await.expect("read jsonl");
        let value: serde_json::Value =
            serde_json::from_str(contents.trim()).expect("valid event json");
        assert_eq!(value["event_type"], DLQ_EVENT_TYPE);
        assert_eq!(value["event_version"], EVENT_VERSION);
        assert_eq!(value["dead_letter"]["id"], 42);
        assert_eq!(value["dead_letter"]["category"], "movies");
    }
}
