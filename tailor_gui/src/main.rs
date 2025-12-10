// src/main.rs - Phase 4.5 Updates
mod config;

// Phase 1 modules
pub mod profile_system;
pub mod hardware_monitor;
pub mod keyboard_control;

// Phase 2 modules
pub mod hardware_control;
pub mod profile_controller;

// Phase 3 modules
pub mod ui;

// Phase 4 modules
pub mod tray_manager;
pub mod fan_daemon;
pub mod daemon_manager;

// Phase 4.5 modules - NEW
pub mod improved_hardware_monitor;
pub mod single_instance;

use gtk::prelude::*;
use gtk::{gio, Application};
use adw;
use std::sync::{Arc, Mutex};
use crate::daemon_manager::DaemonManager;
use crate::tray_manager::{TrayManager, setup_tray_actions};
use crate::profile_controller::ProfileController;
use crate::single_instance::SingleInstance;

const APP_ID: &str = "com.github.tuxedo.control";

fn main() -> glib::ExitCode {
    // Check for single instance
    let mut instance_lock = SingleInstance::new(APP_ID).expect("Failed to create instance lock");
    
    if !instance_lock.try_acquire().expect("Failed to acquire lock") {
        // Another instance is running
        eprintln!("TUXEDO Control is already running");
        
        // Try to activate the existing instance
        if let Err(e) = instance_lock.activate_running_instance() {
            eprintln!("Failed to activate running instance: {}", e);
            
            // Show GTK dialog
            gtk::init().expect("Failed to initialize GTK");
            let dialog = gtk::MessageDialog::new(
                None::<&gtk::Window>,
                gtk::DialogFlags::MODAL,
                gtk::MessageType::Info,
                gtk::ButtonsType::Ok,
                "TUXEDO Control is already running.\n\nClick the system tray icon to show the window.",
            );
            dialog.run();
            dialog.close();
        }
        
        return glib::ExitCode::SUCCESS;
    }
    
    // Initialize GTK
    gtk::init().expect("Failed to initialize GTK");
    
    // Create application
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    // Store instance lock to be dropped when app closes
    let instance_lock = Arc::new(Mutex::new(Some(instance_lock)));

    // Connect startup signal
    app.connect_startup(|_| {
        adw::init().expect("Failed to initialize Libadwaita");
        load_css();
    });

    // Connect activate signal
    let instance_lock_clone = Arc::clone(&instance_lock);
    app.connect_activate(move |app| {
        // Check permissions
        match crate::hardware_control::check_permissions() {
            Ok(has_perms) => {
                if !has_perms {
                    show_permission_warning(app);
                }
            }
            Err(e) => {
                eprintln!("Failed to check permissions: {}", e);
            }
        }

        // Initialize controller
        let controller = match ProfileController::new() {
            Ok(ctrl) => Arc::new(Mutex::new(ctrl)),
            Err(e) => {
                eprintln!("Failed to initialize ProfileController: {}", e);
                show_error_dialog(app, "Initialization Error", &format!("Failed to initialize: {}", e));
                return;
            }
        };

        // Initialize daemon manager
        let daemon_manager = match DaemonManager::new(Arc::clone(&controller)) {
            Ok(dm) => Arc::new(Mutex::new(dm)),
            Err(e) => {
                eprintln!("Failed to initialize DaemonManager: {}", e);
                show_error_dialog(app, "Daemon Error", &format!("Failed to initialize daemons: {}", e));
                return;
            }
        };

        // Start daemons
        {
            let dm = daemon_manager.lock().unwrap();
            if let Err(e) = dm.start_all() {
                eprintln!("Warning: Failed to start some daemons: {}", e);
            }
        }

        // Initialize tray
        let tray = Arc::new(Mutex::new(TrayManager::new(Arc::clone(&controller))));
        {
            let mut tray_ref = tray.lock().unwrap();
            tray_ref.setup(app);
        }

        // Setup tray actions
        setup_tray_actions(app, Arc::clone(&controller), Arc::clone(&tray));

        // Create and show main window
        let window = ui::main_window::MainWindow::new(app, Arc::clone(&daemon_manager));
        
        // Setup window close handler (minimize to tray)
        let app_weak = app.downgrade();
        let daemon_mgr_weak = Arc::downgrade(&daemon_manager);
        let instance_lock_weak = Arc::downgrade(&instance_lock_clone);
        
        window.window.connect_close_request(move |window| {
            // Just hide the window (minimize to tray)
            window.hide();
            gtk::Inhibit(true) // Prevent actual close
        });

        window.present();
    });

    // Setup actions
    let daemon_manager_clone = daemon_manager.clone();
    let instance_lock_clone = Arc::clone(&instance_lock);
    setup_actions(&app, daemon_manager_clone, instance_lock_clone);

    // Setup signal handlers for graceful shutdown
    setup_signal_handlers(Arc::clone(&instance_lock));

    app.run()
}

fn setup_signal_handlers(instance_lock: Arc<Mutex<Option<SingleInstance>>>) {
    // Handle SIGUSR1 to bring window to front
    unsafe {
        libc::signal(libc::SIGUSR1, signal_handler as libc::sighandler_t);
    }
}

extern "C" fn signal_handler(_: i32) {
    // Post to GTK main loop to show window
    glib::idle_add_once(|| {
        if let Some(app) = gtk::gio::Application::default() {
            if let Some(app) = app.downcast_ref::<gtk::Application>() {
                if let Some(window) = app.active_window() {
                    window.present();
                }
            }
        }
    });
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        r#"
        .badge {
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 0.8em;
            font-weight: bold;
        }
        
        .success {
            background-color: @success_color;
            color: @success_fg_color;
        }
        
        .accent {
            background-color: @accent_color;
            color: @accent_fg_color;
        }
        "#
    );

    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Could not connect to display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn show_permission_warning(app: &Application) {
    let dialog = adw::MessageDialog::new(
        None::<&gtk::Window>,
        Some("Limited Permissions"),
        Some("The application is not running with root privileges. Hardware control features may not work correctly."),
    );
    dialog.add_response("ok", "OK");
    dialog.add_response("restart", "Restart as Root");
    dialog.set_response_appearance("restart", adw::ResponseAppearance::Suggested);

    let app_weak = app.downgrade();
    dialog.connect_response(None, move |dialog, response| {
        if response == "restart" {
            if let Some(app) = app_weak.upgrade() {
                show_root_instructions(&app);
            }
        }
        dialog.close();
    });

    dialog.present();
}

fn show_root_instructions(app: &Application) {
    let dialog = adw::MessageDialog::new(
        None::<&gtk::Window>,
        Some("Running as Root"),
        Some("To enable hardware control, restart the application with:\n\nsudo tailor-gui\n\nOr use pkexec for a graphical authentication dialog."),
    );
    dialog.add_response("ok", "OK");
    dialog.present();
}

fn show_error_dialog(app: &Application, title: &str, message: &str) {
    let dialog = adw::MessageDialog::new(
        None::<&gtk::Window>,
        Some(title),
        Some(message),
    );
    dialog.add_response("ok", "OK");
    dialog.present();
}

fn setup_actions(
    app: &Application,
    daemon_manager: Arc<Mutex<DaemonManager>>,
    instance_lock: Arc<Mutex<Option<SingleInstance>>>,
) {
    // About action
    let about_action = gio::SimpleAction::new("about", None);
    about_action.connect_activate(move |_, _| {
        show_about_dialog();
    });
    app.add_action(&about_action);

    // Preferences action
    let preferences_action = gio::SimpleAction::new("preferences", None);
    preferences_action.connect_activate(move |_, _| {
        println!("Preferences clicked");
        // TODO: Implement preferences dialog
    });
    app.add_action(&preferences_action);

    // Quit action - Stop daemons and exit
    let quit_action = gio::SimpleAction::new("quit", None);
    let daemon_mgr = Arc::clone(&daemon_manager);
    let instance = Arc::clone(&instance_lock);
    quit_action.connect_activate(glib::clone!(@weak app => move |_, _| {
        println!("Quitting application...");
        
        // Stop all daemons
        {
            let dm = daemon_mgr.lock().unwrap();
            dm.stop_all();
        }
        
        // Release instance lock
        {
            let mut lock = instance.lock().unwrap();
            if let Some(mut inst) = lock.take() {
                inst.release();
            }
        }
        
        // Unload tailord service if running
        let _ = std::process::Command::new("systemctl")
            .args(&["--user", "stop", "tailord.service"])
            .output();
        
        app.quit();
    }));
    app.add_action(&quit_action);
    
    app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
}

fn show_about_dialog() {
    let about = adw::AboutWindow::builder()
        .application_name("TUXEDO Control Center")
        .application_icon("computer")
        .version("0.4.5")
        .developer_name("TUXEDO Community")
        .issue_url("https://github.com/weter11/tuxedo-rs_mod/issues")
        .website("https://github.com/weter11/tuxedo-rs_mod")
        .copyright("© 2024 TUXEDO Community")
        .license_type(gtk::License::Gpl20)
        .build();

    about.present();
}
