// src/improved_hardware_monitor.rs
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub product_name: String,
    pub manufacturer: String,
}

#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub name: String,
    pub median_frequency_mhz: u32,
    pub median_load_percent: f32,
    pub cores: Vec<CpuCoreInfo>,
    pub package_temp: Option<f32>,
    pub package_power_watts: Option<f32>,
    pub scheduler: String,
    pub profile: Option<String>, // From tuxedo-drivers
}

#[derive(Debug, Clone)]
pub struct CpuCoreInfo {
    pub core_id: usize,
    pub frequency_mhz: u32,
    pub load_percent: f32,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpuType {
    Integrated,
    Discrete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpuStatus {
    Active,
    Suspended,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub gpu_type: GpuType,
    pub status: GpuStatus,
    pub frequency_mhz: Option<u32>,
    pub temperature: Option<f32>,
    pub load_percent: Option<f32>,
    pub power_watts: Option<f32>,
    pub voltage_mv: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct FanInfo {
    pub fan_id: String,
    pub name: String,
    pub speed_rpm: Option<u32>,
    pub speed_percent: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct WifiInfo {
    pub name: String,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct NvmeInfo {
    pub name: String,
    pub model: String,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub present: bool,
    pub voltage_mv: Option<u32>,
    pub current_ma: Option<i32>,
    pub charge_percent: Option<u8>,
    pub capacity_mah: Option<u32>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub charge_start_threshold: Option<u8>,
    pub charge_end_threshold: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub system_info: SystemInfo,
    pub cpu: CpuInfo,
    pub gpus: Vec<GpuInfo>,
    pub fans: Vec<FanInfo>,
    pub active_gpu: Option<GpuType>,
    pub wifi: Vec<WifiInfo>,
    pub nvme: Vec<NvmeInfo>,
    pub battery: Option<BatteryInfo>,
}

pub struct ImprovedHardwareMonitor {
    cpu_base_path: PathBuf,
    hwmon_paths: Vec<PathBuf>,
    last_cpu_stats: Option<Vec<CpuStats>>,
}

#[derive(Clone)]
struct CpuStats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
}

impl ImprovedHardwareMonitor {
    pub fn new() -> Result<Self> {
        let cpu_base_path = PathBuf::from("/sys/devices/system/cpu");
        let hwmon_paths = Self::discover_hwmon_paths()?;
        
        Ok(ImprovedHardwareMonitor {
            cpu_base_path,
            hwmon_paths,
            last_cpu_stats: None,
        })
    }
    
    fn discover_hwmon_paths() -> Result<Vec<PathBuf>> {
        let hwmon_base = Path::new("/sys/class/hwmon");
        let mut paths = Vec::new();
        
        if !hwmon_base.exists() {
            return Ok(paths);
        }
        
        for entry in fs::read_dir(hwmon_base)? {
            let entry = entry?;
            paths.push(entry.path());
        }
        
        Ok(paths)
    }
    
    pub fn get_system_stats(&mut self) -> Result<SystemStats> {
        Ok(SystemStats {
            system_info: self.get_system_info()?,
            cpu: self.get_cpu_info()?,
            gpus: self.get_gpu_info()?,
            fans: self.get_fan_info()?,
            active_gpu: self.get_active_gpu()?,
            wifi: self.get_wifi_info()?,
            nvme: self.get_nvme_info()?,
            battery: self.get_battery_info().ok(),
        })
    }
    
    /// Get system information (product name, manufacturer)
    fn get_system_info(&self) -> Result<SystemInfo> {
        let dmi_path = Path::new("/sys/class/dmi/id");
        
        let product_name = fs::read_to_string(dmi_path.join("product_name"))
            .unwrap_or_else(|_| "Unknown".to_string())
            .trim()
            .to_string();
        
        let manufacturer = fs::read_to_string(dmi_path.join("sys_vendor"))
            .unwrap_or_else(|_| "Unknown".to_string())
            .trim()
            .to_string();
        
        Ok(SystemInfo {
            product_name,
            manufacturer,
        })
    }
    
    /// Get CPU information with median calculations
    fn get_cpu_info(&mut self) -> Result<CpuInfo> {
        let cpu_count = self.get_cpu_count()?;
        let current_stats = self.read_cpu_stats()?;
        let mut cores = Vec::new();
        
        for core_id in 0..cpu_count {
            let frequency = self.read_cpu_frequency(core_id).unwrap_or(0);
            
            let load = if let Some(ref last_stats) = self.last_cpu_stats {
                if core_id < last_stats.len() && core_id < current_stats.len() {
                    Self::calculate_cpu_load(&last_stats[core_id], &current_stats[core_id])
                } else {
                    0.0
                }
            }
        }
        
        Ok(None)
    }
    
    fn get_cpu_power(&self) -> Result<Option<f32>> {
        // Try RAPL (Running Average Power Limit)
        let rapl_path = Path::new("/sys/class/powercap/intel-rapl/intel-rapl:0");
        
        if rapl_path.exists() {
            let energy_path = rapl_path.join("energy_uj");
            if energy_path.exists() {
                // This would need to be calculated over time
                // For now, return None as it requires state tracking
                return Ok(None);
            }
        }
        
        // AMD alternative
        for hwmon_path in &self.hwmon_paths {
            let name_path = hwmon_path.join("name");
            if let Ok(name) = fs::read_to_string(&name_path) {
                if name.trim().contains("k10temp") || name.trim().contains("zenpower") {
                    let power_path = hwmon_path.join("power1_input");
                    if power_path.exists() {
                        if let Ok(power_str) = fs::read_to_string(&power_path) {
                            if let Ok(power_uw) = power_str.trim().parse::<u64>() {
                                return Ok(Some(power_uw as f32 / 1_000_000.0));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    fn read_gpu_name(&self, device_path: &Path) -> Result<String> {
        // Try to read from uevent
        let uevent_path = device_path.join("uevent");
        if uevent_path.exists() {
            let content = fs::read_to_string(uevent_path)?;
            for line in content.lines() {
                if line.starts_with("PCI_ID=") {
                    return Ok(line.replace("PCI_ID=", "GPU "));
                }
            }
        }
        
        Ok("Unknown GPU".to_string())
    }
    
    fn read_amd_gpu_freq(&self, device_path: &Path) -> Result<u32> {
        let freq_path = device_path.join("pp_dpm_sclk");
        if freq_path.exists() {
            let content = fs::read_to_string(freq_path)?;
            
            for line in content.lines() {
                if line.contains("*") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let freq_str = parts[1].trim_end_matches("Mhz");
                        return Ok(freq_str.parse()?);
                    }
                }
            }
        }
        
        anyhow::bail!("Could not read GPU frequency")
    }
    
    fn read_amd_gpu_temp(&self, device_path: &Path) -> Result<f32> {
        let hwmon_path = device_path.join("hwmon");
        
        if hwmon_path.exists() {
            for entry in fs::read_dir(hwmon_path)? {
                let entry = entry?;
                let temp_input = entry.path().join("temp1_input");
                
                if temp_input.exists() {
                    let temp_str = fs::read_to_string(temp_input)?;
                    let temp_millidegrees: i32 = temp_str.trim().parse()?;
                    return Ok(temp_millidegrees as f32 / 1000.0);
                }
            }
        }
        
        anyhow::bail!("Could not read GPU temperature")
    }
    
    fn read_amd_gpu_load(&self, device_path: &Path) -> Result<f32> {
        let load_path = device_path.join("gpu_busy_percent");
        
        if load_path.exists() {
            let load_str = fs::read_to_string(load_path)?;
            return Ok(load_str.trim().parse()?);
        }
        
        anyhow::bail!("Could not read GPU load")
    }
    
    fn read_amd_gpu_power(&self, device_path: &Path) -> Result<f32> {
        let hwmon_path = device_path.join("hwmon");
        
        if hwmon_path.exists() {
            for entry in fs::read_dir(hwmon_path)? {
                let entry = entry?;
                let power_input = entry.path().join("power1_average");
                
                if power_input.exists() {
                    let power_str = fs::read_to_string(power_input)?;
                    let power_uw: u64 = power_str.trim().parse()?;
                    return Ok(power_uw as f32 / 1_000_000.0);
                }
            }
        }
        
        anyhow::bail!("Could not read GPU power")
    }
    
    fn detect_intel_gpus(&self) -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();
        let drm_path = Path::new("/sys/class/drm");
        
        if !drm_path.exists() {
            return Ok(gpus);
        }
        
        for entry in fs::read_dir(drm_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("card") && !name.contains("-") {
                    let device_path = path.join("device");
                    
                    if let Ok(vendor) = fs::read_to_string(device_path.join("vendor")) {
                        if vendor.trim() == "0x8086" {
                            let gpu_name = self.read_gpu_name(&device_path)
                                .unwrap_or_else(|_| "Intel GPU".to_string());
                            
                            let status = self.read_gpu_status(&path)?;
                            
                            gpus.push(GpuInfo {
                                name: gpu_name,
                                gpu_type: GpuType::Integrated,
                                status,
                                frequency_mhz: None,
                                temperature: None,
                                load_percent: None,
                                power_watts: None,
                                voltage_mv: None,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(gpus)
    }
    
    fn get_fan_info(&self) -> Result<Vec<FanInfo>> {
        let mut fans = Vec::new();
        
        // Try tuxedo_io first
        let tuxedo_io = Path::new("/sys/devices/platform/tuxedo_io");
        if tuxedo_io.exists() {
            // Check for fan control files
            for i in 1..=10 {
                let fan_speed_path = tuxedo_io.join(format!("fan{}_speed", i));
                if fan_speed_path.exists() {
                    if let Ok(speed_str) = fs::read_to_string(&fan_speed_path) {
                        if let Ok(speed_percent) = speed_str.trim().parse::<u8>() {
                            fans.push(FanInfo {
                                fan_id: format!("fan{}", i),
                                name: format!("Fan {}", i),
                                speed_rpm: None,
                                speed_percent: Some(speed_percent),
                            });
                        }
                    }
                }
            }
        }
        
        // Fallback to hwmon
        if fans.is_empty() {
            for hwmon_path in &self.hwmon_paths {
                for i in 1..=10 {
                    let fan_input_path = hwmon_path.join(format!("fan{}_input", i));
                    
                    if fan_input_path.exists() {
                        let rpm = fs::read_to_string(&fan_input_path)
                            .ok()
                            .and_then(|s| s.trim().parse().ok());
                        
                        let label = fs::read_to_string(hwmon_path.join(format!("fan{}_label", i)))
                            .unwrap_or_else(|_| format!("Fan {}", i));
                        
                        fans.push(FanInfo {
                            fan_id: format!("fan{}", i),
                            name: label.trim().to_string(),
                            speed_rpm: rpm,
                            speed_percent: None,
                        });
                    }
                }
            }
        }
        
        Ok(fans)
    }
} else {
                0.0
            };
            
            cores.push(CpuCoreInfo {
                core_id,
                frequency_mhz: frequency,
                load_percent: load,
                temperature: None,
            });
        
        
        self.last_cpu_stats = Some(current_stats);
        
        // Get temperatures
        let temps = self.get_cpu_temperatures()?;
        for (core_id, temp) in temps {
            if let Some(core) = cores.get_mut(core_id) {
                core.temperature = Some(temp);
            }
        }
        
        // Calculate median frequency and load
        let mut frequencies: Vec<u32> = cores.iter().map(|c| c.frequency_mhz).collect();
        let mut loads: Vec<f32> = cores.iter().map(|c| c.load_percent).collect();
        
        frequencies.sort_unstable();
        loads.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let median_frequency = if frequencies.is_empty() {
            0
        } else {
            frequencies[frequencies.len() / 2]
        };
        
        let median_load = if loads.is_empty() {
            0.0
        } else {
            loads[loads.len() / 2]
        };
        
        Ok(CpuInfo {
            name: self.get_cpu_name()?,
            median_frequency_mhz: median_frequency,
            median_load_percent: median_load,
            cores,
            package_temp: self.get_package_temperature()?,
            package_power_watts: self.get_cpu_power()?,
            scheduler: self.get_cpu_scheduler()?,
            profile: self.get_cpu_profile()?,
        })
    
    
    /// Get CPU name from /proc/cpuinfo
    fn get_cpu_name(&self) -> Result<String> {
        let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;
        
        for line in cpuinfo.lines() {
            if line.starts_with("model name") {
                if let Some(name) = line.split(':').nth(1) {
                    return Ok(name.trim().to_string());
                }
            }
        }
        
        Ok("Unknown CPU".to_string())
    }
    
    /// Get CPU scheduler
    fn get_cpu_scheduler(&self) -> Result<String> {
        // Try to read from kernel command line
        if let Ok(cmdline) = fs::read_to_string("/proc/cmdline") {
            for part in cmdline.split_whitespace() {
                if part.starts_with("sched=") {
                    return Ok(part.replace("sched=", ""));
                }
            }
        }
        
        // Try to detect from sched_features
        if let Ok(features) = fs::read_to_string("/sys/kernel/debug/sched/features") {
            if features.contains("NEXT_BUDDY") {
                return Ok("CFS".to_string());
            }
        }
        
        // Default
        Ok("CFS".to_string())
    }
    
    /// Get CPU profile from tuxedo-drivers
    fn get_cpu_profile(&self) -> Option<String> {
        let tuxedo_io = Path::new("/sys/devices/platform/tuxedo_io");
        
        if tuxedo_io.exists() {
            if let Ok(profile) = fs::read_to_string(tuxedo_io.join("performance_profile")) {
                return Some(profile.trim().to_string());
            }
        }
        
        None
    }
    
    /// Get GPU information with proper status detection
    fn get_gpu_info(&self) -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();
        
        gpus.extend(self.detect_amd_gpus()?);
        gpus.extend(self.detect_intel_gpus()?);
        gpus.extend(self.detect_nvidia_gpus()?);
        
        Ok(gpus)
    }
    
    /// Detect AMD GPUs with voltage and status
    fn detect_amd_gpus(&self) -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();
        let drm_path = Path::new("/sys/class/drm");
        
        if !drm_path.exists() {
            return Ok(gpus);
        }
        
        for entry in fs::read_dir(drm_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("card") && !name.contains("-") {
                    let device_path = path.join("device");
                    
                    if let Ok(vendor) = fs::read_to_string(device_path.join("vendor")) {
                        if vendor.trim() == "0x1002" {
                            let gpu_name = self.read_gpu_name(&device_path)
                                .unwrap_or_else(|_| "AMD GPU".to_string());
                            
                            let gpu_type = if gpu_name.to_lowercase().contains("radeon") &&
                                            gpu_name.to_lowercase().contains("graphics") {
                                GpuType::Integrated
                            } else {
                                GpuType::Discrete
                            };
                            
                            let status = self.read_gpu_status(&path)?;
                            
                            gpus.push(GpuInfo {
                                name: gpu_name,
                                gpu_type,
                                status,
                                frequency_mhz: self.read_amd_gpu_freq(&device_path).ok(),
                                temperature: self.read_amd_gpu_temp(&device_path).ok(),
                                load_percent: self.read_amd_gpu_load(&device_path).ok(),
                                power_watts: self.read_amd_gpu_power(&device_path).ok(),
                                voltage_mv: self.read_amd_gpu_voltage(&device_path).ok(),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(gpus)
    }
    
    /// Read GPU status (active/suspended)
    fn read_gpu_status(&self, drm_path: &Path) -> Result<GpuStatus> {
        let device_path = drm_path.join("device");
        let power_status_path = device_path.join("power/runtime_status");
        
        if power_status_path.exists() {
            let status = fs::read_to_string(power_status_path)?;
            let status = status.trim();
            
            return Ok(match status {
                "active" => GpuStatus::Active,
                "suspended" => GpuStatus::Suspended,
                _ => GpuStatus::Unknown,
            });
        }
        
        Ok(GpuStatus::Unknown)
    }
    
    /// Read AMD GPU voltage
    fn read_amd_gpu_voltage(&self, device_path: &Path) -> Result<u32> {
        let hwmon_path = device_path.join("hwmon");
        
        if hwmon_path.exists() {
            for entry in fs::read_dir(hwmon_path)? {
                let entry = entry?;
                let in0_input = entry.path().join("in0_input");
                
                if in0_input.exists() {
                    let voltage_str = fs::read_to_string(in0_input)?;
                    let voltage_mv: u32 = voltage_str.trim().parse()?;
                    return Ok(voltage_mv);
                }
            }
        }
        
        anyhow::bail!("Could not read GPU voltage")
    }
    
    /// Detect NVIDIA GPUs
    fn detect_nvidia_gpus(&self) -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();
        let drm_path = Path::new("/sys/class/drm");
        
        if !drm_path.exists() {
            return Ok(gpus);
        }
        
        for entry in fs::read_dir(drm_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("card") && !name.contains("-") {
                    let device_path = path.join("device");
                    
                    if let Ok(vendor) = fs::read_to_string(device_path.join("vendor")) {
                        if vendor.trim() == "0x10de" { // NVIDIA
                            let gpu_name = self.read_gpu_name(&device_path)
                                .unwrap_or_else(|_| "NVIDIA GPU".to_string());
                            
                            let status = self.read_gpu_status(&path)?;
                            
                            gpus.push(GpuInfo {
                                name: gpu_name,
                                gpu_type: GpuType::Discrete,
                                status,
                                frequency_mhz: None,
                                temperature: None,
                                load_percent: None,
                                power_watts: None,
                                voltage_mv: None,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(gpus)
    }
    
    /// Get currently active GPU
    fn get_active_gpu(&self) -> Result<Option<GpuType>> {
        let drm_path = Path::new("/sys/class/drm");
        
        if !drm_path.exists() {
            return Ok(None);
        }
        
        // Check each GPU's status
        for entry in fs::read_dir(drm_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("card") && !name.contains("-") {
                    let device_path = path.join("device");
                    
                    // Check if GPU is active
                    let status = self.read_gpu_status(&path)?;
                    
                    if status == GpuStatus::Active {
                        // Determine GPU type
                        if let Ok(vendor) = fs::read_to_string(device_path.join("vendor")) {
                            let vendor = vendor.trim();
                            
                            // Intel or AMD iGPU
                            if vendor == "0x8086" {
                                return Ok(Some(GpuType::Integrated));
                            }
                            
                            // AMD - check if iGPU or dGPU
                            if vendor == "0x1002" {
                                if let Ok(name) = self.read_gpu_name(&device_path) {
                                    if name.to_lowercase().contains("radeon") &&
                                       name.to_lowercase().contains("graphics") {
                                        return Ok(Some(GpuType::Integrated));
                                    }
                                }
                                return Ok(Some(GpuType::Discrete));
                            }
                            
                            // NVIDIA - always discrete
                            if vendor == "0x10de" {
                                return Ok(Some(GpuType::Discrete));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Get WiFi information
    fn get_wifi_info(&self) -> Result<Vec<WifiInfo>> {
        let mut wifi_devices = Vec::new();
        let net_path = Path::new("/sys/class/net");
        
        if !net_path.exists() {
            return Ok(wifi_devices);
        }
        
        for entry in fs::read_dir(net_path)? {
            let entry = entry?;
            let path = entry.path();
            
            // Check if it's a wireless device
            if path.join("wireless").exists() {
                let name = entry.file_name().to_string_lossy().to_string();
                
                // Try to read temperature from hwmon
                let device_path = path.join("device");
                let temp = self.read_device_temperature(&device_path);
                
                wifi_devices.push(WifiInfo {
                    name,
                    temperature: temp,
                });
            }
        }
        
        Ok(wifi_devices)
    }
    
    /// Get NVMe information
    fn get_nvme_info(&self) -> Result<Vec<NvmeInfo>> {
        let mut nvme_devices = Vec::new();
        let block_path = Path::new("/sys/class/block");
        
        if !block_path.exists() {
            return Ok(nvme_devices);
        }
        
        for entry in fs::read_dir(block_path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            if name.starts_with("nvme") && !name.contains("n") {
                continue; // Skip partitions
            }
            
            if name.starts_with("nvme") {
                let device_path = entry.path().join("device");
                
                let model = fs::read_to_string(device_path.join("model"))
                    .unwrap_or_else(|_| "Unknown".to_string())
                    .trim()
                    .to_string();
                
                let temp = self.read_device_temperature(&device_path);
                
                nvme_devices.push(NvmeInfo {
                    name: name.clone(),
                    model,
                    temperature: temp,
                });
            }
        }
        
        Ok(nvme_devices)
    }
    
    /// Read device temperature from hwmon
    fn read_device_temperature(&self, device_path: &Path) -> Option<f32> {
        let hwmon_path = device_path.join("hwmon");
        
        if hwmon_path.exists() {
            for entry in fs::read_dir(hwmon_path).ok()? {
                let entry = entry.ok()?;
                let temp_input = entry.path().join("temp1_input");
                
                if temp_input.exists() {
                    if let Ok(temp_str) = fs::read_to_string(temp_input) {
                        if let Ok(temp_millidegrees) = temp_str.trim().parse::<i32>() {
                            return Some(temp_millidegrees as f32 / 1000.0);
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Get battery information
    fn get_battery_info(&self) -> Result<BatteryInfo> {
        let power_supply_path = Path::new("/sys/class/power_supply");
        
        for entry in fs::read_dir(power_supply_path)? {
            let entry = entry?;
            let path = entry.path();
            
            let type_path = path.join("type");
            if let Ok(device_type) = fs::read_to_string(&type_path) {
                if device_type.trim() == "Battery" {
                    return Ok(BatteryInfo {
                        present: true,
                        voltage_mv: Self::read_battery_value(&path, "voltage_now").ok(),
                        current_ma: Self::read_battery_value(&path, "current_now").ok(),
                        charge_percent: Self::read_battery_value::<u32>(&path, "capacity")
                            .ok()
                            .map(|v| v as u8),
                        capacity_mah: Self::read_battery_value(&path, "charge_full").ok(),
                        manufacturer: fs::read_to_string(path.join("manufacturer"))
                            .ok()
                            .map(|s| s.trim().to_string()),
                        model: fs::read_to_string(path.join("model_name"))
                            .ok()
                            .map(|s| s.trim().to_string()),
                        charge_start_threshold: Self::read_battery_threshold(&path, "start"),
                        charge_end_threshold: Self::read_battery_threshold(&path, "end"),
                    });
                }
            }
        }
        
        Ok(BatteryInfo {
            present: false,
            voltage_mv: None,
            current_ma: None,
            charge_percent: None,
            capacity_mah: None,
            manufacturer: None,
            model: None,
            charge_start_threshold: None,
            charge_end_threshold: None,
        })
    }
    
    fn read_battery_value<T: std::str::FromStr>(path: &Path, name: &str) -> Result<T> {
        let value_str = fs::read_to_string(path.join(name))?;
        value_str.trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Failed to parse battery value"))
    }
    
    fn read_battery_threshold(path: &Path, threshold_type: &str) -> Option<u8> {
        // Try tuxedo-drivers location
        let tuxedo_path = Path::new("/sys/devices/platform/tuxedo_io");
        if tuxedo_path.exists() {
            let threshold_path = tuxedo_path.join(format!("charge_control_{}_threshold", threshold_type));
            if let Ok(value) = fs::read_to_string(&threshold_path) {
                if let Ok(threshold) = value.trim().parse() {
                    return Some(threshold);
                }
            }
        }
        
        // Try standard location
        let threshold_path = path.join(format!("charge_control_{}_threshold", threshold_type));
        if let Ok(value) = fs::read_to_string(&threshold_path) {
            if let Ok(threshold) = value.trim().parse() {
                return Some(threshold);
            }
        }
        
        None
    }
    
    // ... (keep existing helper methods from original hardware_monitor.rs)
    fn get_cpu_count(&self) -> Result<usize> {
        let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;
        let count = cpuinfo.lines()
            .filter(|line| line.starts_with("processor"))
            .count();
        Ok(count)
    }
    
    fn read_cpu_frequency(&self, core_id: usize) -> Result<u32> {
        let freq_path = self.cpu_base_path
            .join(format!("cpu{}", core_id))
            .join("cpufreq/scaling_cur_freq");
        
        if !freq_path.exists() {
            anyhow::bail!("Frequency info not available");
        }
        
        let freq_khz: u32 = fs::read_to_string(freq_path)?
            .trim()
            .parse()
            .context("Failed to parse frequency")?;
        
        Ok(freq_khz / 1000)
    }
    
    fn read_cpu_stats(&self) -> Result<Vec<CpuStats>> {
        let stat_content = fs::read_to_string("/proc/stat")?;
        let mut stats = Vec::new();
        
        for line in stat_content.lines() {
            if line.starts_with("cpu") && !line.starts_with("cpu ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 8 {
                    stats.push(CpuStats {
                        user: parts[1].parse().unwrap_or(0),
                        nice: parts[2].parse().unwrap_or(0),
                        system: parts[3].parse().unwrap_or(0),
                        idle: parts[4].parse().unwrap_or(0),
                        iowait: parts[5].parse().unwrap_or(0),
                        irq: parts[6].parse().unwrap_or(0),
                        softirq: parts[7].parse().unwrap_or(0),
                    });
                }
            }
        }
        
        Ok(stats)
    }
    
    fn calculate_cpu_load(prev: &CpuStats, curr: &CpuStats) -> f32 {
        let prev_idle = prev.idle + prev.iowait;
        let curr_idle = curr.idle + curr.iowait;
        
        let prev_total = prev.user + prev.nice + prev.system + prev_idle + 
                        prev.irq + prev.softirq;
        let curr_total = curr.user + curr.nice + curr.system + curr_idle + 
                        curr.irq + curr.softirq;
        
        let total_diff = curr_total.saturating_sub(prev_total);
        let idle_diff = curr_idle.saturating_sub(prev_idle);
        
        if total_diff == 0 {
            return 0.0;
        }
        
        let usage = (total_diff - idle_diff) as f32 / total_diff as f32;
        (usage * 100.0).min(100.0).max(0.0)
    }
    
    fn get_cpu_temperatures(&self) -> Result<HashMap<usize, f32>> {
        let mut temps = HashMap::new();
        
        for hwmon_path in &self.hwmon_paths {
            let name_path = hwmon_path.join("name");
            if let Ok(name) = fs::read_to_string(&name_path) {
                let name = name.trim();
                
                if name.contains("coretemp") || name.contains("k10temp") || 
                   name.contains("zenpower") {
                    for i in 1..=32 {
                        let temp_label_path = hwmon_path.join(format!("temp{}_label", i));
                        let temp_input_path = hwmon_path.join(format!("temp{}_input", i));
                        
                        if temp_input_path.exists() {
                            if let Ok(label) = fs::read_to_string(&temp_label_path) {
                                let label = label.trim().to_lowercase();
                                
                                if label.contains("core") {
                                    if let Some(core_num) = label.split_whitespace()
                                        .find_map(|s| s.parse::<usize>().ok()) {
                                        
                                        if let Ok(temp_str) = fs::read_to_string(&temp_input_path) {
                                            if let Ok(temp_millidegrees) = temp_str.trim().parse::<i32>() {
                                                temps.insert(core_num, temp_millidegrees as f32 / 1000.0);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(temps)
    }
    
    fn get_package_temperature(&self) -> Result<Option<f32>> {
        for hwmon_path in &self.hwmon_paths {
            let name_path = hwmon_path.join("name");
            if let Ok(name) = fs::read_to_string(&name_path) {
                let name = name.trim();
                
                if name.contains("coretemp") || name.contains("k10temp") || 
                   name.contains("zenpower") {
                    
                    for i in 1..=32 {
                        let temp_label_path = hwmon_path.join(format!("temp{}_label", i));
                        let temp_input_path = hwmon_path.join(format!("temp{}_input", i));
                        
                        if temp_input_path.exists() {
                            if let Ok(label) = fs::read_to_string(&temp_label_path) {
                                let label = label.trim().to_lowercase();
                                
                                if label.contains("package") || label.contains("tdie") {
                                    if let Ok(temp_str) = fs::read_to_string(&temp_input_path) {
                                        if let Ok(temp) = temp_str.trim().parse::<i32>() {
                                            return Ok(Some(temp as f32 / 1000.0));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
