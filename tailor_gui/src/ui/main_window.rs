// src/ui/main_window.rs
use gtk::prelude::*;
use gtk::{Application, Box, Orientation};
use adw::prelude::*;
use adw::{TabBar, TabView, ApplicationWindow, HeaderBar};
use std::sync::{Arc, Mutex};
use crate::profile_controller::ProfileController;
use super::improved_statistics_page::ImprovedStatisticsPage;
use crate::improved_hardware_monitor::ImprovedHardwareMonitor;
use super::profile_page::ProfilePage;
use super::tuning_page::TuningPage;
use super::settings_page::SettingsPage;
use crate::daemon_manager::DaemonManager;

pub struct MainWindow {
    pub window: ApplicationWindow,
}

impl MainWindow {
    pub fn new(app: &Application, daemon_manager: Arc<Mutex<DaemonManager>>) -> Self {
        // Initialize controller
        let controller = match ProfileController::new() {
            Ok(ctrl) => Arc::new(Mutex::new(ctrl)),
            Err(e) => {
                eprintln!("Failed to initialize ProfileController: {}", e);
                eprintln!("The application may not function correctly.");
                // Create a dummy controller for UI testing
                std::process::exit(1);
            }
        };
                // Get controller from daemon manager
        let controller = {
            let dm = daemon_manager.lock().unwrap();
            Arc::clone(&dm.profile_controller)
        };

        // Create main window
        let window = ApplicationWindow::builder()
            .application(app)
            .title("TUXEDO/Clevo Control Center")
            .default_width(900)
            .default_height(650)
            .build();

        // Main container
        let main_box = Box::new(Orientation::Vertical, 0);

        // Header bar
        let header = HeaderBar::new();
        header.set_title_widget(Some(&adw::WindowTitle::new("TUXEDO Control", "")));

        // Menu button
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        
        // Create menu
        let menu = gtk::gio::Menu::new();
        menu.append(Some("About"), Some("app.about"));
        menu.append(Some("Preferences"), Some("app.preferences"));
        menu.append(Some("Quit"), Some("app.quit"));
        
        menu_button.set_menu_model(Some(&menu));
        header.pack_end(&menu_button);

        main_box.append(&header);

        // Tab view
        let tab_view = TabView::new();
        tab_view.set_vexpand(true);

        // Tab bar
        let tab_bar = TabBar::new();
        tab_bar.set_view(Some(&tab_view));
        tab_bar.set_autohide(false);
        main_box.append(&tab_bar);
        main_box.append(&tab_view);

        // Create pages
        let monitor = Arc::new(Mutex::new(
    ImprovedHardwareMonitor::new().expect("Failed to create monitor")
));
let statistics_page = ImprovedStatisticsPage::new(monitor);
        let profile_page = ProfilePage::new(Arc::clone(&controller));
        let tuning_page = TuningPage::new(Arc::clone(&controller));
        let settings_page = SettingsPage::new(Arc::clone(&daemon_manager));  // ADD THIS

        // Add pages to tab view
        tab_view.append(&statistics_page.widget).set_title("Statistics");
        tab_view.append(&profile_page.widget).set_title("Profiles");
        tab_view.append(&tuning_page.widget).set_title("Tuning");
        tab_view.append(&settings_page.widget).set_title("Settings");  // ADD THIS


        window.set_content(Some(&main_box));

        // Handle close request (minimize to tray)
        window.connect_close_request(move |window| {
            // Check if minimize to tray is enabled
            // For now, just hide the window
            window.set_visible(false);
            glib::Propagation::Stop
        });
        
        MainWindow { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
