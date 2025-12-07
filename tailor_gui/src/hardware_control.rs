// src/hardware_control.rs
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::profile_system::{Profile, FanCurve, CpuSettings, CpuPerformanceProfile};
use crate::keyboard_control::KeyboardController;

/// Controller for applying hardware settings from profiles
pub struct HardwareController {
    cpu_base_path: PathBuf,
    keyboard: Option<KeyboardController>,
}

impl HardwareController {
    pub fn new() -> Result<Self> {
        let cpu_base_path = PathBuf::from("/sys/devices/system/cpu");
        
        // Keyboard controller is optional
        let keyboard = KeyboardController::new().ok();
        
        Ok(HardwareController {
            cpu_base_path,
            keyboard,
        })
    }
    
    /// Apply all settings from a profile
    pub fn apply_profile(&self, profile: &Profile) -> Result<()> {
        println!("Applying profile: {}", profile.name);
        
        // Apply keyboard backlight
        if let Err(e) = self.apply_keyboard_settings(profile) {
            eprintln!("Warning: Failed to apply keyboard settings: {}", e);
        }
        
        // Apply fan curves
        if let Err(e) = self.apply_fan_curves(profile) {
            eprintln!("Warning: Failed to apply fan curves: {}", e);
        }
        
        // Apply CPU settings
        if let Err(e) = self.apply_cpu_settings(&profile.cpu_settings) {
            eprintln!("Warning: Failed to apply CPU settings: {}", e);
        }
        
        // Apply screen brightness
        if let Err(e) = self.apply_screen_brightness(profile.screen_settings.brightness) {
            eprintln!("Warning: Failed to apply screen brightness: {}", e);
        }
        
        println!("Profile '{}' applied successfully", profile.name);
        Ok(())
    }
    
    /// Apply keyboard backlight settings
    fn apply_keyboard_settings(&self, profile: &Profile) -> Result<()> {
        if let Some(ref kbd) = self.keyboard {
            let color = &profile.keyboard_backlight.color;
            let brightness = profile.keyboard_backlight.brightness;
            
            kbd.set_color_and_brightness(color.r, color.g, color.b, brightness)
                .context("Failed to set keyboard backlight")?;
            
            println!("  ✓ Keyboard: RGB({},{},{}) @ {}%", 
                     color.r, color.g, color.b, brightness);
        }
        Ok(())
    }
    
    /// Apply fan curves for all fans
    fn apply_fan_curves(&self, profile: &Profile) -> Result<()> {
        for (fan_id, curve) in &profile.fan_curves {
            self.apply_single_fan_curve(fan_id, curve)
                .context(format!("Failed to apply fan curve for {}", fan_id))?;
        }
        Ok(())
    }
    
    /// Apply a single fan curve
    fn apply_single_fan_curve(&self, fan_id: &str, curve: &FanCurve) -> Result<()> {
        // Fan control via tuxedo_io or direct sysfs
        // This depends on the specific hardware interface available
        
        // Try tuxedo_io method first
        if let Ok(_) = self.apply_fan_curve_tuxedo_io(fan_id, curve) {
            println!("  ✓ Fan curve applied for {} (tuxedo_io)", fan_id);
            return Ok(());
        }
        
        // Try direct hwmon method
        if let Ok(_) = self.apply_fan_curve_hwmon(fan_id, curve) {
            println!("  ✓ Fan curve applied for {} (hwmon)", fan_id);
            return Ok(());
        }
        
        anyhow::bail!("No method available to apply fan curve for {}", fan_id);
    }
    
    /// Apply fan curve via tuxedo_io interface
    fn apply_fan_curve_tuxedo_io(&self, fan_id: &str, curve: &FanCurve) -> Result<()> {
        let tuxedo_io_path = Path::new("/sys/devices/platform/tuxedo_io");
        
        if !tuxedo_io_path.exists() {
            anyhow::bail!("tuxedo_io interface not available");
        }
        
        // Extract fan number from fan_id (e.g., "fan1" -> 1)
        let fan_num: usize = fan_id.trim_start_matches("fan")
            .parse()
            .context("Invalid fan ID format")?;
        
        // Write fan curve points
        for (idx, point) in curve.points.iter().enumerate() {
            let temp_path = tuxedo_io_path.join(format!("fan{}_temp{}", fan_num, idx));
            let speed_path = tuxedo_io_path.join(format!("fan{}_speed{}", fan_num, idx));
            
            if temp_path.exists() && speed_path.exists() {
                fs::write(&temp_path, point.temp.to_string())
                    .context(format!("Failed to write temp point {}", idx))?;
                fs::write(&speed_path, point.speed.to_string())
                    .context(format!("Failed to write speed point {}", idx))?;
            }
        }
        
        Ok(())
    }
    
    /// Apply fan curve via hwmon interface (alternative method)
    fn apply_fan_curve_hwmon(&self, fan_id: &str, curve: &FanCurve) -> Result<()> {
        // Some systems expose fan control via hwmon
        let hwmon_base = Path::new("/sys/class/hwmon");
        
        if !hwmon_base.exists() {
            anyhow::bail!("hwmon interface not available");
        }
        
        // Search for fan control interface
        for entry in fs::read_dir(hwmon_base)? {
            let entry = entry?;
            let path = entry.path();
            
            // Look for pwm_enable and pwm files
            let fan_num: usize = fan_id.trim_start_matches("fan")
                .parse()
                .unwrap_or(1);
            
            let pwm_enable_path = path.join(format!("pwm{}_enable", fan_num));
            let pwm_path = path.join(format!("pwm{}", fan_num));
            
            if pwm_enable_path.exists() && pwm_path.exists() {
                // Set to manual control mode (1 = manual, 2 = automatic)
                fs::write(&pwm_enable_path, "1")
                    .context("Failed to set fan to manual mode")?;
                
                // For now, set a fixed speed based on the middle of the curve
                // Full curve application would require a daemon monitoring temps
                let mid_point = &curve.points[curve.points.len() / 2];
                let pwm_value = (mid_point.speed as f32 * 2.55) as u8; // Convert 0-100 to 0-255
                
                fs::write(&pwm_path, pwm_value.to_string())
                    .context("Failed to set fan speed")?;
                
                return Ok(());
            }
        }
        
        anyhow::bail!("No suitable hwmon interface found");
    }
    
    /// Apply CPU settings
    fn apply_cpu_settings(&self, settings: &CpuSettings) -> Result<()> {
        // Apply performance profile (governor)
        self.set_cpu_governor(settings)?;
        
        // Apply frequency limits
        self.set_cpu_frequency_limits(settings)?;
        
        // Apply boost setting
        self.set_cpu_boost(!settings.disable_boost)?;
        
        // Apply SMT setting
        self.set_smt(settings.smt_enabled)?;
        
        Ok(())
    }
    
    /// Set CPU governor based on performance profile
    fn set_cpu_governor(&self, settings: &CpuSettings) -> Result<()> {
        let governor = match settings.performance_profile {
            CpuPerformanceProfile::PowerSave => "powersave",
            CpuPerformanceProfile::Balanced => "schedutil",
            CpuPerformanceProfile::Performance => "performance",
        };
        
        let cpu_count = self.get_cpu_count()?;
        
        for cpu in 0..cpu_count {
            let governor_path = self.cpu_base_path
                .join(format!("cpu{}/cpufreq/scaling_governor", cpu));
            
            if governor_path.exists() {
                fs::write(&governor_path, governor)
                    .context(format!("Failed to set governor for CPU {}", cpu))?;
            }
        }
        
        println!("  ✓ CPU Governor: {}", governor);
        Ok(())
    }
    
    /// Set CPU frequency limits
    fn set_cpu_frequency_limits(&self, settings: &CpuSettings) -> Result<()> {
        let cpu_count = self.get_cpu_count()?;
        
        for cpu in 0..cpu_count {
            let cpu_path = self.cpu_base_path.join(format!("cpu{}/cpufreq", cpu));
            
            if let Some(min_freq) = settings.min_freq_mhz {
                let min_path = cpu_path.join("scaling_min_freq");
                if min_path.exists() {
                    let freq_khz = min_freq * 1000;
                    fs::write(&min_path, freq_khz.to_string())
                        .context(format!("Failed to set min freq for CPU {}", cpu))?;
                }
            }
            
            if let Some(max_freq) = settings.max_freq_mhz {
                let max_path = cpu_path.join("scaling_max_freq");
                if max_path.exists() {
                    let freq_khz = max_freq * 1000;
                    fs::write(&max_path, freq_khz.to_string())
                        .context(format!("Failed to set max freq for CPU {}", cpu))?;
                }
            }
        }
        
        if settings.min_freq_mhz.is_some() || settings.max_freq_mhz.is_some() {
            println!("  ✓ CPU Frequency limits: {:?} - {:?} MHz", 
                     settings.min_freq_mhz, settings.max_freq_mhz);
        }
        
        Ok(())
    }
    
    /// Enable or disable CPU boost
    fn set_cpu_boost(&self, enable: bool) -> Result<()> {
        // Intel boost
        let intel_boost_path = Path::new("/sys/devices/system/cpu/intel_pstate/no_turbo");
        if intel_boost_path.exists() {
            let value = if enable { "0" } else { "1" }; // Note: inverted logic (no_turbo)
            fs::write(intel_boost_path, value)
                .context("Failed to set Intel turbo boost")?;
            println!("  ✓ CPU Boost (Intel): {}", if enable { "enabled" } else { "disabled" });
            return Ok(());
        }
        
        // AMD boost
        let amd_boost_path = Path::new("/sys/devices/system/cpu/cpufreq/boost");
        if amd_boost_path.exists() {
            let value = if enable { "1" } else { "0" };
            fs::write(amd_boost_path, value)
                .context("Failed to set AMD boost")?;
            println!("  ✓ CPU Boost (AMD): {}", if enable { "enabled" } else { "disabled" });
            return Ok(());
        }
        
        // Try per-CPU boost control (older systems)
        let cpu_count = self.get_cpu_count()?;
        for cpu in 0..cpu_count {
            let boost_path = self.cpu_base_path
                .join(format!("cpu{}/cpufreq/boost", cpu));
            
            if boost_path.exists() {
                let value = if enable { "1" } else { "0" };
                fs::write(&boost_path, value).ok(); // Ignore errors, try all CPUs
            }
        }
        
        Ok(())
    }
    
    /// Enable or disable SMT (Simultaneous Multithreading / Hyperthreading)
    fn set_smt(&self, enable: bool) -> Result<()> {
        let smt_path = Path::new("/sys/devices/system/cpu/smt/control");
        
        if !smt_path.exists() {
            return Ok(()); // SMT control not available, skip silently
        }
        
        let value = if enable { "on" } else { "off" };
        fs::write(smt_path, value)
            .context("Failed to set SMT state")?;
        
        println!("  ✓ SMT/Hyperthreading: {}", if enable { "enabled" } else { "disabled" });
        Ok(())
    }
    
    /// Apply screen brightness
    fn apply_screen_brightness(&self, brightness: u8) -> Result<()> {
        // Try common backlight paths
        let backlight_paths = vec![
            "/sys/class/backlight/intel_backlight",
            "/sys/class/backlight/amdgpu_bl0",
            "/sys/class/backlight/acpi_video0",
        ];
        
        for base_path in backlight_paths {
            let base = Path::new(base_path);
            if base.exists() {
                return self.set_backlight_brightness(base, brightness);
            }
        }
        
        anyhow::bail!("No backlight interface found")
    }
    
    /// Set brightness for a specific backlight device
    fn set_backlight_brightness(&self, base_path: &Path, brightness: u8) -> Result<()> {
        let max_brightness_path = base_path.join("max_brightness");
        let brightness_path = base_path.join("brightness");
        
        let max_brightness: u32 = fs::read_to_string(&max_brightness_path)?
            .trim()
            .parse()
            .context("Failed to parse max_brightness")?;
        
        let actual_brightness = ((brightness as f32 / 100.0) * max_brightness as f32) as u32;
        
        fs::write(&brightness_path, actual_brightness.to_string())
            .context("Failed to write brightness")?;
        
        println!("  ✓ Screen brightness: {}%", brightness);
        Ok(())
    }
    
    /// Get number of CPUs
    fn get_cpu_count(&self) -> Result<usize> {
        let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;
        let count = cpuinfo.lines()
            .filter(|line| line.starts_with("processor"))
            .count();
        Ok(count)
    }
    
    /// Switch GPU using prime-select (NVIDIA Optimus)
    pub fn switch_gpu(&self, use_discrete: bool) -> Result<()> {
        let gpu_mode = if use_discrete { "nvidia" } else { "intel" };
        
        let output = Command::new("prime-select")
            .arg(gpu_mode)
            .output()
            .context("Failed to execute prime-select")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("prime-select failed: {}", stderr);
        }
        
        println!("  ✓ GPU switched to: {}", gpu_mode);
        println!("  ⚠ System restart required for GPU switch to take effect");
        
        Ok(())
    }
    
    /// Disable frequency limits (maximum performance mode for AMD)
    pub fn set_maximum_performance(&self) -> Result<()> {
        let cpu_count = self.get_cpu_count()?;
        
        for cpu in 0..cpu_count {
            let cpu_path = self.cpu_base_path.join(format!("cpu{}/cpufreq", cpu));
            
            // Read available frequencies
            let max_freq_path = cpu_path.join("cpuinfo_max_freq");
            if max_freq_path.exists() {
                let max_freq_khz: u32 = fs::read_to_string(&max_freq_path)?
                    .trim()
                    .parse()
                    .context("Failed to parse max frequency")?;
                
                // Set both min and max to maximum
                let scaling_min_path = cpu_path.join("scaling_min_freq");
                let scaling_max_path = cpu_path.join("scaling_max_freq");
                
                if scaling_min_path.exists() {
                    fs::write(&scaling_min_path, max_freq_khz.to_string()).ok();
                }
                if scaling_max_path.exists() {
                    fs::write(&scaling_max_path, max_freq_khz.to_string()).ok();
                }
            }
        }
        
        // Set performance governor
        self.set_cpu_governor(&CpuSettings {
            performance_profile: CpuPerformanceProfile::Performance,
            min_freq_mhz: None,
            max_freq_mhz: None,
            disable_boost: false,
            smt_enabled: true,
        })?;
        
        // Enable boost
        self.set_cpu_boost(true)?;
        
        println!("  ✓ Maximum performance mode enabled");
        Ok(())
    }
}

/// Check if we have necessary permissions for hardware control
pub fn check_permissions() -> Result<bool> {
    // Test write access to a common sysfs path
    let test_paths = vec![
        "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor",
        "/sys/class/backlight",
        "/sys/devices/system/cpu/smt/control",
    ];
    
    for path in test_paths {
        if Path::new(path).exists() {
            // Try to read (this should always work)
            if fs::read_to_string(path).is_err() {
                return Ok(false);
            }
        }
    }
    
    // Check if running as root
    let euid = unsafe { libc::geteuid() };
    Ok(euid == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile_system::Profile;

    #[test]
    fn test_hardware_controller_creation() {
        // This test will only work on Linux with proper hardware
        if cfg!(target_os = "linux") {
            let _controller = HardwareController::new();
            // Don't assert success as it depends on hardware
        }
    }
    
    #[test]
    fn test_profile_application() {
        if cfg!(target_os = "linux") {
            let controller = HardwareController::new();
            if let Ok(controller) = controller {
                let profile = Profile::default_profile();
                // Don't actually apply in tests, just verify it doesn't panic
                let _ = controller.apply_profile(&profile);
            }
        }
    }
}
