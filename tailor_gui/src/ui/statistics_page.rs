// src/ui/statistics_page.rs
use gtk::prelude::*;
use gtk::{Box, Label, Orientation, Grid, Frame, ProgressBar};
use adw::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use crate::profile_controller::ProfileController;
use crate::hardware_monitor::{SystemStats, CpuCoreInfo, GpuInfo};

pub struct StatisticsPage {
    pub widget: Box,
    controller: Arc<Mutex<ProfileController>>,
    
    // CPU widgets
    cpu_package_temp_label: Label,
    cpu_package_power_label: Label,
    cpu_cores_grid: Grid,
    cpu_core_labels: Vec<(Label, Label, Label, ProgressBar)>, // freq, temp, load bar, load %
    
    // GPU widgets
    gpu_container: Box,
    gpu_info_labels: Vec<(Label, Label, Label, Label)>, // name, freq, temp, load
    active_gpu_label: Label,
    
    // Fan widgets
    fan_container: Box,
    fan_labels: Vec<(Label, Label)>, // name, rpm
}

impl StatisticsPage {
    pub fn new(controller: Arc<Mutex<ProfileController>>) -> Self {
        let main_box = Box::new(Orientation::Vertical, 12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);

        // Title
        let title = Label::new(Some("Hardware Statistics"));
        title.add_css_class("title-1");
        title.set_halign(gtk::Align::Start);
        main_box.append(&title);

        // CPU Section
        let cpu_frame = Frame::new(Some("CPU Information"));
        let cpu_box = Box::new(Orientation::Vertical, 8);
        cpu_box.set_margin_top(8);
        cpu_box.set_margin_bottom(8);
        cpu_box.set_margin_start(8);
        cpu_box.set_margin_end(8);

        // CPU package info
        let cpu_package_box = Box::new(Orientation::Horizontal, 12);
        let cpu_package_temp_label = Label::new(Some("Package Temp: --"));
        let cpu_package_power_label = Label::new(Some("Package Power: --"));
        cpu_package_box.append(&cpu_package_temp_label);
        cpu_package_box.append(&cpu_package_power_label);
        cpu_box.append(&cpu_package_box);

        // CPU cores grid (4 columns: Core, Freq, Temp, Load)
        let cpu_cores_grid = Grid::new();
        cpu_cores_grid.set_row_spacing(6);
        cpu_cores_grid.set_column_spacing(12);
        cpu_cores_grid.set_column_homogeneous(false);

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
        cpu_cores_grid.attach(&header_load, 3, 0, 2, 1);

        cpu_box.append(&cpu_cores_grid);
        cpu_frame.set_child(Some(&cpu_box));
        main_box.append(&cpu_frame);

        // GPU Section
        let gpu_frame = Frame::new(Some("GPU Information"));
        let gpu_container = Box::new(Orientation::Vertical, 8);
        gpu_container.set_margin_top(8);
        gpu_container.set_margin_bottom(8);
        gpu_container.set_margin_start(8);
        gpu_container.set_margin_end(8);

        let active_gpu_label = Label::new(Some("Active GPU: --"));
        active_gpu_label.add_css_class("heading");
        gpu_container.append(&active_gpu_label);

        gpu_frame.set_child(Some(&gpu_container));
        main_box.append(&gpu_frame);

        // Fan Section
        let fan_frame = Frame::new(Some("Fan Information"));
        let fan_container = Box::new(Orientation::Vertical, 8);
        fan_container.set_margin_top(8);
        fan_container.set_margin_bottom(8);
        fan_container.set_margin_start(8);
        fan_container.set_margin_end(8);

        fan_frame.set_child(Some(&fan_container));
        main_box.append(&fan_frame);

        let mut page = StatisticsPage {
            widget: main_box,
            controller,
            cpu_package_temp_label,
            cpu_package_power_label,
            cpu_cores_grid,
            cpu_core_labels: Vec::new(),
            gpu_container,
            gpu_info_labels: Vec::new(),
            active_gpu_label,
            fan_container,
            fan_labels: Vec::new(),
        };

        // Initialize with placeholder data
        page.initialize_cpu_cores(16); // Start with 16 cores, will adjust dynamically
        page.start_update_loop();

        page
    }

    fn initialize_cpu_cores(&mut self, count: usize) {
        // Clear existing labels
        self.cpu_core_labels.clear();

        // Create labels for each core
        for i in 0..count {
            let core_label = Label::new(Some(&format!("Core {}", i)));
            let freq_label = Label::new(Some("-- MHz"));
            let temp_label = Label::new(Some("--°C"));
            let load_bar = ProgressBar::new();
            load_bar.set_hexpand(true);
            load_bar.set_show_text(false);
            let load_label = Label::new(Some("--%"));

            // Add to grid (row i+1 because row 0 is headers)
            self.cpu_cores_grid.attach(&core_label, 0, (i + 1) as i32, 1, 1);
            self.cpu_cores_grid.attach(&freq_label, 1, (i + 1) as i32, 1, 1);
            self.cpu_cores_grid.attach(&temp_label, 2, (i + 1) as i32, 1, 1);
            self.cpu_cores_grid.attach(&load_bar, 3, (i + 1) as i32, 1, 1);
            self.cpu_cores_grid.attach(&load_label, 4, (i + 1) as i32, 1, 1);

            self.cpu_core_labels.push((freq_label, temp_label, load_label, load_bar));
        }
    }

    fn update_cpu_info(&mut self, stats: &SystemStats) {
        // Update package info
        if let Some(temp) = stats.cpu.package_temp {
            self.cpu_package_temp_label.set_text(&format!("Package Temp: {:.1}°C", temp));
        }

        if let Some(power) = stats.cpu.package_power_watts {
            self.cpu_package_power_label.set_text(&format!("Package Power: {:.2}W", power));
        }

        // Adjust core count if needed
        if stats.cpu.cores.len() != self.cpu_core_labels.len() {
            self.initialize_cpu_cores(stats.cpu.cores.len());
        }

        // Update each core
        for (i, core) in stats.cpu.cores.iter().enumerate() {
            if i < self.cpu_core_labels.len() {
                let (freq_label, temp_label, load_label, load_bar) = &self.cpu_core_labels[i];

                freq_label.set_text(&format!("{} MHz", core.frequency_mhz));

                if let Some(temp) = core.temperature {
                    temp_label.set_text(&format!("{:.1}°C", temp));
                } else {
                    temp_label.set_text("--°C");
                }

                load_label.set_text(&format!("{:.0}%", core.load_percent));
                load_bar.set_fraction(core.load_percent as f64 / 100.0);
            }
        }
    }

    fn update_gpu_info(&mut self, stats: &SystemStats) {
        // Update active GPU
        self.active_gpu_label.set_text(&format!("Active GPU: {:?}", stats.active_gpu));

        // Clear old GPU labels if count changed
        if stats.gpus.len() != self.gpu_info_labels.len() {
            // Remove old widgets
            while let Some(child) = self.gpu_container.first_child() {
                if child != self.active_gpu_label.clone().upcast::<gtk::Widget>() {
                    self.gpu_container.remove(&child);
                }
            }
            self.gpu_info_labels.clear();

            // Create new widgets for each GPU
            for gpu in &stats.gpus {
                let gpu_box = Box::new(Orientation::Vertical, 4);
                gpu_box.set_margin_top(4);

                let name_label = Label::new(Some(&format!("{} ({:?})", gpu.name, gpu.gpu_type)));
                name_label.set_halign(gtk::Align::Start);
                name_label.add_css_class("heading");

                let info_box = Box::new(Orientation::Horizontal, 12);
                let freq_label = Label::new(Some("Freq: --"));
                let temp_label = Label::new(Some("Temp: --"));
                let load_label = Label::new(Some("Load: --"));

                info_box.append(&freq_label);
                info_box.append(&temp_label);
                info_box.append(&load_label);

                gpu_box.append(&name_label);
                gpu_box.append(&info_box);

                self.gpu_container.append(&gpu_box);
                self.gpu_info_labels.push((name_label, freq_label, temp_label, load_label));
            }
        }

        // Update GPU info
        for (i, gpu) in stats.gpus.iter().enumerate() {
            if i < self.gpu_info_labels.len() {
                let (name_label, freq_label, temp_label, load_label) = &self.gpu_info_labels[i];

                name_label.set_text(&format!("{} ({:?})", gpu.name, gpu.gpu_type));

                if let Some(freq) = gpu.frequency_mhz {
                    freq_label.set_text(&format!("Freq: {} MHz", freq));
                } else {
                    freq_label.set_text("Freq: --");
                }

                if let Some(temp) = gpu.temperature {
                    temp_label.set_text(&format!("Temp: {:.1}°C", temp));
                } else {
                    temp_label.set_text("Temp: --");
                }

                if let Some(load) = gpu.load_percent {
                    load_label.set_text(&format!("Load: {:.0}%", load));
                } else {
                    load_label.set_text("Load: --");
                }
            }
        }
    }

    fn update_fan_info(&mut self, stats: &SystemStats) {
        // Clear old fan labels if count changed
        if stats.fans.len() != self.fan_labels.len() {
            while let Some(child) = self.fan_container.first_child() {
                self.fan_container.remove(&child);
            }
            self.fan_labels.clear();

            // Create new widgets for each fan
            for _ in &stats.fans {
                let fan_box = Box::new(Orientation::Horizontal, 12);
                let name_label = Label::new(Some("Fan"));
                let rpm_label = Label::new(Some("-- RPM"));

                fan_box.append(&name_label);
                fan_box.append(&rpm_label);

                self.fan_container.append(&fan_box);
                self.fan_labels.push((name_label, rpm_label));
            }
        }

        // Update fan info
        for (i, fan) in stats.fans.iter().enumerate() {
            if i < self.fan_labels.len() {
                let (name_label, rpm_label) = &self.fan_labels[i];

                name_label.set_text(&fan.name);

                if let Some(rpm) = fan.speed_rpm {
                    rpm_label.set_text(&format!("{} RPM", rpm));
                } else {
                    rpm_label.set_text("-- RPM");
                }
            }
        }
    }

    fn start_update_loop(&self) {
        let controller = Arc::clone(&self.controller);
        let cpu_package_temp = self.cpu_package_temp_label.clone();
        let cpu_package_power = self.cpu_package_power_label.clone();

        // Clone all the labels we need to update
        let cpu_cores: Vec<_> = self.cpu_core_labels.iter().cloned().collect();
        let active_gpu = self.active_gpu_label.clone();

        glib::timeout_add_local(Duration::from_secs(2), move || {
            // Get stats in a separate thread to avoid blocking UI
            let ctrl = controller.lock().unwrap();
            if let Ok(stats) = ctrl.get_hardware_stats() {
                // Update CPU package info
                if let Some(temp) = stats.cpu.package_temp {
                    cpu_package_temp.set_text(&format!("Package Temp: {:.1}°C", temp));
                }
                if let Some(power) = stats.cpu.package_power_watts {
                    cpu_package_power.set_text(&format!("Package Power: {:.2}W", power));
                }

                // Update CPU cores
                for (i, core) in stats.cpu.cores.iter().enumerate() {
                    if i < cpu_cores.len() {
                        let (freq, temp, load_text, load_bar) = &cpu_cores[i];
                        freq.set_text(&format!("{} MHz", core.frequency_mhz));
                        
                        if let Some(t) = core.temperature {
                            temp.set_text(&format!("{:.1}°C", t));
                        }
                        
                        load_text.set_text(&format!("{:.0}%", core.load_percent));
                        load_bar.set_fraction(core.load_percent as f64 / 100.0);
                    }
                }

                // Update active GPU
                active_gpu.set_text(&format!("Active GPU: {:?}", stats.active_gpu));
            }

            glib::ControlFlow::Continue
        });
    }
}
