use tailor_gui::keyboard_control::*;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    println!("=== Keyboard Backlight Demo ===\n");
    
    if !is_keyboard_backlight_available() {
        println!("❌ Keyboard backlight not available");
        return Ok(());
    }
    
    let kbd = KeyboardController::new()?;
    println!("✓ Controller initialized\n");
    
    let current_brightness = kbd.get_brightness()?;
    let (r, g, b) = kbd.get_color()?;
    
    println!("Current state:");
    println!("  Brightness: {}%", current_brightness);
    println!("  Color: RGB({}, {}, {})\n", r, g, b);
    
    // Brightness sweep
    println!("Demo: Brightness sweep");
    for brightness in (0..=100).step_by(20) {
        println!("  Setting brightness to {}%", brightness);
        kbd.set_brightness(brightness)?;
        thread::sleep(Duration::from_millis(500));
    }
    
    // Restore
    kbd.set_color_and_brightness(r, g, b, current_brightness)?;
    println!("\n✓ Demo completed!");
    
    Ok(())
}
