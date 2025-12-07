// src/keyboard_control.rs
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Controller for Clevo RGB keyboard backlight
/// Interfaces with /sys/class/leds/rgb:kbd_backlight/
pub struct KeyboardController {
    base_path: PathBuf,
    max_brightness: u8,
}

impl KeyboardController {
    /// Create a new keyboard controller
    pub fn new() -> Result<Self> {
        let base_path = PathBuf::from("/sys/class/leds/rgb:kbd_backlight");
        
        if !base_path.exists() {
            anyhow::bail!(
                "Keyboard backlight interface not found at {}. \
                 Is the keyboard RGB driver loaded?",
                base_path.display()
            );
        }
        
        // Read max brightness
        let max_brightness = Self::read_max_brightness(&base_path)?;
        
        Ok(KeyboardController {
            base_path,
            max_brightness,
        })
    }
    
    /// Create controller with custom path (for testing)
    pub fn with_path(path: PathBuf) -> Result<Self> {
        let max_brightness = Self::read_max_brightness(&path)?;
        Ok(KeyboardController {
            base_path: path,
            max_brightness,
        })
    }
    
    fn read_max_brightness(path: &Path) -> Result<u8> {
        let max_path = path.join("max_brightness");
        let content = fs::read_to_string(&max_path)
            .context("Failed to read max_brightness")?;
        
        content.trim()
            .parse()
            .context("Failed to parse max_brightness")
    }
    
    /// Get current brightness (0-100%)
    pub fn get_brightness(&self) -> Result<u8> {
        let brightness_path = self.base_path.join("brightness");
        let content = fs::read_to_string(&brightness_path)
            .context("Failed to read brightness")?;
        
        let raw_brightness: u8 = content.trim()
            .parse()
            .context("Failed to parse brightness")?;
        
        // Convert from raw value to percentage
        let percentage = if self.max_brightness > 0 {
            ((raw_brightness as f32 / self.max_brightness as f32) * 100.0) as u8
        } else {
            0
        };
        
        Ok(percentage.min(100))
    }
    
    /// Set brightness (0-100%)
    pub fn set_brightness(&self, percentage: u8) -> Result<()> {
        if percentage > 100 {
            anyhow::bail!("Brightness percentage must be 0-100, got {}", percentage);
        }
        
        // Convert percentage to raw value
        let raw_value = ((percentage as f32 / 100.0) * self.max_brightness as f32) as u8;
        
        let brightness_path = self.base_path.join("brightness");
        fs::write(&brightness_path, raw_value.to_string())
            .context("Failed to write brightness")?;
        
        Ok(())
    }
    
    /// Get current RGB color
    pub fn get_color(&self) -> Result<(u8, u8, u8)> {
        let multi_intensity_path = self.base_path.join("multi_intensity");
        
        if !multi_intensity_path.exists() {
            anyhow::bail!("RGB color control not available (multi_intensity missing)");
        }
        
        let content = fs::read_to_string(&multi_intensity_path)
            .context("Failed to read multi_intensity")?;
        
        // Parse format: "R G B" (space-separated)
        let parts: Vec<&str> = content.trim().split_whitespace().collect();
        
        if parts.len() != 3 {
            anyhow::bail!("Invalid multi_intensity format: {}", content);
        }
        
        let r = parts[0].parse().context("Failed to parse red value")?;
        let g = parts[1].parse().context("Failed to parse green value")?;
        let b = parts[2].parse().context("Failed to parse blue value")?;
        
        Ok((r, g, b))
    }
    
    /// Set RGB color (0-255 per channel)
    pub fn set_color(&self, r: u8, g: u8, b: u8) -> Result<()> {
        let multi_intensity_path = self.base_path.join("multi_intensity");
        
        if !multi_intensity_path.exists() {
            anyhow::bail!("RGB color control not available (multi_intensity missing)");
        }
        
        let color_str = format!("{} {} {}", r, g, b);
        fs::write(&multi_intensity_path, color_str)
            .context("Failed to write multi_intensity")?;
        
        Ok(())
    }
    
    /// Set both color and brightness in one operation
    pub fn set_color_and_brightness(&self, r: u8, g: u8, b: u8, brightness: u8) -> Result<()> {
        self.set_color(r, g, b)?;
        self.set_brightness(brightness)?;
        Ok(())
    }
    
    /// Get the maximum brightness value supported by hardware
    pub fn max_brightness(&self) -> u8 {
        self.max_brightness
    }
    
    /// Check if RGB color control is available
    pub fn has_rgb_support(&self) -> bool {
        self.base_path.join("multi_intensity").exists()
    }
    
    /// Turn off keyboard backlight
    pub fn turn_off(&self) -> Result<()> {
        self.set_brightness(0)
    }
    
    /// Check if keyboard backlight is currently on
    pub fn is_on(&self) -> Result<bool> {
        Ok(self.get_brightness()? > 0)
    }
}

/// Helper function to check if keyboard backlight is available on the system
pub fn is_keyboard_backlight_available() -> bool {
    Path::new("/sys/class/leds/rgb:kbd_backlight").exists()
}

/// Get list of available LED devices (for debugging)
pub fn list_led_devices() -> Result<Vec<String>> {
    let leds_path = Path::new("/sys/class/leds");
    let mut devices = Vec::new();
    
    if !leds_path.exists() {
        return Ok(devices);
    }
    
    for entry in fs::read_dir(leds_path)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            devices.push(name.to_string());
        }
    }
    
    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_mock_keyboard_sysfs(temp_dir: &TempDir) -> PathBuf {
        let kbd_path = temp_dir.path().join("rgb:kbd_backlight");
        fs::create_dir_all(&kbd_path).unwrap();
        
        // Create mock files
        let mut max_brightness = fs::File::create(kbd_path.join("max_brightness")).unwrap();
        max_brightness.write_all(b"255").unwrap();
        
        let mut brightness = fs::File::create(kbd_path.join("brightness")).unwrap();
        brightness.write_all(b"128").unwrap();
        
        let mut multi_intensity = fs::File::create(kbd_path.join("multi_intensity")).unwrap();
        multi_intensity.write_all(b"255 255 255").unwrap();
        
        kbd_path
    }

    #[test]
    fn test_brightness_conversion() {
        let temp_dir = TempDir::new().unwrap();
        let kbd_path = create_mock_keyboard_sysfs(&temp_dir);
        let controller = KeyboardController::with_path(kbd_path).unwrap();
        
        // Test getting brightness
        let brightness = controller.get_brightness().unwrap();
        assert_eq!(brightness, 50); // 128/255 ≈ 50%
    }
    
    #[test]
    fn test_set_brightness() {
        let temp_dir = TempDir::new().unwrap();
        let kbd_path = create_mock_keyboard_sysfs(&temp_dir);
        let controller = KeyboardController::with_path(kbd_path).unwrap();
        
        // Set to 100%
        controller.set_brightness(100).unwrap();
        assert_eq!(controller.get_brightness().unwrap(), 100);
        
        // Set to 0%
        controller.set_brightness(0).unwrap();
        assert_eq!(controller.get_brightness().unwrap(), 0);
    }
    
    #[test]
    fn test_color_operations() {
        let temp_dir = TempDir::new().unwrap();
        let kbd_path = create_mock_keyboard_sysfs(&temp_dir);
        let controller = KeyboardController::with_path(kbd_path.clone()).unwrap();
        
        // Set red color
        controller.set_color(255, 0, 0).unwrap();
        let (r, g, b) = controller.get_color().unwrap();
        assert_eq!((r, g, b), (255, 0, 0));
        
        // Set blue color
        controller.set_color(0, 0, 255).unwrap();
        let (r, g, b) = controller.get_color().unwrap();
        assert_eq!((r, g, b), (0, 0, 255));
    }
    
    #[test]
    fn test_invalid_brightness() {
        let temp_dir = TempDir::new().unwrap();
        let kbd_path = create_mock_keyboard_sysfs(&temp_dir);
        let controller = KeyboardController::with_path(kbd_path).unwrap();
        
        // Should fail for brightness > 100
        assert!(controller.set_brightness(101).is_err());
    }
    
    #[test]
    fn test_rgb_support_check() {
        let temp_dir = TempDir::new().unwrap();
        let kbd_path = create_mock_keyboard_sysfs(&temp_dir);
        let controller = KeyboardController::with_path(kbd_path).unwrap();
        
        assert!(controller.has_rgb_support());
    }
}
