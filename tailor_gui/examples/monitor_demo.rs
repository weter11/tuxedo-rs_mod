use tailor_gui::hardware_monitor::HardwareMonitor;
use tailor_gui::profile_system::ProfileManager;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    println!("=== Tuxedo Control - Phase 1 Demo ===\n");
    
    // Initialize components
    let mut monitor = HardwareMonitor::new()?;
    let profile_mgr = ProfileManager::new()?;
    
    println!("Profile Manager initialized with {} profiles", 
             profile_mgr.get_profiles().len());
    println!("Active profile: {}\n", 
             profile_mgr.get_active_profile().name);
    
    // Monitor loop
    for i in 0..5 {
        println!("--- Sample {} ---", i + 1);
        
        let stats = monitor.get_system_stats()?;
        
        // CPU Info
        println!("CPU:");
        println!("  Package Temp: {:.1}°C", 
                 stats.cpu.package_temp.unwrap_or(0.0));
        
        for core in stats.cpu.cores.iter().take(4) {
            println!("  Core {}: {} MHz, {:.1}% load", 
                     core.core_id, core.frequency_mhz, core.load_percent);
        }
        
        // GPU Info
        println!("\nGPUs:");
        for gpu in &stats.gpus {
            println!("  {}: {:?}", gpu.name, gpu.gpu_type);
            if let Some(temp) = gpu.temperature {
                println!("    Temp: {:.1}°C", temp);
            }
        }
        
        println!("  Active GPU: {:?}\n", stats.active_gpu);
        
        thread::sleep(Duration::from_secs(2));
    }
    
    Ok(())
}
