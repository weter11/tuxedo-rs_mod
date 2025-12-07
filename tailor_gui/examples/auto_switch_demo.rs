// examples/auto_switch_demo.rs
//! Demonstrates automatic profile switching based on running applications

use tailor_gui::profile_controller::{ProfileController, ProfileBuilder};
use tailor_gui::profile_system::CpuPerformanceProfile;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    println!("=== Automatic Profile Switching Demo ===\n");
    
    // Check permissions
    if !tailor_gui::hardware_control::check_permissions()? {
        eprintln!("⚠️  Warning: Not running as root. Hardware control may fail.");
        eprintln!("   Run with: sudo cargo run --example auto_switch_demo\n");
    }
    
    // Initialize controller
    let controller = ProfileController::new()?;
    println!("✓ Profile controller initialized\n");
    
    // Create a gaming profile with auto-switch
    println!("Creating auto-switch gaming profile...");
    let gaming_profile = ProfileBuilder::new("Auto-Gaming")
        .keyboard_color(255, 0, 0) // Red
        .keyboard_brightness(100)
        .cpu_performance(CpuPerformanceProfile::Performance)
        .screen_brightness(100)
        .auto_switch_for_apps(vec![
            "steam".to_string(),
            "lutris".to_string(),
            "gamemode".to_string(),
        ])
        .build();
    
    controller.add_profile(gaming_profile)?;
    println!("✓ Gaming profile created with auto-switch enabled");
    println!("  Triggers: steam, lutris, gamemode\n");
    
    // Start application monitoring
    println!("Starting application monitoring...");
    controller.start_app_monitoring()?;
    println!("✓ Monitoring active\n");
    
    println!("Instructions:");
    println!("  1. Launch Steam or Lutris");
    println!("  2. The profile should automatically switch to 'Auto-Gaming'");
    println!("  3. Close the application");
    println!("  4. The profile should switch back to default");
    println!();
    println!("Monitoring for 60 seconds...");
    println!("Press Ctrl+C to stop early\n");
    
    // Monitor for 60 seconds
    for i in 0..12 {
        thread::sleep(Duration::from_secs(5));
        
        let active_profile = controller.get_active_profile();
        println!("[{}s] Active profile: {}", (i + 1) * 5, active_profile.name);
        
        // Show current hardware stats
        if let Ok(stats) = controller.get_hardware_stats() {
            let avg_freq: u32 = stats.cpu.cores.iter()
                .map(|c| c.frequency_mhz)
                .sum::<u32>() / stats.cpu.cores.len() as u32;
            
            println!("      CPU Avg Freq: {} MHz", avg_freq);
        }
    }
    
    // Stop monitoring
    println!("\nStopping application monitoring...");
    controller.stop_app_monitoring();
    println!("✓ Monitoring stopped\n");
    
    println!("Demo completed!");
    
    Ok(())
}
