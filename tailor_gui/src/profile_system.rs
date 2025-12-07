// src/profile_system.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RGBColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCurvePoint {
    pub temp: u8,      // Temperature in Celsius
    pub speed: u8,     // Fan speed percentage (0-100)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCurve {
    pub points: Vec<FanCurvePoint>, // Should have exactly 8 points
}

impl FanCurve {
    pub fn validate(&self) -> Result<()> {
        if self.points.len() != 8 {
            anyhow::bail!("Fan curve must have exactly 8 points");
        }
        
        // Check that temperatures are in ascending order
        for i in 1..self.points.len() {
            if self.points[i].temp <= self.points[i - 1].temp {
                anyhow::bail!("Fan curve temperatures must be in ascending order");
            }
        }
        
        // Validate ranges
        for point in &self.points {
            if point.speed > 100 {
                anyhow::bail!("Fan speed must be 0-100%");
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardBacklight {
    pub color: RGBColor,
    pub brightness: u8, // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CpuPerformanceProfile {
    PowerSave,
    Balanced,
    Performance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSettings {
    pub performance_profile: CpuPerformanceProfile,
    pub min_freq_mhz: Option<u32>,
    pub max_freq_mhz: Option<u32>,
    pub disable_boost: bool,
    pub smt_enabled: bool, // Hyperthreading/SMT
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSettings {
    pub brightness: u8, // 0-100
    pub auto_brightness: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub is_default: bool,
    
    // Hardware settings
    pub keyboard_backlight: KeyboardBacklight,
    pub fan_curves: HashMap<String, FanCurve>, // fan_id -> curve
    pub cpu_settings: CpuSettings,
    pub screen_settings: ScreenSettings,
    
    // Auto-switching rules
    pub auto_switch_enabled: bool,
    pub trigger_apps: Vec<String>, // App names/executables that trigger this profile
}

impl Profile {
    pub fn default_profile() -> Self {
        let mut fan_curves = HashMap::new();
        
        // Default fan curve with 8 points
        let default_curve = FanCurve {
            points: vec![
                FanCurvePoint { temp: 40, speed: 30 },
                FanCurvePoint { temp: 50, speed: 40 },
                FanCurvePoint { temp: 60, speed: 50 },
                FanCurvePoint { temp: 65, speed: 60 },
                FanCurvePoint { temp: 70, speed: 70 },
                FanCurvePoint { temp: 75, speed: 80 },
                FanCurvePoint { temp: 80, speed: 90 },
                FanCurvePoint { temp: 85, speed: 100 },
            ],
        };
        
        fan_curves.insert("fan1".to_string(), default_curve.clone());
        fan_curves.insert("fan2".to_string(), default_curve);
        
        Profile {
            name: "Default".to_string(),
            is_default: true,
            keyboard_backlight: KeyboardBacklight {
                color: RGBColor { r: 255, g: 255, b: 255 },
                brightness: 50,
            },
            fan_curves,
            cpu_settings: CpuSettings {
                performance_profile: CpuPerformanceProfile::Balanced,
                min_freq_mhz: None,
                max_freq_mhz: None,
                disable_boost: false,
                smt_enabled: true,
            },
            screen_settings: ScreenSettings {
                brightness: 70,
                auto_brightness: false,
            },
            auto_switch_enabled: false,
            trigger_apps: Vec::new(),
        }
    }
    
    pub fn validate(&self) -> Result<()> {
        // Validate fan curves
        for (fan_id, curve) in &self.fan_curves {
            curve.validate()
                .context(format!("Invalid fan curve for {}", fan_id))?;
        }
        
        // Validate brightness values
        if self.keyboard_backlight.brightness > 100 {
            anyhow::bail!("Keyboard brightness must be 0-100");
        }
        if self.screen_settings.brightness > 100 {
            anyhow::bail!("Screen brightness must be 0-100");
        }
        
        Ok(())
    }
}

pub struct ProfileManager {
    profiles: Vec<Profile>,
    active_profile_index: usize,
    config_dir: PathBuf,
}

impl ProfileManager {
    pub fn new() -> Result<Self> {
        let config_dir = Self::get_config_dir()?;
        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;
        
        let mut manager = ProfileManager {
            profiles: Vec::new(),
            active_profile_index: 0,
            config_dir,
        };
        
        manager.load_profiles()?;
        
        // Ensure at least one profile exists
        if manager.profiles.is_empty() {
            manager.profiles.push(Profile::default_profile());
            manager.save_profiles()?;
        }
        
        Ok(manager)
    }
    
    fn get_config_dir() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".config/tuxedo-control"))
    }
    
    fn profiles_file(&self) -> PathBuf {
        self.config_dir.join("profiles.json")
    }
    
    pub fn load_profiles(&mut self) -> Result<()> {
        let profiles_file = self.profiles_file();
        
        if !profiles_file.exists() {
            return Ok(());
        }
        
        let content = fs::read_to_string(&profiles_file)
            .context("Failed to read profiles file")?;
        
        self.profiles = serde_json::from_str(&content)
            .context("Failed to parse profiles")?;
        
        // Validate all profiles
        for profile in &self.profiles {
            profile.validate()
                .context(format!("Invalid profile: {}", profile.name))?;
        }
        
        Ok(())
    }
    
    pub fn save_profiles(&self) -> Result<()> {
        let profiles_file = self.profiles_file();
        let content = serde_json::to_string_pretty(&self.profiles)
            .context("Failed to serialize profiles")?;
        
        fs::write(&profiles_file, content)
            .context("Failed to write profiles file")?;
        
        Ok(())
    }
    
    pub fn add_profile(&mut self, mut profile: Profile) -> Result<()> {
        profile.validate()
            .context("Profile validation failed")?;
        
        // Ensure unique name
        if self.profiles.iter().any(|p| p.name == profile.name) {
            anyhow::bail!("Profile with name '{}' already exists", profile.name);
        }
        
        self.profiles.push(profile);
        self.save_profiles()?;
        Ok(())
    }
    
    pub fn update_profile(&mut self, index: usize, profile: Profile) -> Result<()> {
        if index >= self.profiles.len() {
            anyhow::bail!("Profile index out of bounds");
        }
        
        profile.validate()
            .context("Profile validation failed")?;
        
        self.profiles[index] = profile;
        self.save_profiles()?;
        Ok(())
    }
    
    pub fn delete_profile(&mut self, index: usize) -> Result<()> {
        if index >= self.profiles.len() {
            anyhow::bail!("Profile index out of bounds");
        }
        
        if self.profiles[index].is_default {
            anyhow::bail!("Cannot delete default profile");
        }
        
        self.profiles.remove(index);
        
        // Adjust active profile index if needed
        if self.active_profile_index >= self.profiles.len() {
            self.active_profile_index = 0;
        }
        
        self.save_profiles()?;
        Ok(())
    }
    
    pub fn set_active_profile(&mut self, index: usize) -> Result<()> {
        if index >= self.profiles.len() {
            anyhow::bail!("Profile index out of bounds");
        }
        
        self.active_profile_index = index;
        Ok(())
    }
    
    pub fn get_active_profile(&self) -> &Profile {
        &self.profiles[self.active_profile_index]
    }
    
    pub fn get_profiles(&self) -> &[Profile] {
        &self.profiles
    }
    
    pub fn find_profile_for_app(&self, app_name: &str) -> Option<usize> {
        self.profiles
            .iter()
            .enumerate()
            .find(|(_, profile)| {
                profile.auto_switch_enabled && 
                profile.trigger_apps.iter().any(|trigger| {
                    app_name.to_lowercase().contains(&trigger.to_lowercase())
                })
            })
            .map(|(index, _)| index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fan_curve_validation() {
        let mut curve = FanCurve {
            points: vec![
                FanCurvePoint { temp: 40, speed: 30 },
                FanCurvePoint { temp: 50, speed: 40 },
                FanCurvePoint { temp: 60, speed: 50 },
                FanCurvePoint { temp: 65, speed: 60 },
                FanCurvePoint { temp: 70, speed: 70 },
                FanCurvePoint { temp: 75, speed: 80 },
                FanCurvePoint { temp: 80, speed: 90 },
                FanCurvePoint { temp: 85, speed: 100 },
            ],
        };
        
        assert!(curve.validate().is_ok());
        
        // Test invalid number of points
        curve.points.pop();
        assert!(curve.validate().is_err());
    }
    
    #[test]
    fn test_profile_validation() {
        let profile = Profile::default_profile();
        assert!(profile.validate().is_ok());
    }
}
