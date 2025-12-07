// src/profile_controller.rs
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::profile_system::{ProfileManager, Profile};
use crate::hardware_monitor::HardwareMonitor;
use crate::hardware_control::HardwareController;

/// High-level controller that manages profile application and monitoring
pub struct ProfileController {
    profile_manager: Arc<Mutex<ProfileManager>>,
    hardware_controller: Arc<HardwareController>,
    hardware_monitor: Arc<Mutex<HardwareMonitor>>,
    monitoring_enabled: Arc<Mutex<bool>>,
}

impl ProfileController {
    pub fn new() -> Result<Self> {
        Ok(ProfileController {
            profile_manager: Arc::new(Mutex::new(ProfileManager::new()?)),
            hardware_controller: Arc::new(HardwareController::new()?),
            hardware_monitor: Arc::new(Mutex::new(HardwareMonitor::new()?)),
            monitoring_enabled: Arc::new(Mutex::new(false)),
        })
    }
    
    /// Apply a profile by index
    pub fn apply_profile(&self, profile_index: usize) -> Result<()> {
        let mut mgr = self.profile_manager.lock().unwrap();
        mgr.set_active_profile(profile_index)?;
        let profile = mgr.get_active_profile().clone();
        drop(mgr); // Release lock
        
        self.hardware_controller.apply_profile(&profile)
    }
    
    /// Apply a profile by name
    pub fn apply_profile_by_name(&self, name: &str) -> Result<()> {
        let mgr = self.profile_manager.lock().unwrap();
        let profile_index = mgr.get_profiles()
            .iter()
            .position(|p| p.name == name)
            .context(format!("Profile '{}' not found", name))?;
        drop(mgr);
        
        self.apply_profile(profile_index)
    }
    
    /// Get the currently active profile
    pub fn get_active_profile(&self) -> Profile {
        let mgr = self.profile_manager.lock().unwrap();
        mgr.get_active_profile().clone()
    }
    
    /// Get all profiles
    pub fn get_all_profiles(&self) -> Vec<Profile> {
        let mgr = self.profile_manager.lock().unwrap();
        mgr.get_profiles().to_vec()
    }
    
    /// Add a new profile
    pub fn add_profile(&self, profile: Profile) -> Result<()> {
        let mut mgr = self.profile_manager.lock().unwrap();
        mgr.add_profile(profile)
    }
    
    /// Update an existing profile
    pub fn update_profile(&self, index: usize, profile: Profile) -> Result<()> {
        let mut mgr = self.profile_manager.lock().unwrap();
        mgr.update_profile(index, profile)
    }
    
    /// Delete a profile
    pub fn delete_profile(&self, index: usize) -> Result<()> {
        let mut mgr = self.profile_manager.lock().unwrap();
        mgr.delete_profile(index)
    }
    
    /// Get current hardware statistics
    pub fn get_hardware_stats(&self) -> Result<crate::hardware_monitor::SystemStats> {
        let mut monitor = self.hardware_monitor.lock().unwrap();
        monitor.get_system_stats()
    }
    
    /// Switch GPU (requires restart)
    pub fn switch_gpu(&self, use_discrete: bool) -> Result<()> {
        self.hardware_controller.switch_gpu(use_discrete)
    }
    
    /// Enable maximum performance mode
    pub fn enable_maximum_performance(&self) -> Result<()> {
        self.hardware_controller.set_maximum_performance()
    }
    
    /// Start monitoring for application-triggered profile switching
    pub fn start_app_monitoring(&self) -> Result<()> {
        let mut enabled = self.monitoring_enabled.lock().unwrap();
        if *enabled {
            return Ok(()); // Already monitoring
        }
        *enabled = true;
        drop(enabled);
        
        let profile_manager = Arc::clone(&self.profile_manager);
        let hardware_controller = Arc::clone(&self.hardware_controller);
        let monitoring_enabled = Arc::clone(&self.monitoring_enabled);
        
        thread::spawn(move || {
            let mut last_detected_app = String::new();
            
            loop {
                // Check if monitoring is still enabled
                {
                    let enabled = monitoring_enabled.lock().unwrap();
                    if !*enabled {
                        break;
                    }
                }
                
                // Get running processes
                if let Ok(current_app) = detect_running_apps() {
                    if current_app != last_detected_app {
                        // Check if any profile should be triggered
                        let mgr = profile_manager.lock().unwrap();
                        if let Some(profile_index) = mgr.find_profile_for_app(&current_app) {
                            let profile = mgr.get_profiles()[profile_index].clone();
                            drop(mgr);
                            
                            println!("Auto-switching to profile '{}' for app: {}", 
                                     profile.name, current_app);
                            
                            if let Err(e) = hardware_controller.apply_profile(&profile) {
                                eprintln!("Failed to apply profile: {}", e);
                            }
                            
                            last_detected_app = current_app;
                        }
                    }
                }
                
                thread::sleep(Duration::from_secs(5)); // Check every 5 seconds
            }
        });
        
        println!("Application monitoring started");
        Ok(())
    }
    
    /// Stop monitoring for application-triggered profile switching
    pub fn stop_app_monitoring(&self) {
        let mut enabled = self.monitoring_enabled.lock().unwrap();
        *enabled = false;
        println!("Application monitoring stopped");
    }
}

/// Detect running applications (Steam, Lutris, etc.)
fn detect_running_apps() -> Result<String> {
    // Read /proc to find running processes
    let proc_path = std::path::Path::new("/proc");
    
    for entry in std::fs::read_dir(proc_path)? {
        let entry = entry?;
        let path = entry.path();
        
        // Only check numeric directories (PIDs)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.chars().all(|c| c.is_numeric()) {
                // Read cmdline
                let cmdline_path = path.join("cmdline");
                if let Ok(cmdline) = std::fs::read_to_string(&cmdline_path) {
                    let cmdline_lower = cmdline.to_lowercase();
                    
                    // Check for known gaming apps
                    if cmdline_lower.contains("steam") {
                        return Ok("steam".to_string());
                    }
                    if cmdline_lower.contains("lutris") {
                        return Ok("lutris".to_string());
                    }
                    if cmdline_lower.contains("gamemode") {
                        return Ok("gamemode".to_string());
                    }
                    // Add more apps as needed
                }
            }
        }
    }
    
    Ok(String::new())
}

/// Builder for creating profiles easily
pub struct ProfileBuilder {
    profile: Profile,
}

impl ProfileBuilder {
    pub fn new(name: &str) -> Self {
        let mut profile = Profile::default_profile();
        profile.name = name.to_string();
        profile.is_default = false;
        
        ProfileBuilder { profile }
    }
    
    pub fn keyboard_color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.profile.keyboard_backlight.color.r = r;
        self.profile.keyboard_backlight.color.g = g;
        self.profile.keyboard_backlight.color.b = b;
        self
    }
    
    pub fn keyboard_brightness(mut self, brightness: u8) -> Self {
        self.profile.keyboard_backlight.brightness = brightness;
        self
    }
    
    pub fn cpu_performance(mut self, profile: crate::profile_system::CpuPerformanceProfile) -> Self {
        self.profile.cpu_settings.performance_profile = profile;
        self
    }
    
    pub fn cpu_frequency_limits(mut self, min_mhz: Option<u32>, max_mhz: Option<u32>) -> Self {
        self.profile.cpu_settings.min_freq_mhz = min_mhz;
        self.profile.cpu_settings.max_freq_mhz = max_mhz;
        self
    }
    
    pub fn disable_boost(mut self, disable: bool) -> Self {
        self.profile.cpu_settings.disable_boost = disable;
        self
    }
    
    pub fn smt_enabled(mut self, enabled: bool) -> Self {
        self.profile.cpu_settings.smt_enabled = enabled;
        self
    }
    
    pub fn screen_brightness(mut self, brightness: u8) -> Self {
        self.profile.screen_settings.brightness = brightness;
        self
    }
    
    pub fn auto_switch_for_apps(mut self, apps: Vec<String>) -> Self {
        self.profile.auto_switch_enabled = true;
        self.profile.trigger_apps = apps;
        self
    }
    
    pub fn build(self) -> Profile {
        self.profile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_builder() {
        let profile = ProfileBuilder::new("Test Gaming")
            .keyboard_color(255, 0, 0)
            .keyboard_brightness(100)
            .cpu_performance(crate::profile_system::CpuPerformanceProfile::Performance)
            .auto_switch_for_apps(vec!["steam".to_string()])
            .build();
        
        assert_eq!(profile.name, "Test Gaming");
        assert_eq!(profile.keyboard_backlight.color.r, 255);
        assert!(profile.auto_switch_enabled);
    }
}
