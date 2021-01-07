extern crate gdk_pixbuf;
use gdk_pixbuf::Pixbuf;
use gtk;
use gtk::prelude::*;

use std::path::Path;
use std::env;

const ICON_FOLDER_CLOSED: &str = "folder-symbolic";
const ICON_FOLDER_OPEN: &str = "folder-open-symbolic";
const ICON_FILE: &str = "text-x-generic-symbolic";
const ICON_SIZE: i32 = 16;

pub fn get_icon(names: Vec<&str>) -> Pixbuf {
    let icon_theme = gtk::IconTheme::get_default().unwrap();
    let lookup_flags = gtk::IconLookupFlags::empty();
    match get_icon_path() {
        Some(icon_path) => {
            for name in names {
                if Path::new(&format!("{}/{}.svg", icon_path, name)[..]).exists() {
                    return maybe_invert_pixbuf(Pixbuf::new_from_file(format!("{}/{}.svg", icon_path, name)).unwrap());
                }
            }
            return maybe_invert_pixbuf(icon_theme.load_icon(ICON_FILE, ICON_SIZE, lookup_flags).unwrap().unwrap());
        },
        None => return maybe_invert_pixbuf(icon_theme.load_icon(ICON_FILE, ICON_SIZE, lookup_flags).unwrap().unwrap()),
    }
}

pub fn get_folder_open() -> Pixbuf {
    let icon_theme = gtk::IconTheme::get_default().unwrap();
    let icon = icon_theme.load_icon(ICON_FOLDER_OPEN, ICON_SIZE, gtk::IconLookupFlags::empty()).unwrap().unwrap();
    maybe_invert_pixbuf(icon)
}

pub fn get_folder_closed() -> Pixbuf {
    let icon_theme = gtk::IconTheme::get_default().unwrap();
    let icon = icon_theme.load_icon(ICON_FOLDER_CLOSED, ICON_SIZE, gtk::IconLookupFlags::empty()).unwrap().unwrap();
    maybe_invert_pixbuf(icon)
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

fn is_black_monochrome(input: &Pixbuf) -> bool {
    unsafe {
        let bytes = input.get_pixels();
        for i in 0..bytes.len() {
            if ((i + 1) % 4 != 0) && bytes[i] != 0 {
                return false;
            }
        }
        return true;
    }
}

fn maybe_invert_pixbuf(input: Pixbuf) -> Pixbuf {
    let prefer_dark_theme = env::var("NVIM_GTK_PREFER_DARK_THEME")
        .map(|opt| opt.trim() == "1")
        .unwrap_or(false);
    if prefer_dark_theme && is_black_monochrome(&input) {
        return invert_pixbuf(&input);
    } else {
        let theme = gtk::Settings::get_default().unwrap().get_property_gtk_theme_name().unwrap();
        let theme = theme.as_str();
        if (theme.ends_with("dark") || theme.ends_with("Inverse")) && is_black_monochrome(&input) {
            return invert_pixbuf(&input);
        } else {
            return input;
        }
    }
}

fn invert_pixbuf(input: &Pixbuf) -> Pixbuf {
    unsafe {
        let bytes = input.get_pixels();
        let height = input.get_height();
        let width = input.get_width();
        let bpp = 4;
        let rowstride = width * bpp;
        let new_pixbuf = input.copy().unwrap();
        for y in 0..height {
            for x in 0..width {
                let r = 255 - bytes[x as usize * bpp as usize + (y as usize * rowstride as usize) + 0];
                let g = 255 - bytes[x as usize * bpp as usize + (y as usize * rowstride as usize) + 1];
                let b = 255 - bytes[x as usize * bpp as usize + (y as usize * rowstride as usize) + 2];
                let a = bytes[x as usize * bpp as usize + (y as usize * rowstride as usize) + 3];
                new_pixbuf.put_pixel(x, y, r, g, b, a)
            }
        }
        new_pixbuf
    }
/*  int ht,wt;
  int i=0,j=0;
  int rowstride=0;  
  int bpp=0;
  gchar *pixel;


  if(gdk_pixbuf_get_bits_per_sample(pb)!=8)   //we handle only 24 bit images.
  	return;                               //look at 3 bytes per pixel.

  bpp=3;	         	  //getting attributes of height,
  ht=gdk_pixbuf_get_height(pb);   //width, and bpp.Also get pointer
  wt=gdk_pixbuf_get_width(pb);	  //to pixels.
  pixel=gdk_pixbuf_get_pixels(pb);
  rowstride=wt*bpp;

  for(i=0;i<ht;i++)		//iterate over the height of image.
    for(j=0;j<rowstride;j+=bpp)   //read every pixel in the row.skip
				//bpp bytes 
      {	
      
      //access pixel[i][j] as
      // pixel[i*rowstride + j]

      //access red,green and blue as
	pixel[i*rowstride + j+0]=255-pixel[i*rowstride + j+0];
	pixel[i*rowstride + j+1]=255-pixel[i*rowstride + j+1];
	pixel[i*rowstride + j+2]=255-pixel[i*rowstride + j+2];	      
      }  
  return;*/
}
