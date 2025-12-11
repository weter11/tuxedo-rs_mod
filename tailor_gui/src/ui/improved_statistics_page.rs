// src/ui/improved_statistics_page.rs
use gtk::prelude::*;
use gtk::{Box, Label, Orientation, Grid, Frame, Button, Expander};
use adw::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::improved_hardware_monitor::{ImprovedHardwareMonitor, SystemStats, GpuStatus};

pub struct ImprovedStatisticsPage {
    pub widget: Box,
    monitor: Arc<Mutex<ImprovedHardwareMonitor>>,
    
    // System info
    system_info_label: Label,
    
    // CPU widgets
    cpu_name_label: Label,
    cpu_median_freq_label: Label,
    cpu_median_load_label: Label,
    cpu_package_temp_label: Label,
    cpu_package_power_label: Label,
    cpu_scheduler_label: Label,
    cpu_profile_label: Label,
    cpu_details_expander: Expander,
    cpu_cores_grid: Grid,
    
    // GPU widgets
    gpu_container: Box,
    
    // Battery widgets
    battery_container: Box,
    
    // WiFi widgets
    wifi_container: Box,
    
    // NVMe widgets
    nvme_container: Box,
    
    // Fan widgets
    fan_container: Box,
}

impl ImprovedStatisticsPage {
    pub fn new(monitor: Arc<Mutex<ImprovedHardwareMonitor>>) -> Self {
        let main_box = Box::new(Orientation::Vertical, 12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);

        // System Info at top
        let system_info_label = Label::new(Some("Loading system information..."));
        system_info_label.add_css_class("title-2");
        system_info_label.set_halign(gtk::Align::Start);
        main_box.append(&system_info_label);

        // CPU Section
        let cpu_frame = Frame::new(Some("CPU Information"));
        let cpu_box = Box::new(Orientation::Vertical, 8);
        cpu_box.set_margin_top(8);
        cpu_box.set_margin_bottom(8);
        cpu_box.set_margin_start(8);
        cpu_box.set_margin_end(8);

        let cpu_name_label = Label::new(Some("CPU: Loading..."));
        cpu_name_label.set_halign(gtk::Align::Start);
        cpu_box.append(&cpu_name_label);

        // Summary info
        let cpu_summary_box = Box::new(Orientation::Horizontal, 12);
        let cpu_median_freq_label = Label::new(Some("Freq: --"));
        let cpu_median_load_label = Label::new(Some("Load: --"));
        let cpu_package_temp_label = Label::new(Some("Temp: --"));
        let cpu_package_power_label = Label::new(Some("Power: --"));
        
        cpu_summary_box.append(&cpu_median_freq_label);
        cpu_summary_box.append(&cpu_median_load_label);
        cpu_summary_box.append(&cpu_package_temp_label);
        cpu_summary_box.append(&cpu_package_power_label);
        cpu_box.append(&cpu_summary_box);

        // Scheduler and Profile
        let cpu_info_box = Box::new(Orientation::Horizontal, 12);
        let cpu_scheduler_label = Label::new(Some("Scheduler: --"));
        let cpu_profile_label = Label::new(Some("Profile: --"));
        cpu_info_box.append(&cpu_scheduler_label);
        cpu_info_box.append(&cpu_profile_label);
        cpu_box.append(&cpu_info_box);

        // Expandable per-core details
        let cpu_details_expander = Expander::new(Some("Show per-core details"));
        let cpu_cores_grid = Grid::new();
        cpu_cores_grid.set_row_spacing(6);
        cpu_cores_grid.set_column_spacing(12);
        cpu_cores_grid.set_margin_top(8);
        
        // Headers
        let header_core = Label::new(Some("Core"));
        header_core.add_css_class("heading");
        let header_freq = Label::new(Some("Frequency"));
        header_freq.add_css_class("heading");
        let header_temp = Label::new(Some("Temp"));
        header_temp.add_css_class("heading");
        let header_load = Label::new(Some("Load"));
        header_load.add_css_class("heading");

        cpu_cores_grid.attach(&header_core, 0, 0, 1, 1);
        cpu_cores_grid.attach(&header_freq, 1, 0, 1, 1);
        cpu_cores_grid.attach(&header_temp, 2, 0, 1, 1);
        cpu_cores_grid.attach(&header_load, 3, 0, 1, 1);

        cpu_details_expander.set_child(Some(&cpu_cores_grid));
        cpu_box.append(&cpu_details_expander);

        cpu_frame.set_child(Some(&cpu_box));
        main_box.append(&cpu_frame);

        // GPU Section
        let gpu_frame = Frame::new(Some("GPU Information"));
        let gpu_container = Box::new(Orientation::Vertical, 8);
        gpu_container.set_margin_top(8);
        gpu_container.set_margin_bottom(8);
        gpu_container.set_margin_start(8);
        gpu_container.set_margin_end(8);
        gpu_frame.set_child(Some(&gpu_container));
        main_box.append(&gpu_frame);

        // Battery Section
        let battery_frame = Frame::new(Some("Battery Information"));
        let battery_container = Box::new(Orientation::Vertical, 8);
        battery_container.set_margin_top(8);
        battery_container.set_margin_bottom(8);
        battery_container.set_margin_start(8);
        battery_container.set_margin_end(8);
        battery_frame.set_child(Some(&battery_container));
        main_box.append(&battery_frame);

        // WiFi Section
        let wifi_frame = Frame::new(Some("WiFi Information"));
        let wifi_container = Box::new(Orientation::Vertical, 8);
        wifi_container.set_margin_top(8);
        wifi_container.set_margin_bottom(8);
        wifi_container.set_margin_start(8);
        wifi_container.set_margin_end(8);
        wifi_frame.set_child(Some(&wifi_container));
        main_box.append(&wifi_frame);

        // NVMe Section
        let nvme_frame = Frame::new(Some("Storage Information"));
        let nvme_container = Box::new(Orientation::Vertical, 8);
        nvme_container.set_margin_top(8);
        nvme_container.set_margin_bottom(8);
        nvme_container.set_margin_start(8);
        nvme_container.set_margin_end(8);
        nvme_frame.set_child(Some(&nvme_container));
        main_box.append(&nvme_frame);

        // Fan Section
        let fan_frame = Frame::new(Some("Fan Information"));
        let fan_container = Box::new(Orientation::Vertical, 8);
        fan_container.set_margin_top(8);
        fan_container.set_margin_bottom(8);
        fan_container.set_margin_start(8);
        fan_container.set_margin_end(8);
        fan_frame.set_child(Some(&fan_container));
        main_box.append(&fan_frame);

        let page = ImprovedStatisticsPage {
            widget: main_box,
            monitor: monitor.clone(),
            system_info_label,
            cpu_name_label,
            cpu_median_freq_label,
            cpu_median_load_label,
            cpu_package_temp_label,
            cpu_package_power_label,
            cpu_scheduler_label,
            cpu_profile_label,
            cpu_details_expander,
            cpu_cores_grid,
            gpu_container,
            battery_container,
            wifi_container,
            nvme_container,
            fan_container,
        };

        page.start_update_loop();
        page
    }

    fn update_display(&self, stats: &SystemStats) {
        // Update system info
        self.system_info_label.set_markup(&format!(
            "<b>{}</b> by {}",
            stats.system_info.product_name,
            stats.system_info.manufacturer
        ));

        // Update CPU info
        self.cpu_name_label.set_text(&format!("CPU: {}", stats.cpu.name));
        
        self.cpu_median_freq_label.set_text(&format!(
            "Freq: {} MHz",
            stats.cpu.median_frequency_mhz
        ));
        
        self.cpu_median_load_label.set_text(&format!(
            "Load: {:.1}%",
            stats.cpu.median_load_percent
        ));
        
        if let Some(temp) = stats.cpu.package_temp {
            self.cpu_package_temp_label.set_text(&format!("Temp: {:.1}°C", temp));
        } else {
            self.cpu_package_temp_label.set_text("Temp: N/A");
        }
        
        if let Some(power) = stats.cpu.package_power_watts {
            self.cpu_package_power_label.set_text(&format!("Power: {:.2}W", power));
        } else {
            self.cpu_package_power_label.set_text("Power: N/A");
        }

        self.cpu_scheduler_label.set_text(&format!("Scheduler: {}", stats.cpu.scheduler));
        
        if let Some(ref profile) = stats.cpu.profile {
            self.cpu_profile_label.set_text(&format!("Profile: {}", profile));
        } else {
            self.cpu_profile_label.set_text("Profile: N/A");
        }

        // Update per-core details (only when expanded)
        self.update_cpu_cores(&stats.cpu.cores);

        // Update GPU info
        self.update_gpu_display(stats);

        // Update battery info
        self.update_battery_display(stats);

        // Update WiFi info
        self.update_wifi_display(stats);

        // Update NVMe info
        self.update_nvme_display(stats);

        // Update fan info
        self.update_fan_display(stats);
    }

    fn update_cpu_cores(&self, cores: &[crate::improved_hardware_monitor::CpuCoreInfo]) {
        // Clear existing rows (except header)
        let mut child = self.cpu_cores_grid.first_child();
        let mut row = 0;
        
        while let Some(widget) = child {
            let next = widget.next_sibling();
            if row > 0 { // Keep header row
                self.cpu_cores_grid.remove(&widget);
            }
            child = next;
            row += 1;
        }

        // Add core data
        for (i, core) in cores.iter().enumerate() {
            let core_label = Label::new(Some(&format!("Core {}", core.core_id)));
            let freq_label = Label::new(Some(&format!("{} MHz", core.frequency_mhz)));
            
            let temp_text = if let Some(temp) = core.temperature {
                format!("{:.1}°C", temp)
            } else {
                "N/A".to_string()
            };
            let temp_label = Label::new(Some(&temp_text));
            
            let load_label = Label::new(Some(&format!("{:.1}%", core.load_percent)));

            self.cpu_cores_grid.attach(&core_label, 0, (i + 1) as i32, 1, 1);
            self.cpu_cores_grid.attach(&freq_label, 1, (i + 1) as i32, 1, 1);
            self.cpu_cores_grid.attach(&temp_label, 2, (i + 1) as i32, 1, 1);
            self.cpu_cores_grid.attach(&load_label, 3, (i + 1) as i32, 1, 1);
        }
    }

    fn update_gpu_display(&self, stats: &SystemStats) {
        // Clear old widgets
        while let Some(child) = self.gpu_container.first_child() {
            self.gpu_container.remove(&child);
        }

        // Active GPU indicator
        if let Some(ref active_gpu) = stats.active_gpu {
            let active_label = Label::new(Some(&format!("Active GPU: {:?}", active_gpu)));
            active_label.add_css_class("heading");
            active_label.set_halign(gtk::Align::Start);
            self.gpu_container.append(&active_label);
        }

        // Individual GPU info
        for gpu in &stats.gpus {
            let gpu_box = Box::new(Orientation::Vertical, 4);
            gpu_box.set_margin_top(4);

            let name_label = Label::new(Some(&format!(
                "{} ({:?}) - {:?}",
                gpu.name, gpu.gpu_type, gpu.status
            )));
            name_label.set_halign(gtk::Align::Start);
            name_label.add_css_class("heading");

            let info_box = Box::new(Orientation::Horizontal, 12);
            
            let freq_text = gpu.frequency_mhz
                .map(|f| format!("Freq: {} MHz", f))
                .unwrap_or_else(|| "Freq: N/A".to_string());
            info_box.append(&Label::new(Some(&freq_text)));
            
            let temp_text = gpu.temperature
                .map(|t| format!("Temp: {:.1}°C", t))
                .unwrap_or_else(|| "Temp: N/A".to_string());
            info_box.append(&Label::new(Some(&temp_text)));
            
            let load_text = gpu.load_percent
                .map(|l| format!("Load: {:.1}%", l))
                .unwrap_or_else(|| "Load: N/A".to_string());
            info_box.append(&Label::new(Some(&load_text)));
            
            let power_text = gpu.power_watts
                .map(|p| format!("Power: {:.2}W", p))
                .unwrap_or_else(|| "Power: N/A".to_string());
            info_box.append(&Label::new(Some(&power_text)));
            
            let voltage_text = gpu.voltage_mv
                .map(|v| format!("Voltage: {}mV", v))
                .unwrap_or_else(|| "Voltage: N/A".to_string());
            info_box.append(&Label::new(Some(&voltage_text)));

            gpu_box.append(&name_label);
            gpu_box.append(&info_box);
            self.gpu_container.append(&gpu_box);
        }

        if stats.gpus.is_empty() {
            self.gpu_container.append(&Label::new(Some("No GPU information available")));
        }
    }

    fn update_battery_display(&self, stats: &SystemStats) {
        // Clear old widgets
        while let Some(child) = self.battery_container.first_child() {
            self.battery_container.remove(&child);
        }

        if let Some(ref battery) = stats.battery {
            if !battery.present {
                self.battery_container.append(&Label::new(Some("No battery detected")));
                return;
            }

            // Battery info grid
            let grid = Grid::new();
            grid.set_row_spacing(4);
            grid.set_column_spacing(12);

            let mut row = 0;

            if let Some(charge) = battery.charge_percent {
                grid.attach(&Label::new(Some("Charge:")), 0, row, 1, 1);
                grid.attach(&Label::new(Some(&format!("{}%", charge))), 1, row, 1, 1);
                row += 1;
            }

            if let Some(voltage) = battery.voltage_mv {
                grid.attach(&Label::new(Some("Voltage:")), 0, row, 1, 1);
                grid.attach(&Label::new(Some(&format!("{}mV", voltage))), 1, row, 1, 1);
                row += 1;
            }

            if let Some(current) = battery.current_ma {
                grid.attach(&Label::new(Some("Current:")), 0, row, 1, 1);
                grid.attach(&Label::new(Some(&format!("{}mA", current))), 1, row, 1, 1);
                row += 1;
            }

            if let Some(capacity) = battery.capacity_mah {
                grid.attach(&Label::new(Some("Capacity:")), 0, row, 1, 1);
                grid.attach(&Label::new(Some(&format!("{}mAh", capacity))), 1, row, 1, 1);
                row += 1;
            }

            if let Some(manufacturer) = &battery.manufacturer {
    grid.attach(&Label::new(Some("Manufacturer:")), 0, row, 1, 1);
    grid.attach(&Label::new(Some(manufacturer.as_str())), 1, row, 1, 1);
    row += 1;
}

if let Some(model) = &battery.model {
    grid.attach(&Label::new(Some("Model:")), 0, row, 1, 1);
    grid.attach(&Label::new(Some(model.as_str())), 1, row, 1, 1);
    row += 1;
}

            if let (Some(start), Some(end)) = (battery.charge_start_threshold, battery.charge_end_threshold) {
                grid.attach(&Label::new(Some("Charge Thresholds:")), 0, row, 1, 1);
                grid.attach(&Label::new(Some(&format!("{}% - {}%", start, end))), 1, row, 1, 1);
            }

            self.battery_container.append(&grid);
        } else {
            self.battery_container.append(&Label::new(Some("No battery information available")));
        }
    }

    fn update_wifi_display(&self, stats: &SystemStats) {
        // Clear old widgets
        while let Some(child) = self.wifi_container.first_child() {
            self.wifi_container.remove(&child);
        }

        if stats.wifi.is_empty() {
            self.wifi_container.append(&Label::new(Some("No WiFi devices detected")));
            return;
        }

        for wifi in &stats.wifi {
            let wifi_box = Box::new(Orientation::Horizontal, 12);
            wifi_box.append(&Label::new(Some(&format!("Device: {}", wifi.name))));
            
            if let Some(temp) = wifi.temperature {
                wifi_box.append(&Label::new(Some(&format!("Temp: {:.1}°C", temp))));
            } else {
                wifi_box.append(&Label::new(Some("Temp: N/A")));
            }
            
            self.wifi_container.append(&wifi_box);
        }
    }

    fn update_nvme_display(&self, stats: &SystemStats) {
        // Clear old widgets
        while let Some(child) = self.nvme_container.first_child() {
            self.nvme_container.remove(&child);
        }

        if stats.nvme.is_empty() {
            self.nvme_container.append(&Label::new(Some("No NVMe devices detected")));
            return;
        }

        for nvme in &stats.nvme {
            let nvme_box = Box::new(Orientation::Vertical, 4);
            
            let name_label = Label::new(Some(&format!("{}: {}", nvme.name, nvme.model)));
            name_label.set_halign(gtk::Align::Start);
            
            let temp_text = nvme.temperature
                .map(|t| format!("Temperature: {:.1}°C", t))
                .unwrap_or_else(|| "Temperature: N/A".to_string());
            let temp_label = Label::new(Some(&temp_text));
            temp_label.set_halign(gtk::Align::Start);
            
            nvme_box.append(&name_label);
            nvme_box.append(&temp_label);
            self.nvme_container.append(&nvme_box);
        }
    }

    fn update_fan_display(&self, stats: &SystemStats) {
        // Clear old widgets
        while let Some(child) = self.fan_container.first_child() {
            self.fan_container.remove(&child);
        }

        if stats.fans.is_empty() {
            self.fan_container.append(&Label::new(Some("No fan information available")));
            return;
        }

        for fan in &stats.fans {
            let fan_box = Box::new(Orientation::Horizontal, 12);
            fan_box.append(&Label::new(Some(&format!("{}", fan.name))));
            
            if let Some(rpm) = fan.speed_rpm {
                fan_box.append(&Label::new(Some(&format!("{} RPM", rpm))));
            }
            
            if let Some(percent) = fan.speed_percent {
                fan_box.append(&Label::new(Some(&format!("{}%", percent))));
            }
            
            if fan.speed_rpm.is_none() && fan.speed_percent.is_none() {
                fan_box.append(&Label::new(Some("N/A")));
            }
            
            self.fan_container.append(&fan_box);
        }
    }

    fn start_update_loop(&self) {
        let monitor: Arc<Mutex<ImprovedHardwareMonitor>> = Arc::clone(&self.monitor);
        let system_info = self.system_info_label.clone();
        let cpu_name = self.cpu_name_label.clone();
        let cpu_freq = self.cpu_median_freq_label.clone();
        let cpu_load = self.cpu_median_load_label.clone();
        let cpu_temp = self.cpu_package_temp_label.clone();
        let cpu_power = self.cpu_package_power_label.clone();
        let cpu_scheduler = self.cpu_scheduler_label.clone();
        let cpu_profile = self.cpu_profile_label.clone();
        
        let gpu_container = self.gpu_container.clone();
        let battery_container = self.battery_container.clone();
        let wifi_container = self.wifi_container.clone();
        let nvme_container = self.nvme_container.clone();
        let fan_container = self.fan_container.clone();

        let page_widget = self.widget.clone();

        glib::timeout_add_local(Duration::from_secs(2), move || {
            let mut mon = monitor.lock().unwrap();
            if let Ok(stats) = mon.get_system_stats() {
                // This is a simplified update - in a real implementation,
                // you'd call self.update_display(&stats) but we need to pass self
                // For now, just update the basic labels
                
                system_info.set_markup(&format!(
                    "<b>{}</b> by {}",
                    stats.system_info.product_name,
                    stats.system_info.manufacturer
                ));

                cpu_name.set_text(&format!("CPU: {}", stats.cpu.name));
                cpu_freq.set_text(&format!("Freq: {} MHz", stats.cpu.median_frequency_mhz));
                cpu_load.set_text(&format!("Load: {:.1}%", stats.cpu.median_load_percent));
                
                if let Some(temp) = stats.cpu.package_temp {
                    cpu_temp.set_text(&format!("Temp: {:.1}°C", temp));
                } else {
                    cpu_temp.set_text("Temp: N/A");
                }
                
                if let Some(power) = stats.cpu.package_power_watts {
                    cpu_power.set_text(&format!("Power: {:.2}W", power));
                } else {
                    cpu_power.set_text("Power: N/A");
                }

                cpu_scheduler.set_text(&format!("Scheduler: {}", stats.cpu.scheduler));
                
                if let Some(ref profile) = stats.cpu.profile {
                    cpu_profile.set_text(&format!("Profile: {}", profile));
                } else {
                    cpu_profile.set_text("Profile: N/A");
                }
            }

            glib::ControlFlow::Continue
        });
    }
}
