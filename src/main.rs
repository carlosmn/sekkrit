extern crate gtk;
extern crate gdk;
extern crate glib;
extern crate opvault;
extern crate serde_json;
extern crate serde;

use std::error::Error;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Button, Window, WindowType, Box, Entry, Orientation, FileChooserDialog, FileChooserAction};
use opvault::{Uuid, Detail, LoginFieldKind};

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
        let ok_response: i32 = gtk::ResponseType::Ok.into();
        selector.add_buttons(&[("Open", ok_response), ("Cancel", gtk::ResponseType::Cancel.into())]);
        let res: i32 = selector.run();
        if res == ok_response {
            if let Some(file) = selector.get_filename() {
                input_clone.set_text(&file.to_string_lossy());
            }
        }

        selector.destroy();
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
    item_model.set_sort_column_id(gtk::SortColumn::Index(1), gtk::SortType::Ascending);

    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    folder_model.insert_with_values(None, &[0], &[&String::from("All")]);
    for (_uuid, folder) in &vault.folders {
        let overview = if let Ok(o) = folder.overview() {
            o
        } else {
            continue
        };

        let title = &overview.title;
        folder_model.insert_with_values(None, &[0, 1], &[&title.clone(), &folder.uuid.to_string()]);
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
        if let Ok(overview) = item.overview() {
            if let Some(title) = overview.title {
                let stock_id = item_stock_icon(&item);
                item_model.insert_with_values(None, &[0, 1, 2],  &[&stock_id.to_string(), &title.clone(), &item.uuid.to_string()]);
            }
        }
    }

    let details_scrolled = gtk::ScrolledWindow::new(None, None);
    details_scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    let details_scrolled_clone = details_scrolled.clone();

    let vault_clone = vault.clone();
    item_tree.connect_cursor_changed(move |tree_view| {
        let selection = tree_view.get_selection();
        if let Some((model, iter)) = selection.get_selected() {
            let uuid_str = model.get_value(&iter, 2).get::<String>().unwrap();
            let uuid = Uuid::parse_str(&uuid_str).unwrap();

            let item = match vault_clone.get_item(&uuid) {
                Some(i) => i,
                None => return,
            };

            println!("category {:?}", item.category);
            let detail = match item.detail() {
                Ok(d) => d,
                Err(e) => {
                    println!("{:?}", e);
                    return
                }
            };
            println!("overview {:?}", item.overview());
            println!("detail {:?}", detail);

            for c in details_scrolled_clone.get_children() {
                details_scrolled_clone.remove(&c);
            }

            let grid = grid_from_details(detail);
            let ebox = gtk::EventBox::new();
            ebox.add(&grid);
            let css_rule = b".details_view { background-color: white; }";
            let css_provider = gtk::CssProvider::new();
            gtk::CssProviderExt::load_from_data(&css_provider, css_rule).unwrap();
            if let Some(style_ctx) = ebox.get_style_context() {
                style_ctx.add_class("details_view");
                style_ctx.add_provider(&css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            }

            details_scrolled_clone.add_with_viewport(&ebox);
            details_scrolled_clone.show_all();
        }
    });

    item_tree.set_model(Some(&filter_item_model));

    let scrolled = gtk::ScrolledWindow::new(None, None);
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.add_with_viewport(&item_tree);

    hbox.add(&folder_tree);
    hbox.add(&scrolled);
    hbox.add(&details_scrolled);
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
        Identity => "vcard",
        Tombstone => "edit-delete",
        Database => "drive-multidisk",
        Email => "mail-read",
        _ => "pda",
    }
}

fn grid_from_details(d: Detail) -> gtk::Grid {
    let grid = gtk::Grid::new();

    match d {
        Detail::Login(l) => {
            for (n, f) in l.fields.iter().enumerate() {
                match f.kind {
                    LoginFieldKind::Password  => {
                        let name = f.designation.as_ref().unwrap_or(&f.name);
                        let password = f.value.clone();

                        insert_password(&grid, n as i32, name, password);
                    },
                    LoginFieldKind::Text if f.name == "password" => {
                        let name = f.designation.as_ref().unwrap_or(&f.name);
                        let password = f.value.clone();

                        insert_password(&grid, n as i32, name, password);
                    },
                    LoginFieldKind::Text | LoginFieldKind::I | LoginFieldKind::Email => {
                        let name = f.designation.as_ref().unwrap_or(&f.name);
                        let value = &f.value;

                        let name_box = gtk::EventBox::new();
                        let name_label = gtk::Label::new(Some(name.as_str()));
                        name_box.add(&name_label);
                        grid.attach(&name_box, 0, n as i32, 1, 1);

                        let value_box = gtk::EventBox::new();
                        let value_label = gtk::Label::new(Some(value.as_str()));
                        value_box.add(&value_label);
                        grid.attach(&value_box, 1, n as i32, 1, 1);
                    },
                    LoginFieldKind::Checkbox | LoginFieldKind::Button | LoginFieldKind::S => {},
                };
            }
        }
        Detail::Password(p) => {
            insert_password(&grid, 0, "pasword", p.password);
        }
        Detail::Generic(g) => {
            let mut pos = 0i32;
            for s in g.sections {
                if s.name != "" && s.fields.len() != 0 {
                    let ebox = gtk::EventBox::new();
                    let label = gtk::Label::new(None);
                    label.set_markup(&format!("<b>{}</b>", s.title));
                    ebox.add(&label);
                    grid.attach(&ebox, 0, pos, 2, 1);
                    pos += 1;
                }

                for f in s.fields {
                    if f.value.is_none() {
                        continue;
                    }

                    let ebox = gtk::EventBox::new();
                    let label = gtk::Label::new(Some(f.name.as_str()));
                    ebox.add(&label);
                    grid.attach(&ebox, 0, pos, 1, 1);

                    let ebox = gtk::EventBox::new();
                    let label = gtk::Label::new(Some(format!("{:?}", f.value).as_str()));
                    ebox.add(&label);
                    grid.attach(&ebox, 1, pos, 1, 1);
                    pos += 1;
                }
            }
        }
    };

    grid
}

fn insert_password(grid: &gtk::Grid, n: i32, name: &str, password: String) {
    let name_box = gtk::EventBox::new();
    let name_label = gtk::Label::new(Some(name));
    name_box.add(&name_label);
    grid.attach(&name_box, 0, n as i32, 1, 1);

    let hbox = gtk::Box::new(Orientation::Horizontal, 0);

    let value_text = gtk::Entry::new();
    value_text.set_text(&"sekkrit");
    value_text.set_visibility(false);
    value_text.set_editable(false);
    hbox.add(&value_text);

    let copy_button = gtk::Button::new_from_icon_name("edit-copy", gtk::IconSize::Button.into());
    copy_button.connect_clicked(move |_button| {
        let cp = gtk::Clipboard::get(&gdk::Atom::intern("CLIPBOARD"));
        cp.set_text(&password);
    });

    hbox.add(&copy_button);

    grid.attach(&hbox, 1, n as i32, 1, 1);
}
