extern crate gtk;
extern crate opvault;

use gtk::prelude::*;
use gtk::{Button, Window, WindowType, Box, Entry, Orientation, FileChooserDialog, FileChooserAction};

fn main() {
    if gtk::init().is_err() {
        println!("failed to init GK+");
        return;
    }

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
    window.show_all();

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
        for item in vault.get_items() {
            if let Ok(bin) = item.overview() {
                println!("{}", String::from_utf8_lossy(bin.as_slice()));
            }
        }
    });

    gtk::main();
}
