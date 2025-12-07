// examples/profile_application_demo.rs
//! Demonstrates applying profiles and controlling hardware

use tailor_gui::profile_controller::{ProfileController, ProfileBuilder};
use tailor_gui::profile_system::CpuPerformanceProfile;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    println!("=== Profile Application Demo ===\n");
    
    // Check permissions
    if !tailor_gui::hardware_control::check_permissions()? {
        eprintln!("⚠️  Warning: Not running as root. Hardware control may fail.");
        eprintln!("   Run with: sudo cargo run --example profile_application_demo\n");
    }
    
    // Initialize controller
    let controller = ProfileController::new()?;
    println!("✓ Profile controller initialized\n");
    
    // Show current profiles
    println!("Available profiles:");
    for (idx, profile) in controller.get_all_profiles().iter().enumerate() {
        println!("  [{}] {}", idx, profile.name);
    }
    println!();
    
    // Create a gaming profile
    println!("Creating gaming profile...");
    let gaming_profile = ProfileBuilder::new("Gaming Performance")
        .keyboard_color(255, 0, 0) // Red
        .keyboard_brightness(100)
        .cpu_performance(CpuPerformanceProfile::Performance)
        .cpu_frequency_limits(None, None) // No limits
        .disable_boost(false) // Enable boost
        .smt_enabled(true)
        .screen_brightness(100)
        .auto_switch_for_apps(vec![
            "steam".to_string(),
            "lutris".to_string(),
        ])
        .build();
    
    controller.add_profile(gaming_profile)?;
    println!("✓ Gaming profile created\n");
    
    // Create a power-saving profile
    println!("Creating power-saving profile...");
    let powersave_profile = ProfileBuilder::new("Power Saver")
        .keyboard_color(0, 128, 255) // Blue
        .keyboard_brightness(30)
        .cpu_performance(CpuPerformanceProfile::PowerSave)
        .cpu_frequency_limits(Some(800), Some(2000)) // Limit to 800-2000 MHz
        .screen_brightness(40)
        .build();
    
    controller.add_profile(powersave_profile)?;
    println!("✓ Power-saving profile created\n");
    
    // Test applying profiles
    println!("Testing profile application...\n");
    
    // Apply gaming profile
    println!("Applying Gaming Performance profile...");
    controller.apply_profile_by_name("Gaming Performance")?;
    println!("✓ Profile applied");
    thread::sleep(Duration::from_secs(3));
    
    // Show hardware stats
    println!("\nCurrent hardware stats:");
    if let Ok(stats) = controller.get_hardware_stats() {
        println!("  CPU Package Temp: {:.1}°C", 
                 stats.cpu.package_temp.unwrap_or(0.0));
        
        let avg_freq: u32 = stats.cpu.cores.iter()
            .map(|c| c.frequency_mhz)
            .sum::<u32>() / stats.cpu.cores.len() as u32;
        
        println!("  CPU Avg Frequency: {} MHz", avg_freq);
        
        for gpu in &stats.gpus {
            if let Some(temp) = gpu.temperature {
                println!("  GPU Temp: {:.1}°C", temp);
            }
        }
    }
    
    println!("\nWaiting 5 seconds...");
    thread::sleep(Duration::from_secs(5));
    
    // Apply power-saving profile
    println!("\nApplying Power Saver profile...");
    controller.apply_profile_by_name("Power Saver")?;
    println!("✓ Profile applied");
    thread::sleep(Duration::from_secs(3));
    
    // Show stats again
    println!("\nCurrent hardware stats:");
    if let Ok(stats) = controller.get_hardware_stats() {
        let avg_freq: u32 = stats.cpu.cores.iter()
            .map(|c| c.frequency_mhz)
            .sum::<u32>() / stats.cpu.cores.len() as u32;
        
        println!("  CPU Avg Frequency: {} MHz (should be lower now)", avg_freq);
    }
    
    // Restore default profile
    println!("\nRestoring default profile...");
    controller.apply_profile(0)?;
    println!("✓ Default profile restored\n");
    
    println!("Demo completed successfully!");
    
    Ok(())
}
