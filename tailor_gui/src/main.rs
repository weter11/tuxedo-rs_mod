mod config;

// Phase 1 modules
pub mod profile_system;
pub mod hardware_monitor;
pub mod keyboard_control;

// Phase 2 modules
pub mod hardware_control;
pub mod profile_controller;

// Phase 3 modules - ADD THIS
pub mod ui;

// Add this line to access MessageDialog methods like add_response
use adw::prelude::*;

use gtk::prelude::*;
use gtk::{gio, Application};
use adw;

const APP_ID: &str = "com.github.tuxedo.control";

fn main() -> glib::ExitCode {
    // Initialize GTK
    gtk::init().expect("Failed to initialize GTK");
    
    // Create application
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    // Connect startup signal
    app.connect_startup(|_| {
        adw::init().expect("Failed to initialize Libadwaita");
        load_css();
    });

    // Connect activate signal
    app.connect_activate(|app| {
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

        // Create and show main window
        let window = ui::main_window::MainWindow::new(app);
        window.present();
    });

    // Setup actions
    setup_actions(&app);

    // Run application
    app.run()
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
            // Show instructions for running as root
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

fn setup_actions(app: &Application) {
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

    // Quit action
    let quit_action = gio::SimpleAction::new("quit", None);
    // Make sure you have: use gtk::prelude::*; at the top of the file
quit_action.connect_activate(glib::clone!(@weak app => move |_, _| {
    app.quit();
}));
    app.add_action(&quit_action);
    
    app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
}

fn show_about_dialog() {
    let about = adw::AboutWindow::builder()
        .application_name("TUXEDO Control Center")
        .application_icon("computer")
        .version("0.3.0")
        .developer_name("TUXEDO Community")
        .issue_url("https://github.com/weter11/tuxedo-rs_mod/issues")
        .website("https://github.com/weter11/tuxedo-rs_mod")
        .copyright("© 2024 TUXEDO Community")
        .license_type(gtk::License::Gpl20)
        .build();

    about.present();
}
