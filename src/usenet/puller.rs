use anyhow::{anyhow, bail, Context, Result};
use async_process::Command;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::time::sleep;

use super::events::UsenetEventSink;
use super::media_prepare;
use super::queue::UsenetQueue;

const READY_FILE: &str = ".rusty-ready";
const REMOTE_RSYNC_BUSY: &str = "Another instance of rrsync is already accessing this directory";
const REMOTE_RSYNC_ATTEMPTS: usize = 120;
const REMOTE_RSYNC_RETRY_DELAY: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UsenetCategory {
    Tv,
    Movies,
    Audio,
}

impl UsenetCategory {
    pub fn as_remote_name(&self) -> &'static str {
        match self {
            Self::Tv => "tv",
            Self::Movies => "movies",
            Self::Audio => "audio",
        }
    }

    pub fn local_drop_root(&self) -> PathBuf {
        match self {
            Self::Tv => PathBuf::from("/media/wd_red/wd_red_sync_folder/longterm-companion-tv"),
            Self::Movies => {
                PathBuf::from("/media/wd_red/wd_red_sync_folder/longterm-companion-movies")
            }
            Self::Audio => {
                PathBuf::from("/media/wd_red/wd_red_sync_folder/longterm-companion-music")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct UsenetPullConfig {
    pub remote_user: String,
    pub remote_host: String,
    pub ssh_key_path: PathBuf,
    pub remote_complete_root: PathBuf,
    pub staging_root: PathBuf,
    pub remote_cleanup_enabled: bool,
    pub event_sink: Option<UsenetEventSink>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PullSummary {
    pub category: UsenetCategory,
    pub remote_jobs_seen: usize,
    pub pulled: usize,
    pub skipped_ready: usize,
    pub skipped_dead_letter: usize,
    pub iso_remuxed: usize,
    pub skipped_empty_remote: usize,
    pub dead_lettered: usize,
    pub remote_cleaned: usize,
}

pub async fn pull_category(
    category: UsenetCategory,
    config: UsenetPullConfig,
    queue: UsenetQueue,
) -> Result<PullSummary> {
    if !config.ssh_key_path.exists() {
        bail!(
            "download gateway SSH key does not exist: {}",
            config.ssh_key_path.display()
        );
    }

    let destination_root = category.local_drop_root();
    let staging_category_root = config.staging_root.join(category.as_remote_name());

    fs::create_dir_all(&destination_root)
        .await
        .with_context(|| format!("creating destination root {}", destination_root.display()))?;
    fs::create_dir_all(&staging_category_root)
        .await
        .with_context(|| format!("creating staging root {}", staging_category_root.display()))?;

    let remote_jobs = list_remote_jobs(category, &config).await?;
    info!(
        "usenet pull found {} remote jobs for category={:?}",
        remote_jobs.len(),
        category
    );

    let mut pulled = 0;
    let mut skipped_ready = 0;
    let mut skipped_dead_letter = 0;
    let mut iso_remuxed = 0;
    let mut skipped_empty_remote = 0;
    let mut dead_lettered = 0;
    let mut remote_cleaned = 0;

    for job in &remote_jobs {
        validate_job_name(job)?;
        let destination = destination_root.join(job);

        if queue.is_dead_lettered(category, job).await? {
            skipped_dead_letter += 1;
            info!(
                "usenet pull skipping dead-lettered job category={:?} job={}",
                category, job
            );
            continue;
        }

        if !remote_job_has_files(category, job, &config).await? {
            skipped_empty_remote += 1;
            info!(
                "usenet pull skipping empty remote job category={:?} job={}",
                category, job
            );
            continue;
        }

        if destination.join(READY_FILE).exists() {
            skipped_ready += 1;
            if config.remote_cleanup_enabled && try_cleanup_remote_job(category, job, &config).await
            {
                remote_cleaned += 1;
            }
            info!(
                "usenet pull skipping already-ready job category={:?} job={} remote_cleaned={}",
                category, job, config.remote_cleanup_enabled
            );
            continue;
        }

        let staging = staging_category_root.join(format!("{job}.partial"));
        fs::create_dir_all(&staging)
            .await
            .with_context(|| format!("creating staging directory {}", staging.display()))?;

        rsync_job(category, job, &staging, &config).await?;
        set_media_permissions(&staging).await;

        if destination.exists() {
            warn!(
                "usenet pull destination exists without ready marker; merging category={:?} job={} destination={}",
                category,
                job,
                destination.display()
            );
            rsync_local_merge(&staging, &destination).await?;
            fs::remove_dir_all(&staging)
                .await
                .with_context(|| format!("removing staging directory {}", staging.display()))?;
        } else {
            fs::rename(&staging, &destination).await.with_context(|| {
                format!(
                    "moving staging directory {} to {}",
                    staging.display(),
                    destination.display()
                )
            })?;
        }

        set_media_permissions(&destination).await;
        let media_summary = match media_prepare::prepare_for_filebot(
            category,
            job,
            &destination,
            &config,
        )
        .await
        {
            Ok(summary) => summary,
            Err(error) => {
                let reason = format!("{error:#}");
                let quarantine_path =
                        quarantine_failed_job(category, job, &destination, &staging, &config)
                            .await
                            .map(Some)
                            .unwrap_or_else(|quarantine_error| {
                                warn!(
                                    "usenet pull failed to quarantine dead-lettered job category={:?} job={}: {quarantine_error:#}",
                                    category, job
                                );
                                None
                            });
                let remote_path = format!("{}/{}", category.as_remote_name(), job);
                let dead_letter = queue
                    .dead_letter_release(
                        category,
                        job,
                        &reason,
                        quarantine_path.as_deref().or(Some(&destination)),
                        &remote_path,
                    )
                    .await?;
                if let Some(event_sink) = &config.event_sink {
                    if let Err(event_error) = event_sink.emit_dead_letter(&dead_letter).await {
                        warn!(
                            "usenet pull failed to emit DLQ event category={:?} job={} dlq_id={}: {event_error:#}",
                            category, job, dead_letter.id
                        );
                    }
                }
                let remote_cleaned_for_job = config.remote_cleanup_enabled
                    && quarantine_path.is_some()
                    && try_cleanup_remote_job(category, job, &config).await;
                if remote_cleaned_for_job {
                    remote_cleaned += 1;
                }
                dead_lettered += 1;
                warn!(
                        "usenet pull dead-lettered job category={:?} job={} dlq_id={} attempts={} remote_cleaned={} reason={}",
                        category,
                        job,
                        dead_letter.id,
                        dead_letter.attempts,
                        remote_cleaned_for_job,
                        reason
                    );
                continue;
            }
        };
        iso_remuxed += media_summary.iso_remuxed;
        set_media_permissions(&destination).await;
        let remote_cleaned_for_job =
            config.remote_cleanup_enabled && try_cleanup_remote_job(category, job, &config).await;
        if remote_cleaned_for_job {
            remote_cleaned += 1;
        }
        create_ready_marker(&destination).await?;
        pulled += 1;
        info!(
            "usenet pull completed category={:?} job={} destination={} iso_remuxed={} remote_cleaned={}",
            category,
            job,
            destination.display(),
            media_summary.iso_remuxed,
            remote_cleaned_for_job
        );
    }

    Ok(PullSummary {
        category,
        remote_jobs_seen: remote_jobs.len(),
        pulled,
        skipped_ready,
        skipped_dead_letter,
        iso_remuxed,
        skipped_empty_remote,
        dead_lettered,
        remote_cleaned,
    })
}

async fn list_remote_jobs(
    category: UsenetCategory,
    config: &UsenetPullConfig,
) -> Result<Vec<String>> {
    let remote_source = remote_source(config, &format!("{}/", category.as_remote_name()));

    debug!("usenet pull remote list source: {}", remote_source);
    let output = run_remote_rsync_with_retry(
        Command::new("rsync")
            .arg("--list-only")
            .arg("-e")
            .arg(ssh_transport(config))
            .arg(remote_source),
        "listing remote usenet job directories",
    )
    .await?;

    let stdout = String::from_utf8(output.stdout).context("decoding remote rsync listing")?;
    let jobs = stdout
        .lines()
        .filter_map(parse_top_level_rsync_directory)
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(jobs)
}

async fn remote_job_has_files(
    category: UsenetCategory,
    job: &str,
    config: &UsenetPullConfig,
) -> Result<bool> {
    let remote_path = format!("{}/{}/", category.as_remote_name(), job);
    let remote_source = remote_source(config, &remote_path);

    let output = run_remote_rsync_with_retry(
        Command::new("rsync")
            .arg("--list-only")
            .arg("-r")
            .arg("-e")
            .arg(ssh_transport(config))
            .arg(remote_source),
        &format!("checking remote job contents {job}"),
    )
    .await?;

    let stdout = String::from_utf8(output.stdout).context("decoding remote rsync job listing")?;
    Ok(rsync_listing_has_files(&stdout))
}

fn rsync_listing_has_files(output: &str) -> bool {
    output.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with('-') || line.starts_with('l')
    })
}

async fn rsync_job(
    category: UsenetCategory,
    job: &str,
    staging: &Path,
    config: &UsenetPullConfig,
) -> Result<()> {
    let remote_path = format!("{}/{}/", category.as_remote_name(), job);
    let remote_source = remote_source(config, &remote_path);

    let ssh_transport = ssh_transport(config);
    let output = run_remote_rsync_with_retry(
        Command::new("rsync")
            .arg("-a")
            .arg("--partial")
            .arg("--partial-dir=.rsync-partial")
            .arg("--info=stats2")
            .arg("--chmod=Du=rwx,Dg=rwx,Do=rx,Fu=rw,Fg=rw,Fo=r")
            .arg("-e")
            .arg(ssh_transport)
            .arg(remote_source)
            .arg(path_with_trailing_slash(staging)),
        &format!("rsyncing remote job {job}"),
    )
    .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        info!("{}", stdout.trim_end());
    }

    Ok(())
}

async fn cleanup_remote_job(
    category: UsenetCategory,
    job: &str,
    config: &UsenetPullConfig,
) -> Result<()> {
    let empty_source = config.staging_root.join(".remote-cleanup-empty");
    fs::create_dir_all(&empty_source)
        .await
        .with_context(|| format!("creating remote cleanup source {}", empty_source.display()))?;

    let remote_path = format!("{}/{}/", category.as_remote_name(), job);
    let remote_destination = remote_source(config, &remote_path);
    let output = run_remote_rsync_with_retry(
        Command::new("rsync")
            .arg("-a")
            .arg("--delete")
            .arg("--force")
            .arg("--info=stats2,del")
            .arg("-e")
            .arg(ssh_transport(config))
            .arg(path_with_trailing_slash(&empty_source))
            .arg(remote_destination),
        &format!("cleaning remote job {job}"),
    )
    .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.trim().is_empty() {
        info!("{}", stdout.trim_end());
    }

    if remote_job_has_files(category, job, config).await? {
        bail!("remote cleanup left files behind for category={category:?} job={job}");
    }

    info!(
        "usenet pull cleaned remote job contents category={:?} job={}",
        category, job
    );
    Ok(())
}

async fn try_cleanup_remote_job(
    category: UsenetCategory,
    job: &str,
    config: &UsenetPullConfig,
) -> bool {
    match cleanup_remote_job(category, job, config).await {
        Ok(_) => true,
        Err(error) => {
            warn!(
                "usenet remote cleanup failed; continuing with local import category={:?} job={}: {error:#}",
                category, job
            );
            false
        }
    }
}

async fn quarantine_failed_job(
    category: UsenetCategory,
    job: &str,
    destination: &Path,
    staging: &Path,
    config: &UsenetPullConfig,
) -> Result<PathBuf> {
    let quarantine_base = config
        .staging_root
        .join("unsupported-originals")
        .join(category.as_remote_name());
    fs::create_dir_all(&quarantine_base)
        .await
        .with_context(|| format!("creating quarantine root {}", quarantine_base.display()))?;

    let quarantine = unique_path(quarantine_base.join(job));
    if destination.exists() {
        fs::rename(destination, &quarantine)
            .await
            .with_context(|| {
                format!(
                    "moving failed destination {} to quarantine {}",
                    destination.display(),
                    quarantine.display()
                )
            })?;
        if staging.exists() {
            fs::remove_dir_all(staging)
                .await
                .with_context(|| format!("removing duplicate staging {}", staging.display()))?;
        }
        return Ok(quarantine);
    }

    if staging.exists() {
        fs::rename(staging, &quarantine).await.with_context(|| {
            format!(
                "moving failed staging {} to quarantine {}",
                staging.display(),
                quarantine.display()
            )
        })?;
        return Ok(quarantine);
    }

    fs::create_dir_all(&quarantine)
        .await
        .with_context(|| format!("creating empty failure quarantine {}", quarantine.display()))?;
    Ok(quarantine)
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("dead-letter");

    for index in 1.. {
        let candidate = parent.join(format!("{name}.{index}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded loop returns a unique path")
}

async fn run_remote_rsync_with_retry(
    command: &mut Command,
    description: &str,
) -> Result<async_process::Output> {
    for attempt in 1..=REMOTE_RSYNC_ATTEMPTS {
        let output = command
            .output()
            .await
            .with_context(|| format!("{description}: spawning rsync"))?;

        if output.status.success() {
            return Ok(output);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains(REMOTE_RSYNC_BUSY) && attempt < REMOTE_RSYNC_ATTEMPTS {
            warn!(
                "{description}: remote rrsync is busy, retrying in {}s (attempt {}/{})",
                REMOTE_RSYNC_RETRY_DELAY.as_secs(),
                attempt,
                REMOTE_RSYNC_ATTEMPTS
            );
            sleep(REMOTE_RSYNC_RETRY_DELAY).await;
            continue;
        }

        bail!("{description} failed: {}", stderr);
    }

    unreachable!("retry loop always returns or bails")
}

fn parse_top_level_rsync_directory(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if !line.starts_with('d') {
        return None;
    }

    let name = nth_whitespace_separated_tail(line, 4)?.trim_end();
    if name == "." || name.contains('/') {
        return None;
    }

    Some(name)
}

fn nth_whitespace_separated_tail(line: &str, fields_to_skip: usize) -> Option<&str> {
    let mut index = 0;
    let bytes = line.as_bytes();

    for _ in 0..fields_to_skip {
        while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
    }

    (index < bytes.len()).then_some(&line[index..])
}

async fn rsync_local_merge(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)
        .await
        .with_context(|| format!("creating merge destination {}", destination.display()))?;

    let status = Command::new("rsync")
        .arg("-a")
        .arg("--delete")
        .arg(path_with_trailing_slash(source))
        .arg(path_with_trailing_slash(destination))
        .status()
        .await
        .context("merging staged job into existing destination")?;

    if !status.success() {
        bail!("local staging merge failed with status {status}");
    }

    Ok(())
}

async fn create_ready_marker(destination: &Path) -> Result<()> {
    let tmp_marker = destination.join(format!("{READY_FILE}.tmp"));
    let marker = destination.join(READY_FILE);
    fs::write(&tmp_marker, b"ready\n")
        .await
        .with_context(|| format!("writing ready marker {}", tmp_marker.display()))?;
    fs::rename(&tmp_marker, &marker)
        .await
        .with_context(|| format!("renaming ready marker {}", marker.display()))?;
    Ok(())
}

async fn set_media_permissions(path: &Path) {
    let chgrp = Command::new("chgrp")
        .arg("-R")
        .arg("media_mgmt")
        .arg(path)
        .status()
        .await;
    if let Err(error) = chgrp {
        warn!("unable to chgrp {}: {}", path.display(), error);
    }

    let chmod = Command::new("chmod")
        .arg("-R")
        .arg("g+rwX")
        .arg(path)
        .status()
        .await;
    if let Err(error) = chmod {
        warn!("unable to chmod {}: {}", path.display(), error);
    }
}

fn ssh_transport(config: &UsenetPullConfig) -> String {
    let args = ssh_args(config)
        .into_iter()
        .map(|arg| shell_quote(OsStr::new(&arg)))
        .collect::<Vec<_>>()
        .join(" ");
    format!("ssh {args}")
}

fn remote_source(config: &UsenetPullConfig, path: &str) -> String {
    format!("{}@{}:{}", config.remote_user, config.remote_host, path)
}

fn ssh_args(config: &UsenetPullConfig) -> Vec<String> {
    vec![
        "-i".to_string(),
        config.ssh_key_path.display().to_string(),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "IdentitiesOnly=yes".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-o".to_string(),
        "ConnectTimeout=15".to_string(),
    ]
}

fn validate_job_name(job: &str) -> Result<()> {
    let path = Path::new(job);
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(anyhow!("unsafe remote job name: {job:?}")),
    }
}

fn path_with_trailing_slash(path: &Path) -> String {
    format!("{}/", path.display())
}

fn shell_quote(value: &OsStr) -> String {
    let value = value.to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_files_in_recursive_rsync_listing() {
        let listing = r#"
drwxrwxr-x          4,096 2026/05/25 12:00:00 .
drwxrwxr-x          4,096 2026/05/25 12:00:00 BDMV
-rw-rw-r-- 34,042,294,272 2026/05/25 12:00:00 BDMV/STREAM/00009.m2ts
"#;

        assert!(rsync_listing_has_files(listing));
    }

    #[test]
    fn treats_empty_rsync_listing_as_empty() {
        let listing = r#"
drwxrwxr-x          4,096 2026/05/25 12:00:00 .
drwxrwxr-x          4,096 2026/05/25 12:00:00 .rsync-partial
"#;

        assert!(!rsync_listing_has_files(listing));
    }
}
