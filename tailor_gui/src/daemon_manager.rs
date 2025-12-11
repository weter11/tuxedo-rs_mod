// src/daemon_manager.rs
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::profile_controller::ProfileController;
use crate::fan_daemon::FanDaemon;

/// Manages all background daemon processes
pub struct DaemonManager {
    pub profile_controller: Arc<Mutex<ProfileController>>,  // Add 'pub'
    fan_daemon: Arc<Mutex<FanDaemon>>,
    app_monitoring_enabled: Arc<Mutex<bool>>,
}

impl DaemonManager {
    pub fn new(profile_controller: Arc<Mutex<ProfileController>>) -> Result<Self> {
        let fan_daemon = FanDaemon::new(Duration::from_secs(2))?;
        
        Ok(DaemonManager {
            profile_controller,
            fan_daemon: Arc::new(Mutex::new(fan_daemon)),
            app_monitoring_enabled: Arc::new(Mutex::new(false)),
        })
    }

    /// Start all daemons with the current profile
    pub fn start_all(&self) -> Result<()> {
        println!("Starting all daemons...");
        
        // Get current profile
        let profile = {
            let ctrl = self.profile_controller.lock().unwrap();
            ctrl.get_active_profile()
        };

        // Start fan daemon
        self.start_fan_daemon(profile.clone())?;

        // Start app monitoring if enabled in profile
        if profile.auto_switch_enabled {
            self.start_app_monitoring()?;
        }

        println!("All daemons started");
        Ok(())
    }

    /// Stop all daemons
    pub fn stop_all(&self) {
        println!("Stopping all daemons...");

        // Stop fan daemon
        self.stop_fan_daemon();

        // Stop app monitoring
        self.stop_app_monitoring();

        println!("All daemons stopped");
    }

    /// Start the fan curve daemon
    pub fn start_fan_daemon(&self, profile: crate::profile_system::Profile) -> Result<()> {
        let fan_daemon = self.fan_daemon.lock().unwrap();
        fan_daemon.start(profile)?;
        println!("Fan daemon started");
        Ok(())
    }

    /// Stop the fan curve daemon
    pub fn stop_fan_daemon(&self) {
        let fan_daemon = self.fan_daemon.lock().unwrap();
        fan_daemon.stop();
    }

    /// Update fan daemon with new profile
    pub fn update_fan_daemon_profile(&self, profile: crate::profile_system::Profile) {
        let fan_daemon = self.fan_daemon.lock().unwrap();
        fan_daemon.update_profile(profile);
    }

    /// Start application monitoring
    pub fn start_app_monitoring(&self) -> Result<()> {
        let mut enabled = self.app_monitoring_enabled.lock().unwrap();
        if *enabled {
            return Ok(()); // Already running
        }
        *enabled = true;
        drop(enabled);

        let ctrl = self.profile_controller.lock().unwrap();
        ctrl.start_app_monitoring()?;
        println!("App monitoring started");
        Ok(())
    }

    /// Stop application monitoring
    pub fn stop_app_monitoring(&self) {
        let mut enabled = self.app_monitoring_enabled.lock().unwrap();
        *enabled = false;
        drop(enabled);

        let ctrl = self.profile_controller.lock().unwrap();
        ctrl.stop_app_monitoring();
        println!("App monitoring stopped");
    }

    /// Check if fan daemon is running
    pub fn is_fan_daemon_running(&self) -> bool {
        let fan_daemon = self.fan_daemon.lock().unwrap();
        fan_daemon.is_running()
    }

    /// Check if app monitoring is running
    pub fn is_app_monitoring_running(&self) -> bool {
        *self.app_monitoring_enabled.lock().unwrap()
    }

    /// Apply a new profile and update daemons
    pub fn apply_profile(&self, profile_index: usize) -> Result<()> {
        // Apply profile through controller
        let ctrl = self.profile_controller.lock().unwrap();
        ctrl.apply_profile(profile_index)?;
        let profile = ctrl.get_active_profile();
        drop(ctrl);

        // Update fan daemon with new profile
        self.update_fan_daemon_profile(profile.clone());

        // Update app monitoring based on new profile
        if profile.auto_switch_enabled {
            if !self.is_app_monitoring_running() {
                self.start_app_monitoring()?;
            }
        } else {
            if self.is_app_monitoring_running() {
                self.stop_app_monitoring();
            }
        }

        Ok(())
    }

    /// Get status of all daemons
    pub fn get_status(&self) -> DaemonStatus {
        DaemonStatus {
            fan_daemon_running: self.is_fan_daemon_running(),
            app_monitoring_running: self.is_app_monitoring_running(),
            active_profile: {
                let ctrl = self.profile_controller.lock().unwrap();
                ctrl.get_active_profile().name.clone()
            },
        }
    }
}

/// Status information for daemons
#[derive(Debug, Clone)]
pub struct DaemonStatus {
    pub fan_daemon_running: bool,
    pub app_monitoring_running: bool,
    pub active_profile: String,
}

impl std::fmt::Display for DaemonStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Fan Daemon: {}\nApp Monitoring: {}\nActive Profile: {}",
            if self.fan_daemon_running { "Running" } else { "Stopped" },
            if self.app_monitoring_running { "Running" } else { "Stopped" },
            self.active_profile
        )
    }
}
