use std::{path::{Path, PathBuf}, fs::File, io::{Write, ErrorKind}};
use directories::BaseDirs;
use fd_lock::{RwLock, RwLockWriteGuard};
use fs2::FileExt;

use crate::{errors::watcher_errors::path_error::PathError, logger::{structs::Logger, debug::DebugLogging}};

#[derive(Debug)]
pub struct Lockfile{
    pub locked: bool,
    pub lock: Option<File>,
    pub pid: u32,
    pub path: PathBuf
}

impl Lockfile {
    pub fn set(pid: Option<u32>, path: Option<&PathBuf>) -> Result<Self, std::io::Error> {
        let default_path = Lockfile::default_lockfile_path().expect("unable to configure path to write pid to");

        let mut preliminary_self = Lockfile {
            locked: false,
            pid: pid.unwrap_or(std::process::id()).clone(),
            path: path.unwrap_or(&default_path).clone(),
            lock: None,
        };

        let file_exists = preliminary_self.file_path_valid();

        Logger::log_debug_string(&format!("default path to lockfile is: {}", default_path.to_str().unwrap()));

        match file_exists {
            true => {
                // file exists and is unlocked
                preliminary_self.lock_and_update_existing()?;

                return Ok(preliminary_self)
            }
            false => {
                // file doesn't exist, need to write a new one
                preliminary_self.create_if_nonexistent()?;
                preliminary_self.acquire_file_lock()?;

                return Ok(preliminary_self)
            }
        }
    }

    fn default_lockfile_path() -> Option<PathBuf> {
        let home_dir = BaseDirs::new().and_then(|p| Some(p.home_dir().to_path_buf()))?.canonicalize().ok()?;
        let rusty_hooks_subdir = Path::new("rusty-hooks/rusty-hooks.pid").to_path_buf();
        let full_path = [
            home_dir.as_path(),
            rusty_hooks_subdir.as_path()
        ].iter().collect();
        Some(full_path)
    }

    fn lock_and_update_existing(&mut self) -> Result<(), std::io::Error> {
        self.check_for_existing_lock()?;
        let converted_pid = Lockfile::convert(&[self.pid; 4]);
        if let Some(mut t) = &self.lock.as_ref() {
            t.write_all(&converted_pid)?;
        } else {
            self.acquire_file_lock()?;
        };
        Ok(())
    }

    fn check_for_existing_lock(&self) -> Result<(), std::io::Error> {
        let io_error_kind = std::io::ErrorKind::InvalidFilename;
        let filepath_error = std::io::Error::new(io_error_kind, PathError::InvalidPath("error with path provided for lockfile".into()));
        self.file_path_valid().then_some(()).ok_or(filepath_error)?;
        let file = File::open(&self.path)?.try_clone()?;
        file.try_lock_exclusive()
    }

    fn file_path_valid(&self) -> bool {
        self.path.is_file().clone()
    }

    fn convert(data: &[u32; 4]) -> [u8; 16] {
        let mut res = [0; 16];
        for i in 0..4 {
            res[4*i..][..4].copy_from_slice(&data[i].to_le_bytes());
        }
        res
    }

    fn write_pid_file<'a>(&'a self, valid_path: &'a Path) -> Result<&Path, std::io::Error> {
        let mut file = File::create(valid_path)?;
        let pid_as_u8_vec = &Lockfile::convert(&[self.pid; 4]);
        file.write_all(pid_as_u8_vec)?;
        Ok(valid_path)
    }

    fn acquire_file_lock(&mut self) -> std::io::Result<()> {
        let file = File::open(&self.path)?.try_clone()?;
        file.try_lock_exclusive()?;
        self.lock = Some(file);
        self.locked = true;
        Ok(())
    }

    fn create_if_nonexistent(&mut self) -> Result<(),std::io::Error> {
        if self.file_path_valid() {
            return Ok(())
        }

        let provided_path_is_dir = &self.path.is_dir();

        let intended_dir = &self.path.parent();

        Logger::log_debug_string(intended_dir.unwrap().to_str().unwrap());

        return match (provided_path_is_dir, intended_dir) {
            (true, _) => {
                // create file inside dir
                let path_with_filename = &self.path.join("rusty-hooks.pid");
                let mut file = File::create(path_with_filename)?;
                let pid_as_u8_vec = &Lockfile::convert(&[self.pid; 4]);
                file.write_all(pid_as_u8_vec)?;
                self.path = path_with_filename.clone();
                Ok(())
            }
            (false, Some(path)) => {
                // use the path to create file
                let path_with_filename = path.join("rusty-hooks.pid");
                let mut file = File::create(path_with_filename.clone())?;
                let pid_as_u8_vec = &Lockfile::convert(&[self.pid; 4]);
                file.write_all(pid_as_u8_vec)?;
                self.path = path_with_filename;
                Ok(())
            }
            (false, None) => {
                // no useable path
                let io_error_kind = std::io::ErrorKind::InvalidFilename;
                let custom_error2 = std::io::Error::new(ErrorKind::InvalidFilename, PathError::InvalidPath("error with path provided for lockfile".into()));
                Err(custom_error2)
            }
        }
    }
}
