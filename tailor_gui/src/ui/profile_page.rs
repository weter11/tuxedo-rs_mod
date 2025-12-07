// src/ui/profile_page.rs
use gtk::prelude::*;
use gtk::{Box, Button, Label, Orientation, ListBox, ScrolledWindow, Frame};
use adw::prelude::*;
use adw::{ActionRow, PreferencesGroup};
use std::sync::{Arc, Mutex};
use crate::profile_controller::ProfileController;
use crate::profile_system::Profile;

pub struct ProfilePage {
    pub widget: Box,
    controller: Arc<Mutex<ProfileController>>,
    profile_list: ListBox,
    apply_button: Button,
    delete_button: Button,
    selected_profile_index: Arc<Mutex<Option<usize>>>,
}

impl ProfilePage {
    pub fn new(controller: Arc<Mutex<ProfileController>>) -> Self {
        let main_box = Box::new(Orientation::Vertical, 12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);

        // Title
        let header_box = Box::new(Orientation::Horizontal, 12);
        let title = Label::new(Some("Profiles"));
        title.add_css_class("title-1");
        title.set_halign(gtk::Align::Start);
        title.set_hexpand(true);

        let add_button = Button::with_label("New Profile");
        add_button.add_css_class("suggested-action");

        header_box.append(&title);
        header_box.append(&add_button);
        main_box.append(&header_box);

        // Profile list
        let scrolled = ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_min_content_height(300);

        let profile_list = ListBox::new();
        profile_list.add_css_class("boxed-list");
        scrolled.set_child(Some(&profile_list));
        main_box.append(&scrolled);

        // Action buttons
        let button_box = Box::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk::Align::End);

        let apply_button = Button::with_label("Apply Profile");
        apply_button.add_css_class("suggested-action");
        apply_button.set_sensitive(false);

        let edit_button = Button::with_label("Edit");
        edit_button.set_sensitive(false);

        let delete_button = Button::with_label("Delete");
        delete_button.add_css_class("destructive-action");
        delete_button.set_sensitive(false);

        button_box.append(&apply_button);
        button_box.append(&edit_button);
        button_box.append(&delete_button);
        main_box.append(&button_box);

        let selected_profile_index = Arc::new(Mutex::new(None));

        let mut page = ProfilePage {
            widget: main_box,
            controller: controller.clone(),
            profile_list: profile_list.clone(),
            apply_button: apply_button.clone(),
            delete_button: delete_button.clone(),
            selected_profile_index: selected_profile_index.clone(),
        };

        page.refresh_profile_list();
        page.setup_signals(add_button, edit_button);

        page
    }

    fn refresh_profile_list(&self) {
        // Clear existing items
        while let Some(child) = self.profile_list.first_child() {
            self.profile_list.remove(&child);
        }

        // Get profiles
        let ctrl = self.controller.lock().unwrap();
        let profiles = ctrl.get_all_profiles();
        let active_profile = ctrl.get_active_profile();
        drop(ctrl);

        // Add each profile to the list
        for (index, profile) in profiles.iter().enumerate() {
            let row = ActionRow::new();
            row.set_title(&profile.name);

            // Create subtitle with profile info
            let mut subtitle_parts = Vec::new();

            // Keyboard info
            let kbd = &profile.keyboard_backlight;
            subtitle_parts.push(format!(
                "KB: RGB({},{},{}) @ {}%",
                kbd.color.r, kbd.color.g, kbd.color.b, kbd.brightness
            ));

            // CPU info
            subtitle_parts.push(format!("CPU: {:?}", profile.cpu_settings.performance_profile));

            // Auto-switch info
            if profile.auto_switch_enabled && !profile.trigger_apps.is_empty() {
                subtitle_parts.push(format!("Auto: {}", profile.trigger_apps.join(", ")));
            }

            row.set_subtitle(&subtitle_parts.join(" • "));

            // Mark active profile
            if profile.name == active_profile.name {
                row.add_suffix(&{
                    let badge = Label::new(Some("Active"));
                    badge.add_css_class("success");
                    badge.add_css_class("badge");
                    badge
                });
            }

            // Mark default profile
            if profile.is_default {
                row.add_suffix(&{
                    let badge = Label::new(Some("Default"));
                    badge.add_css_class("accent");
                    badge.add_css_class("badge");
                    badge
                });
            }

            self.profile_list.append(&row);
        }
    }

    fn setup_signals(&mut self, add_button: Button, edit_button: Button) {
        // Profile selection
        let apply_button = self.apply_button.clone();
        let delete_button = self.delete_button.clone();
        let selected_index = Arc::clone(&self.selected_profile_index);
        let controller = Arc::clone(&self.controller);
        let profile_list = self.profile_list.clone();

        self.profile_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let index = row.index() as usize;
                *selected_index.lock().unwrap() = Some(index);
                apply_button.set_sensitive(true);
                edit_button.set_sensitive(true);

                // Check if it's the default profile
                let ctrl = controller.lock().unwrap();
                let profiles = ctrl.get_all_profiles();
                let is_default = profiles.get(index).map(|p| p.is_default).unwrap_or(false);
                drop(ctrl);

                delete_button.set_sensitive(!is_default);
            } else {
                *selected_index.lock().unwrap() = None;
                apply_button.set_sensitive(false);
                edit_button.set_sensitive(false);
                delete_button.set_sensitive(false);
            }
        });

        // Apply button
        let controller = Arc::clone(&self.controller);
        let selected_index = Arc::clone(&self.selected_profile_index);
        let profile_list_clone = self.profile_list.clone();
        let widget = self.widget.clone();

        self.apply_button.connect_clicked(move |_| {
            if let Some(index) = *selected_index.lock().unwrap() {
                let ctrl = controller.lock().unwrap();
                if let Err(e) = ctrl.apply_profile(index) {
                    eprintln!("Failed to apply profile: {}", e);
                    show_error_dialog(&widget, "Failed to apply profile", &e.to_string());
                } else {
                    drop(ctrl);
                    // Refresh list to update "Active" badge
                    // Note: Would need to store self reference to call refresh_profile_list
                    println!("Profile applied successfully");
                }
            }
        });

        // Delete button
        let controller = Arc::clone(&self.controller);
        let selected_index = Arc::clone(&self.selected_profile_index);
        let widget = self.widget.clone();

        self.delete_button.connect_clicked(move |_| {
            if let Some(index) = *selected_index.lock().unwrap() {
                // Show confirmation dialog
                let dialog = adw::MessageDialog::new(
                    widget.root().and_downcast_ref::<gtk::Window>(),
                    Some("Delete Profile?"),
                    Some("This action cannot be undone."),
                );
                dialog.add_response("cancel", "Cancel");
                dialog.add_response("delete", "Delete");
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                let ctrl = Arc::clone(&controller);
                dialog.connect_response(None, move |dialog, response| {
                    if response == "delete" {
                        let mut ctrl = ctrl.lock().unwrap();
                        if let Err(e) = ctrl.delete_profile(index) {
                            eprintln!("Failed to delete profile: {}", e);
                        }
                    }
                    dialog.close();
                });

                dialog.present();
            }
        });

        // Add button
        let controller = Arc::clone(&self.controller);
        let widget = self.widget.clone();

        add_button.connect_clicked(move |_| {
            // For now, show a simple dialog
            // In a full implementation, this would open a profile editor
            show_info_dialog(
                &widget,
                "New Profile",
                "Profile editor will be implemented in the tuning page",
            );
        });

        // Edit button
        let widget = self.widget.clone();
        edit_button.connect_clicked(move |_| {
            show_info_dialog(
                &widget,
                "Edit Profile",
                "Profile editor will be implemented in the tuning page",
            );
        });
    }
}

fn show_error_dialog(widget: &Box, title: &str, message: &str) {
    let dialog = adw::MessageDialog::new(
        widget.root().and_downcast_ref::<gtk::Window>(),
        Some(title),
        Some(message),
    );
    dialog.add_response("ok", "OK");
    dialog.present();
}

fn show_info_dialog(widget: &Box, title: &str, message: &str) {
    let dialog = adw::MessageDialog::new(
        widget.root().and_downcast_ref::<gtk::Window>(),
        Some(title),
        Some(message),
    );
    dialog.add_response("ok", "OK");
    dialog.present();
}
