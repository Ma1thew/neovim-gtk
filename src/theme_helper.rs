use gtk;
use gtk::SettingsExt;
use std::env;

pub fn is_dark_theme() -> bool {
    let force_dark_theme = env::var("NVIM_GTK_PREFER_DARK_THEME")
        .map(|opt| opt.trim() == "1")
        .unwrap_or(false);
    let settings = gtk::Settings::get_default().unwrap();
    let theme = settings.get_property_gtk_theme_name().unwrap();
    let theme = theme.as_str();
    let dark_theme_name = theme.ends_with("dark") || theme.ends_with("Inverse");
    let prefers_dark_theme = settings.get_property_gtk_application_prefer_dark_theme();
    force_dark_theme || dark_theme_name || prefers_dark_theme
}
