// src/ui/tuning_page.rs
use gtk::prelude::*;
use gtk::{Box, Button, Label, Orientation, Scale, SpinButton, Adjustment, ComboBoxText, Switch, Entry, Grid};
use adw::prelude::*;
use adw::{PreferencesGroup, ActionRow, ComboRow, SpinRow};
use std::sync::{Arc, Mutex};
use crate::profile_controller::ProfileController;
use crate::profile_system::{Profile, CpuPerformanceProfile, RGBColor};

pub struct TuningPage {
    pub widget: Box,
    controller: Arc<Mutex<ProfileController>>,
    
    // Keyboard controls
    kb_red_scale: Scale,
    kb_green_scale: Scale,
    kb_blue_scale: Scale,
    kb_brightness_scale: Scale,
    
    // CPU controls
    cpu_profile_combo: ComboBoxText,
    cpu_min_freq_spin: SpinButton,
    cpu_max_freq_spin: SpinButton,
    cpu_boost_switch: Switch,
    cpu_smt_switch: Switch,
    
    // Screen controls
    screen_brightness_scale: Scale,
    screen_auto_switch: Switch,
    
    // Auto-switch controls
    auto_switch_enabled: Switch,
    trigger_apps_entry: Entry,
    
    current_profile: Arc<Mutex<Option<Profile>>>,
}

impl TuningPage {
    pub fn new(controller: Arc<Mutex<ProfileController>>) -> Self {
        let main_box = Box::new(Orientation::Vertical, 0);

        // Use Adwaita preferences groups for better styling
        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_vexpand(true);

        let content_box = Box::new(Orientation::Vertical, 24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);
        content_box.set_margin_start(12);
        content_box.set_margin_end(12);

        // Title
        let title = Label::new(Some("Profile Tuning"));
        title.add_css_class("title-1");
        title.set_halign(gtk::Align::Start);
        content_box.append(&title);

        // Keyboard Backlight Group
        let kb_group = PreferencesGroup::new();
        kb_group.set_title("Keyboard Backlight");

        // RGB Color controls
        let kb_color_box = Box::new(Orientation::Vertical, 8);
        
        let kb_red_scale = create_rgb_scale("Red", 255.0);
        let kb_green_scale = create_rgb_scale("Green", 255.0);
        let kb_blue_scale = create_rgb_scale("Blue", 255.0);
        
        kb_color_box.append(&create_scale_row("Red", &kb_red_scale));
        kb_color_box.append(&create_scale_row("Green", &kb_green_scale));
        kb_color_box.append(&create_scale_row("Blue", &kb_blue_scale));
        
        let kb_color_row = ActionRow::new();
        kb_color_row.set_title("RGB Color");
        kb_color_row.set_child(Some(&kb_color_box));
        kb_group.add(&kb_color_row);

        // Brightness control
        let kb_brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        kb_brightness_scale.set_value(50.0);
        kb_brightness_scale.set_draw_value(true);
        kb_brightness_scale.set_value_pos(gtk::PositionType::Right);
        kb_brightness_scale.set_hexpand(true);
        
        let kb_brightness_row = ActionRow::new();
        kb_brightness_row.set_title("Brightness");
        kb_brightness_row.set_child(Some(&kb_brightness_scale));
        kb_group.add(&kb_brightness_row);

        content_box.append(&kb_group);

        // CPU Settings Group
        let cpu_group = PreferencesGroup::new();
        cpu_group.set_title("CPU Settings");

        // Performance profile
        let cpu_profile_combo = ComboBoxText::new();
        cpu_profile_combo.append(Some("powersave"), "Power Save");
        cpu_profile_combo.append(Some("balanced"), "Balanced");
        cpu_profile_combo.append(Some("performance"), "Performance");
        cpu_profile_combo.set_active_id(Some("balanced"));
        
        let cpu_profile_row = ActionRow::new();
        cpu_profile_row.set_title("Performance Profile");
        cpu_profile_row.set_subtitle("CPU governor mode");
        cpu_profile_row.add_suffix(&cpu_profile_combo);
        cpu_group.add(&cpu_profile_row);

        // Frequency limits
        let freq_grid = Grid::new();
        freq_grid.set_row_spacing(8);
        freq_grid.set_column_spacing(12);
        
        let min_label = Label::new(Some("Min Frequency (MHz):"));
        let cpu_min_freq_spin = SpinButton::with_range(800.0, 5000.0, 100.0);
        cpu_min_freq_spin.set_value(800.0);
        
        let max_label = Label::new(Some("Max Frequency (MHz):"));
        let cpu_max_freq_spin = SpinButton::with_range(800.0, 5000.0, 100.0);
        cpu_max_freq_spin.set_value(4000.0);
        
        freq_grid.attach(&min_label, 0, 0, 1, 1);
        freq_grid.attach(&cpu_min_freq_spin, 1, 0, 1, 1);
        freq_grid.attach(&max_label, 0, 1, 1, 1);
        freq_grid.attach(&cpu_max_freq_spin, 1, 1, 1, 1);
        
        let freq_row = ActionRow::new();
        freq_row.set_title("Frequency Limits");
        freq_row.set_child(Some(&freq_grid));
        cpu_group.add(&freq_row);

        // Boost control
        let cpu_boost_switch = Switch::new();
        cpu_boost_switch.set_active(true);
        cpu_boost_switch.set_valign(gtk::Align::Center);
        
        let boost_row = ActionRow::new();
        boost_row.set_title("CPU Boost");
        boost_row.set_subtitle("Enable turbo boost / precision boost");
        boost_row.add_suffix(&cpu_boost_switch);
        boost_row.set_activatable_widget(Some(&cpu_boost_switch));
        cpu_group.add(&boost_row);

        // SMT control
        let cpu_smt_switch = Switch::new();
        cpu_smt_switch.set_active(true);
        cpu_smt_switch.set_valign(gtk::Align::Center);
        
        let smt_row = ActionRow::new();
        smt_row.set_title("Hyperthreading / SMT");
        smt_row.set_subtitle("Simultaneous Multithreading");
        smt_row.add_suffix(&cpu_smt_switch);
        smt_row.set_activatable_widget(Some(&cpu_smt_switch));
        cpu_group.add(&smt_row);

        content_box.append(&cpu_group);

        // Screen Settings Group
        let screen_group = PreferencesGroup::new();
        screen_group.set_title("Screen Settings");

        // Brightness
        let screen_brightness_scale = Scale::with_range(Orientation::Horizontal, 0.0, 100.0, 1.0);
        screen_brightness_scale.set_value(70.0);
        screen_brightness_scale.set_draw_value(true);
        screen_brightness_scale.set_value_pos(gtk::PositionType::Right);
        screen_brightness_scale.set_hexpand(true);
        
        let screen_brightness_row = ActionRow::new();
        screen_brightness_row.set_title("Brightness");
        screen_brightness_row.set_child(Some(&screen_brightness_scale));
        screen_group.add(&screen_brightness_row);

        // Auto brightness
        let screen_auto_switch = Switch::new();
        screen_auto_switch.set_valign(gtk::Align::Center);
        
        let auto_brightness_row = ActionRow::new();
        auto_brightness_row.set_title("Auto Brightness");
        auto_brightness_row.set_subtitle("Adjust brightness automatically");
        auto_brightness_row.add_suffix(&screen_auto_switch);
        auto_brightness_row.set_activatable_widget(Some(&screen_auto_switch));
        screen_group.add(&auto_brightness_row);

        content_box.append(&screen_group);

        // Auto-Switch Settings Group
        let auto_group = PreferencesGroup::new();
        auto_group.set_title("Auto-Switch Settings");

        // Enable auto-switch
        let auto_switch_enabled = Switch::new();
        auto_switch_enabled.set_valign(gtk::Align::Center);
        
        let auto_enable_row = ActionRow::new();
        auto_enable_row.set_title("Enable Auto-Switch");
        auto_enable_row.set_subtitle("Automatically apply this profile for specific apps");
        auto_enable_row.add_suffix(&auto_switch_enabled);
        auto_enable_row.set_activatable_widget(Some(&auto_switch_enabled));
        auto_group.add(&auto_enable_row);

        // Trigger apps
        let trigger_apps_entry = Entry::new();
        trigger_apps_entry.set_placeholder_text(Some("steam, lutris, gamemode"));
        trigger_apps_entry.set_hexpand(true);
        
        let trigger_row = ActionRow::new();
        trigger_row.set_title("Trigger Applications");
        trigger_row.set_subtitle("Comma-separated list of app names");
        trigger_row.add_suffix(&trigger_apps_entry);
        auto_group.add(&trigger_row);

        content_box.append(&auto_group);

        // Action buttons
        let button_box = Box::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk::Align::End);
        button_box.set_margin_top(12);

        let save_button = Button::with_label("Save Profile");
        save_button.add_css_class("suggested-action");

        let reset_button = Button::with_label("Reset");

        button_box.append(&reset_button);
        button_box.append(&save_button);
        content_box.append(&button_box);

        scrolled.set_child(Some(&content_box));
        main_box.append(&scrolled);

        let page = TuningPage {
            widget: main_box,
            controller,
            kb_red_scale,
            kb_green_scale,
            kb_blue_scale,
            kb_brightness_scale,
            cpu_profile_combo,
            cpu_min_freq_spin,
            cpu_max_freq_spin,
            cpu_boost_switch,
            cpu_smt_switch,
            screen_brightness_scale,
            screen_auto_switch,
            auto_switch_enabled,
            trigger_apps_entry,
            current_profile: Arc::new(Mutex::new(None)),
        };

        // Setup button signals
        let page_weak = glib::SendWeakRef::from(page.widget.downgrade());
        save_button.connect_clicked(move |_| {
            // Save profile logic would go here
            println!("Save profile clicked");
        });

        reset_button.connect_clicked(move |_| {
            // Reset to current profile values
            println!("Reset clicked");
        });

        page
    }

    pub fn load_profile(&self, profile: &Profile) {
        // Load keyboard settings
        self.kb_red_scale.set_value(profile.keyboard_backlight.color.r as f64);
        self.kb_green_scale.set_value(profile.keyboard_backlight.color.g as f64);
        self.kb_blue_scale.set_value(profile.keyboard_backlight.color.b as f64);
        self.kb_brightness_scale.set_value(profile.keyboard_backlight.brightness as f64);

        // Load CPU settings
        let profile_id = match profile.cpu_settings.performance_profile {
            CpuPerformanceProfile::PowerSave => "powersave",
            CpuPerformanceProfile::Balanced => "balanced",
            CpuPerformanceProfile::Performance => "performance",
        };
        self.cpu_profile_combo.set_active_id(Some(profile_id));

        if let Some(min_freq) = profile.cpu_settings.min_freq_mhz {
            self.cpu_min_freq_spin.set_value(min_freq as f64);
        }
        if let Some(max_freq) = profile.cpu_settings.max_freq_mhz {
            self.cpu_max_freq_spin.set_value(max_freq as f64);
        }

        self.cpu_boost_switch.set_active(!profile.cpu_settings.disable_boost);
        self.cpu_smt_switch.set_active(profile.cpu_settings.smt_enabled);

        // Load screen settings
        self.screen_brightness_scale.set_value(profile.screen_settings.brightness as f64);
        self.screen_auto_switch.set_active(profile.screen_settings.auto_brightness);

        // Load auto-switch settings
        self.auto_switch_enabled.set_active(profile.auto_switch_enabled);
        self.trigger_apps_entry.set_text(&profile.trigger_apps.join(", "));

        *self.current_profile.lock().unwrap() = Some(profile.clone());
    }
}

fn create_rgb_scale(label: &str, max: f64) -> Scale {
    let scale = Scale::with_range(Orientation::Horizontal, 0.0, max, 1.0);
    scale.set_value(255.0);
    scale.set_draw_value(true);
    scale.set_value_pos(gtk::PositionType::Right);
    scale.set_hexpand(true);
    scale
}

fn create_scale_row(label: &str, scale: &Scale) -> Box {
    let row_box = Box::new(Orientation::Horizontal, 12);
    let label_widget = Label::new(Some(label));
    label_widget.set_width_chars(6);
    row_box.append(&label_widget);
    row_box.append(scale);
    row_box
}
