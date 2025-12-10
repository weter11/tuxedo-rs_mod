// src/tray_manager.rs
use gtk::prelude::*;
use gtk::gio;
use std::sync::{Arc, Mutex};
use crate::profile_controller::ProfileController;

/// System tray manager for profile switching
pub struct TrayManager {
    controller: Arc<Mutex<ProfileController>>,
    status_icon: Option<gtk::gio::Notification>,
}

impl TrayManager {
    pub fn new(controller: Arc<Mutex<ProfileController>>) -> Self {
        TrayManager {
            controller,
            status_icon: None,
        }
    }

    /// Initialize system tray (uses GNotification on modern desktops)
    pub fn setup(&mut self, app: &gtk::Application) {
        // On modern Linux desktops (GNOME, KDE Plasma 6), we use GNotification
        // which shows in the system status area
        
        // Create menu for tray
        let menu = self.create_tray_menu();
        app.set_app_menu(Some(&menu));

        // Show initial notification
        self.send_notification(
            app,
            "TUXEDO Control",
            "Running in background. Click to open.",
        );
    }

    /// Create the tray menu with profile list
    fn create_tray_menu(&self) -> gio::Menu {
        let menu = gio::Menu::new();

        // Profiles section
        let profiles_section = gio::Menu::new();
        
        let ctrl = self.controller.lock().unwrap();
        let profiles = ctrl.get_all_profiles();
        let active_profile = ctrl.get_active_profile();
        drop(ctrl);

        for (idx, profile) in profiles.iter().enumerate() {
            let action_name = format!("app.switch-profile-{}", idx);
            let item = gio::MenuItem::new(Some(&profile.name), Some(&action_name));
            
            // Mark active profile
            if profile.name == active_profile.name {
                item.set_icon(&gio::Icon::for_string("emblem-default").unwrap());
            }
            
            profiles_section.append_item(&item);
        }

        menu.append_section(Some("Profiles"), &profiles_section);

        // Actions section
        let actions_section = gio::Menu::new();
        actions_section.append(Some("Open Control Center"), Some("app.show-window"));
        actions_section.append(Some("Enable Auto-Switch"), Some("app.toggle-auto-switch"));
        actions_section.append(Some("Quit"), Some("app.quit"));

        menu.append_section(None, &actions_section);

        menu
    }

    /// Send a system notification
    pub fn send_notification(&self, app: &gtk::Application, title: &str, body: &str) {
        let notification = gio::Notification::new(title);
        notification.set_body(Some(body));
        notification.set_icon(&gio::Icon::for_string("computer").unwrap());
        
        // Add action to show window
        notification.add_button("Show", "app.show-window");
        
        app.send_notification(Some("tuxedo-control"), &notification);
    }

    /// Update tray menu when profiles change
    pub fn refresh_menu(&self, app: &gtk::Application) {
        let menu = self.create_tray_menu();
        app.set_app_menu(Some(&menu));
    }

    /// Notify about profile switch
    pub fn notify_profile_switch(&self, app: &gtk::Application, profile_name: &str) {
        self.send_notification(
            app,
            "Profile Switched",
            &format!("Now using profile: {}", profile_name),
        );
    }
}

/// Setup tray-related actions
pub fn setup_tray_actions(
    app: &gtk::Application,
    controller: Arc<Mutex<ProfileController>>,
    tray: Arc<Mutex<TrayManager>>,
) {
    // Show window action
    let show_action = gio::SimpleAction::new("show-window", None);
    show_action.connect_activate(glib::clone!(@weak app => move |_, _| {
        if let Some(window) = app.active_window() {
            window.present();
        }
    }));
    app.add_action(&show_action);

    // Toggle auto-switch action
    let auto_switch_action = gio::SimpleAction::new_stateful(
        "toggle-auto-switch",
        None,
        &false.to_variant(),
    );
    
    let ctrl = Arc::clone(&controller);
    auto_switch_action.connect_activate(move |action, _| {
        let state = action.state().unwrap().get::<bool>().unwrap();
        let new_state = !state;
        action.set_state(&new_state.to_variant());

        let ctrl = ctrl.lock().unwrap();
        if new_state {
            if let Err(e) = ctrl.start_app_monitoring() {
                eprintln!("Failed to start app monitoring: {}", e);
            }
        } else {
            ctrl.stop_app_monitoring();
        }
    });
    app.add_action(&auto_switch_action);

    // Profile switch actions
    let ctrl = controller.lock().unwrap();
    let profiles = ctrl.get_all_profiles();
    drop(ctrl);

    for (idx, _profile) in profiles.iter().enumerate() {
        let action_name = format!("switch-profile-{}", idx);
        let action = gio::SimpleAction::new(&action_name, None);

        let ctrl = Arc::clone(&controller);
        let tray_ref = Arc::clone(&tray);
        let app_weak = app.downgrade();
        
        action.connect_activate(move |_, _| {
            let ctrl = ctrl.lock().unwrap();
            if let Err(e) = ctrl.apply_profile(idx) {
                eprintln!("Failed to apply profile: {}", e);
            } else {
                let profile = ctrl.get_active_profile();
                drop(ctrl);
                
                // Send notification
                if let Some(app) = app_weak.upgrade() {
                    let tray = tray_ref.lock().unwrap();
                    tray.notify_profile_switch(&app, &profile.name);
                }
            }
        });

        app.add_action(&action);
    }
}
