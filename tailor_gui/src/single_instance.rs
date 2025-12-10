// src/single_instance.rs
use std::fs::{self, File, OpenOptions};
use std::io::{Write, Read};
use std::path::PathBuf;
use anyhow::{Context, Result};

pub struct SingleInstance {
    lock_file_path: PathBuf,
    _lock_file: Option<File>,
}

impl SingleInstance {
    pub fn new(app_id: &str) -> Result<Self> {
        let lock_file_path = Self::get_lock_file_path(app_id)?;
        
        Ok(SingleInstance {
            lock_file_path,
            _lock_file: None,
        })
    }
    
    fn get_lock_file_path(app_id: &str) -> Result<PathBuf> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .or_else(|_| std::env::var("TMPDIR"))
            .unwrap_or_else(|_| "/tmp".to_string());
        
        let lock_file = format!("{}.lock", app_id.replace('.', "_"));
        Ok(PathBuf::from(runtime_dir).join(lock_file))
    }
    
    /// Check if another instance is running
    pub fn is_running(&self) -> bool {
        if !self.lock_file_path.exists() {
            return false;
        }
        
        // Read PID from lock file
        if let Ok(mut file) = File::open(&self.lock_file_path) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(pid) = contents.trim().parse::<i32>() {
                    // Check if process is still running
                    return Self::is_process_running(pid);
                }
            }
        }
        
        false
    }
    
    /// Try to acquire the lock
    pub fn try_acquire(&mut self) -> Result<bool> {
        // Check if already running
        if self.is_running() {
            return Ok(false);
        }
        
        // Clean up stale lock file
        let _ = fs::remove_file(&self.lock_file_path);
        
        // Create new lock file with current PID
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.lock_file_path)
            .context("Failed to create lock file")?;
        
        let pid = std::process::id();
        writeln!(file, "{}", pid)
            .context("Failed to write PID to lock file")?;
        
        self._lock_file = Some(file);
        
        Ok(true)
    }
    
    /// Release the lock
    pub fn release(&mut self) {
        self._lock_file = None;
        let _ = fs::remove_file(&self.lock_file_path);
    }
    
    /// Check if a process with given PID is running
    fn is_process_running(pid: i32) -> bool {
        let proc_path = format!("/proc/{}", pid);
        PathBuf::from(proc_path).exists()
    }
    
    /// Get the PID of the running instance
    pub fn get_running_pid(&self) -> Option<i32> {
        if let Ok(mut file) = File::open(&self.lock_file_path) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                if let Ok(pid) = contents.trim().parse::<i32>() {
                    if Self::is_process_running(pid) {
                        return Some(pid);
                    }
                }
            }
        }
        None
    }
    
    /// Send a signal to bring the running instance to front
    pub fn activate_running_instance(&self) -> Result<()> {
        if let Some(pid) = self.get_running_pid() {
            // Send USR1 signal to activate window
            unsafe {
                libc::kill(pid, libc::SIGUSR1);
            }
            Ok(())
        } else {
            anyhow::bail!("No running instance found")
        }
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_instance() {
        let mut instance1 = SingleInstance::new("test.app").unwrap();
        
        // First instance should acquire lock
        assert!(instance1.try_acquire().unwrap());
        
        // Second instance should fail
        let mut instance2 = SingleInstance::new("test.app").unwrap();
        assert!(!instance2.try_acquire().unwrap());
        
        // After releasing, second instance should succeed
        instance1.release();
        assert!(instance2.try_acquire().unwrap());
    }
}
