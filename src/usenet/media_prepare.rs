use super::puller::{UsenetCategory, UsenetPullConfig};
use anyhow::{anyhow, bail, Context, Result};
use async_process::Command;
use log::{info, warn};
use serde::Serialize;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tokio::fs;

const MIN_MAIN_STREAM_BYTES: u64 = 1_000_000_000;
const READY_OUTPUT_SUFFIX: &str = "_remux.mkv";

#[derive(Clone, Debug, Default, Serialize)]
pub struct MediaPrepareSummary {
    pub iso_remuxed: usize,
}

struct IsoStream {
    path: String,
    size: u64,
}

pub async fn prepare_for_filebot(
    category: UsenetCategory,
    job: &str,
    destination: &Path,
    config: &UsenetPullConfig,
) -> Result<MediaPrepareSummary> {
    if category != UsenetCategory::Movies {
        return Ok(MediaPrepareSummary::default());
    }

    let iso_paths = find_files_with_extension(destination, "iso")
        .with_context(|| format!("scanning {} for ISO files", destination.display()))?;
    if iso_paths.is_empty() {
        return Ok(MediaPrepareSummary::default());
    }

    let seven_zip = find_seven_zip(config)?;
    ensure_command_exists("ffmpeg").await?;
    ensure_command_exists("ffprobe").await?;

    let mut summary = MediaPrepareSummary::default();
    for iso_path in iso_paths {
        let output = remux_blu_ray_iso(&seven_zip, &iso_path).await?;
        quarantine_source(&iso_path, category, job, config).await?;
        info!(
            "usenet media prepare remuxed ISO source={} output={}",
            iso_path.display(),
            output.display()
        );
        summary.iso_remuxed += 1;
    }

    Ok(summary)
}

async fn remux_blu_ray_iso(seven_zip: &Path, iso_path: &Path) -> Result<PathBuf> {
    let main_stream = largest_blu_ray_stream(seven_zip, iso_path).await?;
    if main_stream.size < MIN_MAIN_STREAM_BYTES {
        bail!(
            "largest Blu-ray stream in {} is too small to trust: {} bytes at {}",
            iso_path.display(),
            main_stream.size,
            main_stream.path
        );
    }

    let output = remux_output_path(iso_path)?;
    let partial = output.with_extension("partial.mkv");

    if output.exists() {
        info!(
            "usenet media prepare output already exists; skipping remux output={}",
            output.display()
        );
        return Ok(output);
    }

    if partial.exists() {
        fs::remove_file(&partial)
            .await
            .with_context(|| format!("removing stale partial MKV {}", partial.display()))?;
    }

    info!(
        "usenet media prepare remuxing ISO source={} stream={} stream_size={} output={}",
        iso_path.display(),
        main_stream.path,
        main_stream.size,
        output.display()
    );

    let title = iso_path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("remuxed Blu-ray");
    let script = r#"
"$1" x -so -tUdf "$2" "$3" | ffmpeg -hide_banner -nostats -v warning -y -fflags +genpts -i pipe:0 -map 0 -c copy -metadata "title=$5" "$4"
"#;
    let output_result = Command::new("bash")
        .arg("-o")
        .arg("pipefail")
        .arg("-c")
        .arg(script)
        .arg("rusty-hooks-remux")
        .arg(seven_zip)
        .arg(iso_path)
        .arg(&main_stream.path)
        .arg(&partial)
        .arg(title)
        .output()
        .await
        .with_context(|| format!("spawning ffmpeg remux for {}", iso_path.display()))?;

    if !output_result.status.success() {
        let stderr = String::from_utf8_lossy(&output_result.stderr);
        let stdout = String::from_utf8_lossy(&output_result.stdout);
        bail!(
            "ffmpeg remux failed for {}: stdout={} stderr={}",
            iso_path.display(),
            stdout.trim(),
            stderr.trim()
        );
    }

    verify_media_file(&partial).await?;
    fs::rename(&partial, &output)
        .await
        .with_context(|| format!("publishing remuxed MKV {}", output.display()))?;

    Ok(output)
}

async fn largest_blu_ray_stream(seven_zip: &Path, iso_path: &Path) -> Result<IsoStream> {
    let output = Command::new(seven_zip)
        .arg("l")
        .arg("-slt")
        .arg("-tUdf")
        .arg(iso_path)
        .output()
        .await
        .with_context(|| format!("listing ISO contents {}", iso_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "unable to read ISO as UDF with {}: {}",
            seven_zip.display(),
            stderr.trim()
        );
    }

    let stdout = String::from_utf8(output.stdout)
        .with_context(|| format!("decoding 7z output for {}", iso_path.display()))?;
    parse_largest_blu_ray_stream(&stdout).ok_or_else(|| {
        anyhow!(
            "no BDMV/STREAM/*.m2ts files found in {}",
            iso_path.display()
        )
    })
}

fn parse_largest_blu_ray_stream(output: &str) -> Option<IsoStream> {
    let mut current_path: Option<String> = None;
    let mut largest: Option<IsoStream> = None;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("Path = ") {
            current_path = Some(path.trim().to_string());
            continue;
        }

        let Some(size) = line.strip_prefix("Size = ") else {
            continue;
        };
        let Some(path) = current_path.as_ref() else {
            continue;
        };
        if !is_blu_ray_stream(path) {
            continue;
        }

        let Ok(size) = size.trim().parse::<u64>() else {
            continue;
        };
        if largest
            .as_ref()
            .map(|stream| size > stream.size)
            .unwrap_or(true)
        {
            largest = Some(IsoStream {
                path: path.clone(),
                size,
            });
        }
    }

    largest
}

fn is_blu_ray_stream(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.starts_with("BDMV/STREAM/") && normalized.to_ascii_lowercase().ends_with(".m2ts")
}

fn remux_output_path(iso_path: &Path) -> Result<PathBuf> {
    let parent = iso_path
        .parent()
        .ok_or_else(|| anyhow!("ISO path has no parent: {}", iso_path.display()))?;
    let stem = iso_path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow!("ISO path has no UTF-8 stem: {}", iso_path.display()))?;
    Ok(parent.join(format!("{stem}{READY_OUTPUT_SUFFIX}")))
}

async fn verify_media_file(path: &Path) -> Result<()> {
    let output = Command::new("ffprobe")
        .arg("-hide_banner")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=nw=1:nk=1")
        .arg(path)
        .output()
        .await
        .with_context(|| format!("spawning ffprobe for {}", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("ffprobe failed for {}: {}", path.display(), stderr.trim());
    }

    Ok(())
}

async fn quarantine_source(
    iso_path: &Path,
    category: UsenetCategory,
    job: &str,
    config: &UsenetPullConfig,
) -> Result<()> {
    let quarantine_root = config
        .staging_root
        .join("unsupported-originals")
        .join(category.as_remote_name())
        .join(job);
    fs::create_dir_all(&quarantine_root)
        .await
        .with_context(|| format!("creating quarantine {}", quarantine_root.display()))?;

    let sidecars = sidecar_paths(iso_path);
    move_file_to_directory(iso_path, &quarantine_root).await?;
    for sidecar in sidecars {
        if sidecar.exists() {
            move_file_to_directory(&sidecar, &quarantine_root).await?;
        }
    }

    Ok(())
}

fn sidecar_paths(iso_path: &Path) -> Vec<PathBuf> {
    ["nfo", "sfv", "txt"]
        .iter()
        .map(|extension| iso_path.with_extension(extension))
        .collect()
}

async fn move_file_to_directory(source: &Path, destination_dir: &Path) -> Result<()> {
    let file_name = source
        .file_name()
        .ok_or_else(|| anyhow!("source path has no file name: {}", source.display()))?;
    let destination = unique_destination(destination_dir.join(file_name));

    match fs::rename(source, &destination).await {
        Ok(_) => Ok(()),
        Err(error) => {
            warn!(
                "rename failed while quarantining {}; falling back to copy/remove: {}",
                source.display(),
                error
            );
            fs::copy(source, &destination).await.with_context(|| {
                format!(
                    "copying quarantine source {} to {}",
                    source.display(),
                    destination.display()
                )
            })?;
            fs::remove_file(source)
                .await
                .with_context(|| format!("removing original {}", source.display()))?;
            Ok(())
        }
    }
}

fn unique_destination(destination: PathBuf) -> PathBuf {
    if !destination.exists() {
        return destination;
    }

    let parent = destination
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = destination
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("file");
    let extension = destination.extension().and_then(OsStr::to_str);

    for index in 1.. {
        let file_name = match extension {
            Some(extension) => format!("{stem}.{index}.{extension}"),
            None => format!("{stem}.{index}"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded loop returns a unique destination")
}

fn find_seven_zip(config: &UsenetPullConfig) -> Result<PathBuf> {
    let candidates = [
        config.staging_root.join("tools/7zz"),
        PathBuf::from("tools/7zz"),
        PathBuf::from("7zz"),
        PathBuf::from("7z"),
    ];

    for candidate in candidates {
        if command_is_available(&candidate) {
            return Ok(candidate);
        }
    }

    bail!(
        "no UDF-capable 7-Zip binary found; expected one of {}/tools/7zz, ./tools/7zz, 7zz, or 7z",
        config.staging_root.display()
    )
}

async fn ensure_command_exists(command: &str) -> Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg("command -v \"$1\" >/dev/null 2>&1")
        .arg("rusty-hooks-command-check")
        .arg(command)
        .status()
        .await
        .with_context(|| format!("checking for command {command}"))?;
    if status.success() {
        Ok(())
    } else {
        bail!("required command not found in PATH: {command}")
    }
}

fn command_is_available(candidate: &Path) -> bool {
    if candidate.components().count() > 1 || candidate.is_absolute() {
        return candidate.is_file();
    }

    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|path| path.join(candidate).is_file()))
        .unwrap_or(false)
}

fn find_files_with_extension(root: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    collect_files_with_extension(root, extension, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_files_with_extension(
    path: &Path,
    extension: &str,
    paths: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in std::fs::read_dir(path).with_context(|| format!("reading {}", path.display()))? {
        let entry = entry.with_context(|| format!("reading directory entry {}", path.display()))?;
        let entry_path = entry.path();
        let metadata = entry
            .metadata()
            .with_context(|| format!("reading metadata {}", entry_path.display()))?;

        if metadata.is_dir() {
            collect_files_with_extension(&entry_path, extension, paths)?;
        } else if metadata.is_file()
            && entry_path
                .extension()
                .and_then(OsStr::to_str)
                .map(|value| value.eq_ignore_ascii_case(extension))
                .unwrap_or(false)
        {
            paths.push(entry_path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_largest_blu_ray_stream() {
        let output = r#"
Path = BDMV/STREAM/00001.m2ts
Size = 100
Path = BDMV/STREAM/00002.m2ts
Size = 900
Path = BDMV/PLAYLIST/00001.mpls
Size = 1000
"#;

        let stream = parse_largest_blu_ray_stream(output).expect("stream");
        assert_eq!(stream.path, "BDMV/STREAM/00002.m2ts");
        assert_eq!(stream.size, 900);
    }

    #[test]
    fn ignores_non_bdmv_streams() {
        let output = r#"
Path = not-bdmv/STREAM/00001.m2ts
Size = 1000
Path = BDMV/PLAYLIST/00001.mpls
Size = 2000
"#;

        assert!(parse_largest_blu_ray_stream(output).is_none());
    }
}
