extern crate gtk;
extern crate glib;
extern crate opvault;
extern crate serde_json;

use std::error::Error;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Button, Window, WindowType, Box, Entry, Orientation, FileChooserDialog, FileChooserAction};
use opvault::Uuid;

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
    if let Err(e) = key_file.load_from_file(cache_path.as_path(), glib::KeyFileFlags::empty()) {
        println!("found no cache file: {}", e.description());
    }
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
        if let Err(e) = key_file.save_to_file(&cache_path) {
            println!("failed to save cache: {}", e.description());
        };

        window_clone.destroy();
        let vault = lv.unlock(pw.as_bytes()).unwrap();
        let main_window = create_main_window(vault);
        main_window.show_all();

    });

    window
}

fn create_main_window(vault: opvault::UnlockedVault) -> Window {
    let vault = Rc::new(vault);

    let w = Window::new(WindowType::Toplevel);
    w.set_title("Sekkrit");
    w.set_default_size(350, 350);

    w.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let item_model = gtk::ListStore::new(&[String::static_type(), String::static_type(), String::static_type(), bool::static_type()]);
    let filter_item_model = gtk::TreeModelFilter::new(&item_model, None);
    let folder_model = gtk::ListStore::new(&[String::static_type(), String::static_type()]);
    let folder_tree = gtk::TreeView::new();
    let column = gtk::TreeViewColumn::new();
    let cell = gtk::CellRendererText::new();
    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", 0);
    folder_tree.set_headers_visible(false);
    folder_tree.append_column(&column);

    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    folder_model.insert_with_values(None, &[0], &[&String::from("All")]);
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
                folder_model.insert_with_values(None, &[0, 1], &[&title.clone(), &folder.uuid.to_string()]);
            }
        }
    }

    let item_model_clone = item_model.clone();
    let filter_item_model_clone = filter_item_model.clone();
    let vault_clone = vault.clone();
    folder_tree.connect_cursor_changed(move |tree_view| {
        let selection = tree_view.get_selection();
        if let Some((model, iter)) = selection.get_selected() {
            let title = model.get_value(&iter, 0).get::<String>().unwrap();
            let uuid = if title == "All" {
                None
            } else {
                let str_uuid = model.get_value(&iter, 1).get::<String>().unwrap();
                Uuid::parse_str(&str_uuid).ok()
            };
            filter_items(vault_clone.clone(), &item_model_clone, uuid);
            filter_item_model_clone.refilter();
        }
    });

    folder_tree.set_model(Some(&folder_model));

    filter_item_model.set_visible_column(3);

    let item_tree = gtk::TreeView::new();
    item_tree.set_headers_visible(false);

    let column = gtk::TreeViewColumn::new();
    let cell = gtk::CellRendererPixbuf::new();
    column.pack_start(&cell, true);
    column.add_attribute(&cell, "icon-name", 0);
    item_tree.append_column(&column);

    let column = gtk::TreeViewColumn::new();
    let cell = gtk::CellRendererText::new();
    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", 1);
    item_tree.append_column(&column);

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
                    let stock_id = item_stock_icon(&item);
                    item_model.insert_with_values(None, &[0, 1, 2],  &[&stock_id.to_string(), &title.clone(), &item.uuid.to_string()]);
                }
            }
        }
    }

    item_tree.set_model(Some(&filter_item_model));

    hbox.add(&folder_tree);
    hbox.add(&item_tree);
    w.add(&hbox);

    w
}

/// Filter down the items in the model to those which have the folder with the given UUID.
fn filter_items(vault: Rc<opvault::UnlockedVault>, model: &gtk::ListStore, uuid: Option<Uuid>) {
    let iter = match model.get_iter_first() {
        Some(i) => i,
        None => return,
    };

    let mut has_next = true;
    while has_next {
        let item_uuid_str = model.get_value(&iter, 2).get::<String>().unwrap();
        let item_uuid = Uuid::parse_str(&item_uuid_str).unwrap();
        let item = vault.get_item(&item_uuid).unwrap();
        let visible = if let Some(filter) = uuid {
            if let Some(item_folder_uuid) = item.folder {
                filter == item_folder_uuid
            } else {
                false
            }
        } else {
            true
        };

        model.set_value(&iter, 3, &visible.to_value());
        has_next = model.iter_next(&iter);
    }
}

fn item_stock_icon(item: &opvault::Item) -> &'static str {
    use opvault::Category::*;
    match item.category {
        Login | Password => "dialog-password",
        _ => "pda",
    }
}
