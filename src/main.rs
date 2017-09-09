extern crate gtk;
extern crate opvault;
extern crate serde_json;

use gtk::prelude::*;
use gtk::{Button, Window, WindowType, Box, Entry, Orientation, FileChooserDialog, FileChooserAction};

fn main() {
    if gtk::init().is_err() {
        println!("failed to init GK+");
        return;
    }

    let unlock_window = create_unlock_window();
    unlock_window.show_all();

    gtk::main();
}

fn create_unlock_window() -> Window {
    let window = Window::new(WindowType::Toplevel);
    window.set_title("Sekkrit");
    window.set_default_size(350, 70);

    let vbox = Box::new(Orientation::Vertical, 0);
    window.add(&vbox);

    let path_input = Entry::new();
    vbox.add(&path_input);

    let diag_button = Button::new_with_label("Select opvault");
    vbox.add(&diag_button);
    let input_clone = path_input.clone();
    let window_clone = window.clone();
    diag_button.connect_clicked(move |_| {
        let selector = FileChooserDialog::new::<Window>(
            Some("Select opvault directory"), Some(&window_clone), FileChooserAction::SelectFolder);
        selector.add_buttons(&[("Open", gtk::ResponseType::Ok.into()), ("Cancel", gtk::ResponseType::Cancel.into())]);
        selector.run();

        let files = selector.get_filenames();
        selector.destroy();

        input_clone.set_text(&*files[0].to_string_lossy());
    });

    let pw_input = Entry::new();
    pw_input.set_input_purpose(gtk::InputPurpose::Password);
    pw_input.set_visibility(false);
    vbox.add(&pw_input);

    let button = Button::new_with_label("Unlock");
    vbox.add(&button);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let input_clone = path_input.clone();
    let pw_clone = pw_input.clone();
    let window_clone = window.clone();
    button.connect_clicked(move |_| {
        let text = input_clone.get_text();
        let p = text.as_ref().unwrap();
        println!("path {}", p);
        let lv = opvault::LockedVault::open(std::path::Path::new(p)).unwrap();

        let pw = pw_clone.get_text();
        let pw = if let Some(text) = pw.as_ref() {
            text
        } else {
            ""
        };

        window_clone.destroy();
        let vault = lv.unlock(pw.as_bytes()).unwrap();
        let main_window = create_main_window(vault);
        main_window.show_all();

    });

    window
}

fn create_main_window(vault: opvault::UnlockedVault) -> Window {
    let w = Window::new(WindowType::Toplevel);
    w.set_title("Sekkrit");
    w.set_default_size(350, 350);

    w.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let store = gtk::ListStore::new(&[gtk::Type::String]);

    let tv = gtk::ListBox::new();
    for item in vault.get_items() {
        if let Ok(bin) = item.overview() {
            let over_value: serde_json::Value = match serde_json::from_slice(bin.as_slice()) {
                Ok(o) => o,
                Err(_) => continue,
            };

            let over = if let Some(obj) = over_value.as_object() {
                obj
            } else {
                continue
            };

            if let Some(title_value) = over.get("title") {
                if let Some(title) = title_value.as_str() {
                    let label = gtk::Label::new_with_mnemonic(None);
                    label.set_label(title);
                    tv.insert(&label, -1);
                }
            }
        }
    }
    w.add(&tv);


    w
}
