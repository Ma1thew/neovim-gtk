extern crate gdk_pixbuf;
use gdk_pixbuf::Pixbuf;
use gtk;
use gtk::prelude::*;

use std::path::Path;
use std::env;

const ICON_FILE: &str = "text-x-generic-symbolic";
const ICON_SIZE: i32 = 16;

pub fn get_icon(names: Vec<&str>) -> Pixbuf {
    let icon_theme = gtk::IconTheme::get_default().unwrap();
    match get_icon_path() {
        Some(icon_path) => {
            for name in names {
                if Path::new(&format!("{}/{}.svg", icon_path, name)[..]).exists() {
                    return Pixbuf::new_from_file(format!("{}/{}.svg", icon_path, name)).unwrap();
                }
            }
            return icon_theme.load_icon(ICON_FILE, ICON_SIZE, gtk::IconLookupFlags::empty()).unwrap().unwrap();
        },
        None => return icon_theme.load_icon(ICON_FILE, ICON_SIZE, gtk::IconLookupFlags::empty()).unwrap().unwrap(),
    }
}

fn get_icon_path() -> Option<String> {
    if let Ok(icon_path) = env::var("NVIM_GTK_ICON_PATH") {
        Some(icon_path)
    } else if let Some(prefix) = option_env!("PREFIX") {
        Some(format!("{}/share/nvim-gtk/icons", prefix))
    } else {
        None
    }
}
