// src/ui/settings_page.rs
use gtk::prelude::*;
use gtk::{Box, Button, Label, Orientation, Switch};
use adw::prelude::*;
use adw::prelude::MessageDialogExt;
use adw::{PreferencesGroup, ActionRow};
use std::sync::{Arc, Mutex};
use crate::daemon_manager::DaemonManager;

pub struct SettingsPage {
    pub widget: Box,
    daemon_manager: Arc<Mutex<DaemonManager>>,
    
    // Daemon controls
    fan_daemon_switch: Switch,
    app_monitoring_switch: Switch,
    minimize_to_tray_switch: Switch,
    start_minimized_switch: Switch,
    
    // Status labels
    status_label: Label,
}

impl SettingsPage {
    pub fn new(daemon_manager: Arc<Mutex<DaemonManager>>) -> Self {
        let main_box = Box::new(Orientation::Vertical, 0);

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_vexpand(true);

        let content_box = Box::new(Orientation::Vertical, 24);
        content_box.set_margin_top(24);
        content_box.set_margin_bottom(24);
        content_box.set_margin_start(12);
        content_box.set_margin_end(12);

        // Title
        let title = Label::new(Some("Settings"));
        title.add_css_class("title-1");
        title.set_halign(gtk::Align::Start);
        content_box.append(&title);

        // Daemon Settings Group
        let daemon_group = PreferencesGroup::new();
        daemon_group.set_title("Background Services");
        daemon_group.set_description(Some("Control background monitoring and automation"));

        // Fan daemon control
        let fan_daemon_switch = Switch::new();
        fan_daemon_switch.set_valign(gtk::Align::Center);
        
        let fan_daemon_row = ActionRow::new();
        fan_daemon_row.set_title("Fan Curve Daemon");
        fan_daemon_row.set_subtitle("Continuously adjust fan speeds based on temperature");
        fan_daemon_row.add_suffix(&fan_daemon_switch);
        fan_daemon_row.set_activatable_widget(Some(&fan_daemon_switch));
        daemon_group.add(&fan_daemon_row);

        // App monitoring control
        let app_monitoring_switch = Switch::new();
        app_monitoring_switch.set_valign(gtk::Align::Center);
        
        let app_monitoring_row = ActionRow::new();
        app_monitoring_row.set_title("Application Monitoring");
        app_monitoring_row.set_subtitle("Automatically switch profiles based on running apps");
        app_monitoring_row.add_suffix(&app_monitoring_switch);
        app_monitoring_row.set_activatable_widget(Some(&app_monitoring_switch));
        daemon_group.add(&app_monitoring_row);

        content_box.append(&daemon_group);

        // Window Behavior Group
        let window_group = PreferencesGroup::new();
        window_group.set_title("Window Behavior");

        // Minimize to tray
        let minimize_to_tray_switch = Switch::new();
        minimize_to_tray_switch.set_valign(gtk::Align::Center);
        minimize_to_tray_switch.set_active(true);
        
        let minimize_row = ActionRow::new();
        minimize_row.set_title("Minimize to System Tray");
        minimize_row.set_subtitle("Keep running in background when window is closed");
        minimize_row.add_suffix(&minimize_to_tray_switch);
        minimize_row.set_activatable_widget(Some(&minimize_to_tray_switch));
        window_group.add(&minimize_row);

        // Start minimized
        let start_minimized_switch = Switch::new();
        start_minimized_switch.set_valign(gtk::Align::Center);
        
        let start_minimized_row = ActionRow::new();
        start_minimized_row.set_title("Start Minimized");
        start_minimized_row.set_subtitle("Start application in system tray");
        start_minimized_row.add_suffix(&start_minimized_switch);
        start_minimized_row.set_activatable_widget(Some(&start_minimized_switch));
        window_group.add(&start_minimized_row);

        content_box.append(&window_group);

        // Status Group
        let status_group = PreferencesGroup::new();
        status_group.set_title("Status");

        let status_label = Label::new(Some("Loading status..."));
        status_label.set_halign(gtk::Align::Start);
        status_label.set_margin_top(8);
        status_label.set_margin_bottom(8);
        
        let status_row = ActionRow::new();
        status_row.set_title("Daemon Status");
        status_row.set_child(Some(&status_label));
        status_group.add(&status_row);

        // Refresh button
        let refresh_button = Button::with_label("Refresh Status");
        refresh_button.set_halign(gtk::Align::Start);
        refresh_button.set_margin_top(8);
        
        let refresh_row = ActionRow::new();
        refresh_row.add_suffix(&refresh_button);
        status_group.add(&refresh_row);

        content_box.append(&status_group);

        // Advanced Group
        let advanced_group = PreferencesGroup::new();
        advanced_group.set_title("Advanced");

        // Reset to defaults
        let reset_button = Button::with_label("Reset to Defaults");
        reset_button.add_css_class("destructive-action");
        reset_button.set_halign(gtk::Align::Start);
        
        let reset_row = ActionRow::new();
        reset_row.set_title("Reset Settings");
        reset_row.set_subtitle("Restore all settings to default values");
        reset_row.add_suffix(&reset_button);
        advanced_group.add(&reset_row);

        content_box.append(&advanced_group);

        scrolled.set_child(Some(&content_box));
        main_box.append(&scrolled);

        let mut page = SettingsPage {
            widget: main_box,
            daemon_manager: daemon_manager.clone(),
            fan_daemon_switch: fan_daemon_switch.clone(),
            app_monitoring_switch: app_monitoring_switch.clone(),
            minimize_to_tray_switch,
            start_minimized_switch,
            status_label: status_label.clone(),
        };

        // Setup initial state
        page.refresh_status();

        // Setup signal handlers
        page.setup_signals(refresh_button, reset_button);

        page
    }

    fn refresh_status(&self) {
        let daemon_mgr = self.daemon_manager.lock().unwrap();
        let status = daemon_mgr.get_status();
        drop(daemon_mgr);

        // Update switches
        self.fan_daemon_switch.set_active(status.fan_daemon_running);
        self.app_monitoring_switch.set_active(status.app_monitoring_running);

        // Update status label
        self.status_label.set_markup(&format!(
            "<span font_family='monospace'>{}</span>",
            glib::markup_escape_text(&status.to_string())
        ));
    }

    fn setup_signals(&self, refresh_button: Button, reset_button: Button) {
        // Fan daemon switch
        let daemon_mgr = Arc::clone(&self.daemon_manager);
        let status_label = self.status_label.clone();
        
        self.fan_daemon_switch.connect_state_set(move |_, enabled| {
            let daemon_mgr = daemon_mgr.lock().unwrap();
            
            if enabled {
                let ctrl = daemon_mgr.profile_controller.lock().unwrap();
                let profile = ctrl.get_active_profile();
                drop(ctrl);
                drop(daemon_mgr);
                
                let daemon_mgr = daemon_mgr.lock().unwrap();
                if let Err(e) = daemon_mgr.start_fan_daemon(profile) {
                    eprintln!("Failed to start fan daemon: {}", e);
                    status_label.set_text(&format!("Error: {}", e));
                    return gtk::Inhibit(true);
                }
            } else {
                daemon_mgr.stop_fan_daemon();
            }
            
            gtk::Inhibit(false)
        });

        // App monitoring switch
        let daemon_mgr = Arc::clone(&self.daemon_manager);
        let status_label = self.status_label.clone();
        
        self.app_monitoring_switch.connect_state_set(move |_, enabled| {
            let daemon_mgr = daemon_mgr.lock().unwrap();
            
            if enabled {
                if let Err(e) = daemon_mgr.start_app_monitoring() {
                    eprintln!("Failed to start app monitoring: {}", e);
                    status_label.set_text(&format!("Error: {}", e));
                    return gtk::Inhibit(true);
                }
            } else {
                daemon_mgr.stop_app_monitoring();
            }
            
            gtk::Inhibit(false)
        });

        // Refresh button
        let page_self = SettingsPage {
            widget: self.widget.clone(),
            daemon_manager: Arc::clone(&self.daemon_manager),
            fan_daemon_switch: self.fan_daemon_switch.clone(),
            app_monitoring_switch: self.app_monitoring_switch.clone(),
            minimize_to_tray_switch: self.minimize_to_tray_switch.clone(),
            start_minimized_switch: self.start_minimized_switch.clone(),
            status_label: self.status_label.clone(),
        };

        refresh_button.connect_clicked(move |_| {
            page_self.refresh_status();
        });

        // Reset button
        let widget = self.widget.clone();
        reset_button.connect_clicked(move |_| {
            let dialog = adw::MessageDialog::new(
                widget.root().and_downcast_ref::<gtk::Window>().as_ref(),
                Some("Reset Settings?"),
                Some("This will restore all settings to their default values. This action cannot be undone."),
            );
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("reset", "Reset");
            dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);

            dialog.connect_response(None, move |dialog, response| {
                if response == "reset" {
                    println!("Resetting to defaults...");
                    // TODO: Implement reset logic
                }
                dialog.close();
            });

            dialog.present();
        });
    }
}
