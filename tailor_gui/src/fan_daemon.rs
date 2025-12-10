// src/fan_daemon.rs
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use crate::profile_system::{Profile, FanCurve, FanCurvePoint};
use crate::hardware_monitor::HardwareMonitor;
use crate::hardware_control::HardwareController;

/// Fan curve monitoring daemon
/// Continuously monitors temperatures and adjusts fan speeds according to curves
pub struct FanDaemon {
    monitor: Arc<Mutex<HardwareMonitor>>,
    controller: Arc<HardwareController>,
    running: Arc<Mutex<bool>>,
    current_profile: Arc<Mutex<Option<Profile>>>,
    update_interval: Duration,
}

impl FanDaemon {
    pub fn new(update_interval: Duration) -> Result<Self> {
        Ok(FanDaemon {
            monitor: Arc::new(Mutex::new(HardwareMonitor::new()?)),
            controller: Arc::new(HardwareController::new()?),
            running: Arc::new(Mutex::new(false)),
            current_profile: Arc::new(Mutex::new(None)),
            update_interval,
        })
    }

    /// Start the fan daemon
    pub fn start(&self, profile: Profile) -> Result<()> {
        let mut running = self.running.lock().unwrap();
        if *running {
            return Ok(()); // Already running
        }
        *running = true;
        drop(running);

        // Store profile
        *self.current_profile.lock().unwrap() = Some(profile);

        // Spawn daemon thread
        let monitor = Arc::clone(&self.monitor);
        let controller = Arc::clone(&self.controller);
        let running = Arc::clone(&self.running);
        let current_profile = Arc::clone(&self.current_profile);
        let interval = self.update_interval;

        thread::spawn(move || {
            println!("Fan daemon started");
            
            loop {
                // Check if still running
                {
                    let is_running = running.lock().unwrap();
                    if !*is_running {
                        break;
                    }
                }

                // Get current temperatures
                let stats = {
                    let mut mon = monitor.lock().unwrap();
                    match mon.get_system_stats() {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Fan daemon: Failed to get stats: {}", e);
                            thread::sleep(interval);
                            continue;
                        }
                    }
                };

                // Get current profile
                let profile = {
                    let prof = current_profile.lock().unwrap();
                    match prof.as_ref() {
                        Some(p) => p.clone(),
                        None => {
                            thread::sleep(interval);
                            continue;
                        }
                    }
                };

                // Apply fan curves based on temperatures
                Self::apply_fan_curves_for_temps(&controller, &profile, &stats);

                thread::sleep(interval);
            }

            println!("Fan daemon stopped");
        });

        Ok(())
    }

    /// Stop the fan daemon
    pub fn stop(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
        println!("Stopping fan daemon...");
    }

    /// Update the active profile
    pub fn update_profile(&self, profile: Profile) {
        *self.current_profile.lock().unwrap() = Some(profile);
        println!("Fan daemon: Profile updated");
    }

    /// Check if daemon is running
    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    /// Apply fan curves based on current temperatures
    fn apply_fan_curves_for_temps(
        controller: &HardwareController,
        profile: &Profile,
        stats: &crate::hardware_monitor::SystemStats,
    ) {
        // Get CPU temperature
        let cpu_temp = stats.cpu.package_temp.unwrap_or(50.0);

        // Get GPU temperature (use first GPU if available)
        let gpu_temp = stats.gpus.first()
            .and_then(|gpu| gpu.temperature)
            .unwrap_or(50.0);

        // Apply each fan curve
        for (fan_id, curve) in &profile.fan_curves {
            // Determine which temperature to use for this fan
            let temp = if fan_id.contains("cpu") || fan_id == "fan1" {
                cpu_temp
            } else if fan_id.contains("gpu") || fan_id == "fan2" {
                gpu_temp
            } else {
                // Default to highest temp
                cpu_temp.max(gpu_temp)
            };

            // Calculate target speed from curve
            let target_speed = Self::calculate_fan_speed(curve, temp);

            // Apply speed (this would need hardware-specific implementation)
            if let Err(e) = Self::set_fan_speed(controller, fan_id, target_speed) {
                eprintln!("Failed to set {} speed: {}", fan_id, e);
            }
        }
    }

    /// Calculate fan speed from curve based on temperature
    fn calculate_fan_speed(curve: &FanCurve, temp: f32) -> u8 {
        let points = &curve.points;

        // If temp is below first point, use first speed
        if temp <= points[0].temp as f32 {
            return points[0].speed;
        }

        // If temp is above last point, use last speed
        if temp >= points[points.len() - 1].temp as f32 {
            return points[points.len() - 1].speed;
        }

        // Find the two points temp is between
        for i in 0..points.len() - 1 {
            let p1 = &points[i];
            let p2 = &points[i + 1];

            if temp >= p1.temp as f32 && temp <= p2.temp as f32 {
                // Linear interpolation between points
                let temp_range = (p2.temp - p1.temp) as f32;
                let speed_range = (p2.speed as i16 - p1.speed as i16) as f32;
                let temp_offset = temp - p1.temp as f32;
                
                let interpolated_speed = p1.speed as f32 + 
                    (speed_range * (temp_offset / temp_range));
                
                return interpolated_speed.round() as u8;
            }
        }

        // Fallback (shouldn't reach here)
        50
    }

    /// Set fan speed (hardware-specific implementation)
    fn set_fan_speed(
        _controller: &HardwareController,
        fan_id: &str,
        speed: u8,
    ) -> Result<()> {
        // Extract fan number
        let fan_num: usize = fan_id.trim_start_matches("fan")
            .parse()
            .unwrap_or(1);

        // Try tuxedo_io interface first
        let tuxedo_io_path = std::path::Path::new("/sys/devices/platform/tuxedo_io");
        if tuxedo_io_path.exists() {
            let speed_path = tuxedo_io_path.join(format!("fan{}_manual_speed", fan_num));
            if speed_path.exists() {
                std::fs::write(&speed_path, speed.to_string())
                    .context("Failed to write fan speed")?;
                return Ok(());
            }
        }

        // Try hwmon interface as fallback
        let hwmon_base = std::path::Path::new("/sys/class/hwmon");
        if hwmon_base.exists() {
            for entry in std::fs::read_dir(hwmon_base)? {
                let entry = entry?;
                let pwm_path = entry.path().join(format!("pwm{}", fan_num));
                
                if pwm_path.exists() {
                    // Convert percentage to PWM value (0-255)
                    let pwm_value = (speed as f32 * 2.55) as u8;
                    std::fs::write(&pwm_path, pwm_value.to_string())
                        .context("Failed to write PWM value")?;
                    return Ok(());
                }
            }
        }

        anyhow::bail!("No fan control interface available for {}", fan_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fan_speed_calculation() {
        let curve = FanCurve {
            points: vec![
                FanCurvePoint { temp: 40, speed: 30 },
                FanCurvePoint { temp: 50, speed: 40 },
                FanCurvePoint { temp: 60, speed: 60 },
                FanCurvePoint { temp: 70, speed: 80 },
                FanCurvePoint { temp: 75, speed: 90 },
                FanCurvePoint { temp: 80, speed: 95 },
                FanCurvePoint { temp: 85, speed: 100 },
                FanCurvePoint { temp: 90, speed: 100 },
            ],
        };

        // Test exact points
        assert_eq!(FanDaemon::calculate_fan_speed(&curve, 40.0), 30);
        assert_eq!(FanDaemon::calculate_fan_speed(&curve, 60.0), 60);
        assert_eq!(FanDaemon::calculate_fan_speed(&curve, 90.0), 100);

        // Test interpolation
        let speed_55 = FanDaemon::calculate_fan_speed(&curve, 55.0);
        assert!(speed_55 >= 40 && speed_55 <= 60);
        assert_eq!(speed_55, 50); // Should be halfway

        // Test below range
        assert_eq!(FanDaemon::calculate_fan_speed(&curve, 30.0), 30);

        // Test above range
        assert_eq!(FanDaemon::calculate_fan_speed(&curve, 95.0), 100);
    }
}
