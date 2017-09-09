extern crate gtk;
extern crate glib;
extern crate opvault;
extern crate serde_json;

use std::error::Error;

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
    let cache_path = match glib::get_user_cache_dir() {
        Some(mut p) => {
            p.push("sekkrit");
            p
        },
        None => panic!("failed to find user cache dir"),
    };

    let key_file= glib::KeyFile::new();
    let res = key_file.load_from_file(cache_path.as_path(), glib::KeyFileFlags::empty());
    match res {
        Ok(p) => println!("found it at {:?}", p),
        Err(e) => println!("error {:?}", e.description()),
    };
    let preselected_vault = key_file.get_string("vault", "last").ok();

    let window = Window::new(WindowType::Toplevel);
    window.set_title("Sekkrit");
    window.set_default_size(350, 70);

    let vbox = Box::new(Orientation::Vertical, 0);
    window.add(&vbox);

    let path_input = Entry::new();
    if let Some(vault_path) = preselected_vault {
        path_input.set_text(&vault_path);
    }
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
        key_file.set_string("vault", "last", p);
        key_file.save_to_file(&cache_path);

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

    let folder_model = gtk::ListStore::new(&[String::static_type()]);
    let folder_tree = gtk::TreeView::new();
    let column = gtk::TreeViewColumn::new();
    let cell = gtk::CellRendererText::new();
    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", 0);
    folder_tree.set_headers_visible(false);
    folder_tree.append_column(&column);
    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    for (_uuid, folder) in &vault.folders {
        let overview = if let Ok(o) = folder.overview() {
            o
        } else {
            continue
        };
        let over_value: serde_json::Value = match serde_json::from_slice(&overview) {
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
                folder_model.insert_with_values(None, &[0], &[&title.clone()]);
            }
        }
    }

    folder_tree.connect_cursor_changed(move |tree_view| {
        let selection = tree_view.get_selection();
        if let Some((model, iter)) = selection.get_selected() {
            let title = model.get_value(&iter, 0).get::<String>();
            println!("title {:?}", title);
        }
    });

    folder_tree.set_model(Some(&folder_model));

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

    hbox.add(&folder_tree);
    hbox.add(&tv);
    w.add(&hbox);

    w
}
