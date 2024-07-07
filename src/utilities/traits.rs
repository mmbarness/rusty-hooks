use std::{collections::hash_map::DefaultHasher, hash::{Hash, Hasher}, path::{Path, PathBuf}, fs::DirEntry};
use log::{debug, info, error};
use tokio::runtime::Runtime;
use crate::{utilities::{thread_types::Channel,timer::Timer}, errors::watcher_errors::path_error::PathError};
use crate::errors::shared_errors::thread_errors::ThreadError;

pub type DirEntries = Vec<Result<DirEntry, std::io::Error>>;

pub trait Utilities {
    fn hasher(path: &String) -> u64 {
        let mut hasher = DefaultHasher::new();
        path.hash(& mut hasher);
        hasher.finish()
    }

    fn dir_contains_file_type(dir: &DirEntries, extension: &String) -> bool {
        let contains = &dir.iter().any(|entry| {
            let Ok(valid_entry) = entry else { return false };
            let entry_path = valid_entry.path();
            let Some(file_extension) = entry_path.extension() else { return false };
            let Some(ext_str) = file_extension.to_str() else { return false };
            if ext_str == extension { return true }
            false
        });
        *contains
    }

    fn get_first_of_file_type(dir: &DirEntries, extension:  &String) -> Option<PathBuf> {
        let possible_matches:Vec<&Result<std::fs::DirEntry, std::io::Error>> = dir.iter().filter(|file| {
            let Ok(valid_entry) = file else { return false };
            let path_buf = valid_entry.path();
            let entry_path = path_buf.as_path();
            let Some(file_extension) = entry_path.extension() else { return false };
            let Some(ext_str) = file_extension.to_str() else { return false };
            if ext_str == extension { return true }
            false
        }).collect();
        let first_match = possible_matches.iter().next()?;
        let is_match = first_match.as_ref().ok();
        Some(is_match?.path())
    }

    fn format_unvalidated_path(segments: &Vec<&String>) -> String {
        let init_path = "".to_string();
        segments.iter().fold(init_path, |path:String, segment| {
            format!("{}{}", path, segment)
        })
    }

    fn build_path(segments: &Vec<&String>) -> Option<PathBuf> {
        let init_path = PathBuf::new();
        let path = segments.iter().fold(init_path, |path:PathBuf, segment| {
            path.to_path_buf().join(segment)
        });
        Self::log_path(&path, log::Level::Debug);
        Self::verify_path(path)
    }

    fn verify_path(path: PathBuf) -> Option<PathBuf> {
        path.exists().then_some(path)
    }

    fn get_parent_dir_of_file(path:&PathBuf) -> Option<PathBuf> {
        if path.is_file() {
            path.parent().and_then(|p| Some(p.to_path_buf()))
        } else {
            None
        }
    }

    fn log_path(path: &Path, log_level: log::Level) -> () {
        let path_string = path.to_str().unwrap_or("error parsing path into string");
        let message = &format!("path: {}", path_string);
        match log_level {
            log::Level::Debug => {
                debug!("{}", message);
            },
            log::Level::Info => {
                info!("{}", message)
            },
            log::Level::Error => {
                error!("{}", message)
            },
            _ => {
                debug!("{}", message);
            }
        };
    }

    fn new_channel<T:std::clone::Clone>() -> Channel<T> {
        tokio::sync::broadcast::channel::<T>(16)
    }

    fn new_runtime(num_threads: usize, thread_name: &String) -> Result<Runtime, ThreadError> {
        Ok(tokio::runtime::Builder::new_multi_thread()
            .worker_threads(num_threads)
            .thread_name(thread_name)
            .thread_stack_size(3 * 1024 * 1024)
            .enable_time()
            .build()?)
    }

    fn new_timer(wait_duration: i64) -> Timer {
        Timer::new(wait_duration)
    }

    fn path_hasher(path: &PathBuf) -> Result<u64, PathError> {
        let mut hasher = DefaultHasher::new();
        let abs_path = path.canonicalize()?;
        abs_path.hash(& mut hasher);
        Ok(hasher.finish())
    }

    fn walk_up_to_event_home_dir<'a>(leaf:PathBuf, root: PathBuf) -> Result<PathBuf,PathError> {
        // walk upwards from the leaf until we hit the parentmost directory of the event, i.e. the path that is one level below the root
        let leaf_hash = Self::path_hasher(&leaf)?;
        let parent_of_leaf = leaf.parent().ok_or(PathError::TraversalError)?;
        let parent_of_leaf_hash = Self::path_hasher(&parent_of_leaf.to_path_buf())?;
        let root_hash = Self::path_hasher(&root)?;
        if parent_of_leaf_hash == root_hash {
            return Ok(leaf)
        } else if leaf_hash == root_hash {
            return Err(PathError::TraversalError)
        } else {
            Self::walk_up_to_event_home_dir(parent_of_leaf.to_path_buf(), root)
        }
    }

    fn path_contains_subdir(path:&PathBuf, subdir:&PathBuf) -> bool {
        let mut ancestors = subdir.ancestors();
        ancestors.any(|p| &p.to_path_buf() == path)
    }
}
