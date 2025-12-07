// src/hardware_monitor.rs
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CpuCoreInfo {
    pub core_id: usize,
    pub frequency_mhz: u32,
    pub load_percent: f32,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub cores: Vec<CpuCoreInfo>,
    pub package_temp: Option<f32>,
    pub package_power_watts: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpuType {
    Integrated,
    Discrete,
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub gpu_type: GpuType,
    pub frequency_mhz: Option<u32>,
    pub temperature: Option<f32>,
    pub load_percent: Option<f32>,
    pub power_watts: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct FanInfo {
    pub fan_id: String,
    pub name: String,
    pub speed_rpm: Option<u32>,
    pub speed_percent: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cpu: CpuInfo,
    pub gpus: Vec<GpuInfo>,
    pub fans: Vec<FanInfo>,
    pub active_gpu: GpuType,
}

pub struct HardwareMonitor {
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

impl HardwareMonitor {
    pub fn new() -> Result<Self> {
        let cpu_base_path = PathBuf::from("/sys/devices/system/cpu");
        let hwmon_paths = Self::discover_hwmon_paths()?;
        
        Ok(HardwareMonitor {
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
            cpu: self.get_cpu_info()?,
            gpus: self.get_gpu_info()?,
            fans: self.get_fan_info()?,
            active_gpu: self.get_active_gpu()?,
        })
    }
    
    fn get_cpu_info(&mut self) -> Result<CpuInfo> {
        let cpu_count = self.get_cpu_count()?;
        let mut cores = Vec::new();
        
        // Read new CPU stats
        let current_stats = self.read_cpu_stats()?;
        
        for core_id in 0..cpu_count {
            let frequency = self.read_cpu_frequency(core_id).unwrap_or(0);
            
            // Calculate load if we have previous stats
            let load = if let Some(ref last_stats) = self.last_cpu_stats {
                if core_id < last_stats.len() && core_id < current_stats.len() {
                    Self::calculate_cpu_load(&last_stats[core_id], &current_stats[core_id])
                } else {
                    0.0
                }
            } else {
                0.0
            };
            
            cores.push(CpuCoreInfo {
                core_id,
                frequency_mhz: frequency,
                load_percent: load,
                temperature: None, // Will be filled from hwmon
            });
        }
        
        // Update last stats
        self.last_cpu_stats = Some(current_stats);
        
        // Get temperatures from hwmon
        let temps = self.get_cpu_temperatures()?;
        for (core_id, temp) in temps {
            if let Some(core) = cores.get_mut(core_id) {
                core.temperature = Some(temp);
            }
        }
        
        Ok(CpuInfo {
            cores,
            package_temp: self.get_package_temperature()?,
            package_power_watts: self.get_cpu_power()?,
        })
    }
    
    fn get_cpu_count(&self) -> Result<usize> {
        let mut count = 0;
        
        while self.cpu_base_path.join(format!("cpu{}", count)).exists() {
            count += 1;
        }
        
        // Subtract 1 because cpu0 exists but we also have cpuidle, cpufreq, etc.
        if count > 0 {
            count -= 1; // Adjust for non-CPU entries
        }
        
        // More reliable method: check /proc/cpuinfo
        let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;
        let processor_count = cpuinfo.lines()
            .filter(|line| line.starts_with("processor"))
            .count();
        
        Ok(processor_count)
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
        
        Ok(freq_khz / 1000) // Convert to MHz
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
                
                // Look for CPU temperature sensors (coretemp, k10temp, zenpower)
                if name.contains("coretemp") || name.contains("k10temp") || 
                   name.contains("zenpower") {
                    // Try to read core temperatures
                    for i in 1..=32 {
                        let temp_label_path = hwmon_path.join(format!("temp{}_label", i));
                        let temp_input_path = hwmon_path.join(format!("temp{}_input", i));
                        
                        if temp_input_path.exists() {
                            if let Ok(label) = fs::read_to_string(&temp_label_path) {
                                let label = label.trim().to_lowercase();
                                
                                // Extract core number
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
                    
                    // Look for package temperature
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
        }
        
        Ok(None)
    }
    
    fn get_cpu_power(&self) -> Result<Option<f32>> {
        // Try to read from RAPL (Running Average Power Limit)
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
        let amd_power_path = Path::new("/sys/class/hwmon");
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
    
    fn get_gpu_info(&self) -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();
        
        // Detect AMD GPUs
        gpus.extend(self.detect_amd_gpus()?);
        
        // Detect Intel GPUs
        gpus.extend(self.detect_intel_gpus()?);
        
        // Detect NVIDIA GPUs
        gpus.extend(self.detect_nvidia_gpus()?);
        
        Ok(gpus)
    }
    
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
                    
                    // Check if it's an AMD GPU
                    if let Ok(vendor) = fs::read_to_string(device_path.join("vendor")) {
                        if vendor.trim() == "0x1002" { // AMD vendor ID
                            let gpu_name = self.read_gpu_name(&device_path)
                                .unwrap_or_else(|_| "AMD GPU".to_string());
                            
                            let gpu_type = if gpu_name.to_lowercase().contains("radeon") &&
                                            gpu_name.to_lowercase().contains("graphics") {
                                GpuType::Integrated
                            } else {
                                GpuType::Discrete
                            };
                            
                            gpus.push(GpuInfo {
                                name: gpu_name,
                                gpu_type,
                                frequency_mhz: self.read_amd_gpu_freq(&device_path).ok(),
                                temperature: self.read_amd_gpu_temp(&device_path).ok(),
                                load_percent: self.read_amd_gpu_load(&device_path).ok(),
                                power_watts: self.read_amd_gpu_power(&device_path).ok(),
                            });
                        }
                    }
                }
            }
        }
        
        Ok(gpus)
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
                        if vendor.trim() == "0x8086" { // Intel vendor ID
                            let gpu_name = self.read_gpu_name(&device_path)
                                .unwrap_or_else(|_| "Intel GPU".to_string());
                            
                            gpus.push(GpuInfo {
                                name: gpu_name,
                                gpu_type: GpuType::Integrated,
                                frequency_mhz: self.read_intel_gpu_freq(&device_path).ok(),
                                temperature: None,
                                load_percent: None,
                                power_watts: None,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(gpus)
    }
    
    fn detect_nvidia_gpus(&self) -> Result<Vec<GpuInfo>> {
        // NVIDIA GPU detection would require nvidia-smi or similar
        // This is a placeholder for future implementation
        Ok(Vec::new())
    }
    
    fn read_gpu_name(&self, device_path: &Path) -> Result<String> {
        // Try to read from uevent
        let uevent_path = device_path.join("uevent");
        if uevent_path.exists() {
            let content = fs::read_to_string(uevent_path)?;
            for line in content.lines() {
                if line.starts_with("PCI_ID=") {
                    // This is a simplified version
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
            
            // Find the active frequency (marked with *)
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
    
    fn read_intel_gpu_freq(&self, device_path: &Path) -> Result<u32> {
        // Intel GPU frequency reading - simplified
        Ok(0)
    }
    
    fn get_fan_info(&self) -> Result<Vec<FanInfo>> {
        let mut fans = Vec::new();
        
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
                        speed_percent: None, // Would need fan max to calculate
                    });
                }
            }
        }
        
        Ok(fans)
    }
    
    fn get_active_gpu(&self) -> Result<GpuType> {
        // Check prime-select status
        let prime_select_output = std::process::Command::new("prime-select")
            .arg("query")
            .output();
        
        if let Ok(output) = prime_select_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if stdout.contains("nvidia") {
                return Ok(GpuType::Discrete);
            } else if stdout.contains("intel") || stdout.contains("amd") {
                return Ok(GpuType::Integrated);
            }
        }
        
        // Fallback: assume integrated
        Ok(GpuType::Integrated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_monitor_creation() {
        // This test will only work on Linux systems with proper sysfs
        if cfg!(target_os = "linux") {
            let monitor = HardwareMonitor::new();
            // Don't assert success as it depends on system configuration
        }
    }
}
