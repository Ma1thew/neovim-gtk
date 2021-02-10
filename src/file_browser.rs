use std::cell::RefCell;
use std::cmp::Ordering;
use std::io;
use std::fs;
use std::fs::DirEntry;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::ops::Deref;

use gio;
use gio::prelude::*;
use gtk;
use gtk::prelude::*;

use neovim_lib::{NeovimApi, NeovimApiAsync};
use htmlescape::encode_minimal;

use crate::misc::{escape_filename, substitute_home_for_tilde};
use crate::nvim::{ErrorReport, NeovimClient, NeovimRef};
use crate::shell;
use crate::subscriptions::SubscriptionKey;

const ICON_FOLDER_OPEN: &str = "folder-open-symbolic";
const ICON_FOLDER_CLOSED: &str = "folder-symbolic";
const ICON_FILE: &str = "text-x-generic-symbolic";

struct Components {
    dir_list_model: gtk::TreeStore,
    dir_list: gtk::ComboBox,
    buf_list: gtk::TreeStore,
    buf_tree_view: gtk::TreeView,
    context_menu: gtk::Menu,
    show_hidden_checkbox: gtk::CheckMenuItem,
    cd_action: gio::SimpleAction,
}

struct State {
    current_dir: String,
    show_hidden: bool,
    selected_path: Option<String>,
}

pub struct FileBrowserWidget {
    store: gtk::TreeStore,
    tree: gtk::TreeView,
    widget: gtk::Box,
    nvim: Option<Rc<NeovimClient>>,
    comps: Components,
    state: Rc<RefCell<State>>,
}

impl Deref for FileBrowserWidget {
    type Target = gtk::Box;

    fn deref(&self) -> &gtk::Box {
        &self.widget
    }
}

#[derive(Copy, Clone, Debug)]
enum FileType {
    File,
    Dir,
}

#[allow(dead_code)]
enum Column {
    Filename,
    Path,
    FileType,
    IconName,
}

impl FileBrowserWidget {
    pub fn new() -> Self {
        let builder = gtk::Builder::new_from_string(include_str!("../resources/side-panel.ui"));
        let widget: gtk::Box = builder.get_object("file_browser").unwrap();
        let tree: gtk::TreeView = builder.get_object("file_browser_tree_view").unwrap();
        let store: gtk::TreeStore = builder.get_object("file_browser_tree_store").unwrap();
        let dir_list_model: gtk::TreeStore = builder.get_object("dir_list_model").unwrap();
        let dir_list: gtk::ComboBox = builder.get_object("dir_list").unwrap();
        let context_menu: gtk::Menu = builder.get_object("file_browser_context_menu").unwrap();
        let buf_list: gtk::TreeStore = builder.get_object("buf_list").unwrap();
        let buf_tree_view: gtk::TreeView = builder.get_object("buf_tree_view").unwrap();
        let show_hidden_checkbox: gtk::CheckMenuItem = builder
            .get_object("file_browser_show_hidden_checkbox")
            .unwrap();

        let file_browser = FileBrowserWidget {
            store,
            tree,
            widget,
            nvim: None,
            comps: Components {
                dir_list_model,
                dir_list,
                buf_list,
                buf_tree_view,
                context_menu,
                show_hidden_checkbox,
                cd_action: gio::SimpleAction::new("cd", None),
            },
            state: Rc::new(RefCell::new(State {
                current_dir: "".to_owned(),
                show_hidden: false,
                selected_path: None,
            })),
        };
        file_browser
    }

    fn nvim(&self) -> Option<NeovimRef> {
        self.nvim.as_ref().unwrap().nvim()
    }

    pub fn init(&mut self, shell_state: &shell::State) {
        // Initialize values.
        let nvim = shell_state.nvim_clone();
        self.nvim = Some(nvim);
        if let Some(dir) = get_current_dir(&mut self.nvim().unwrap()) {
            update_dir_list(&dir, &self.comps.dir_list_model, &self.comps.dir_list);
            self.state.borrow_mut().current_dir = dir;
        }

        // Populate tree.
        tree_reload(&self.store, &self.state.borrow());

        let store = &self.store;
        let state_ref = &self.state;
        self.tree.connect_test_expand_row(clone!(store, state_ref => move |_, iter, _| {
            store.set(&iter, &[Column::IconName as u32], &[&ICON_FOLDER_OPEN]);
            // We cannot recursively populate all directories. Instead, we have prepared a single
            // empty child entry for all non-empty directories, so the row will be expandable. Now,
            // when a directory is expanded, populate its children.
            let state = state_ref.borrow();
            if let Some(child) = store.iter_children(Some(iter)) {
                let filename = store.get_value(&child, Column::Filename as i32);
                if filename.get::<&str>().is_none() {
                    store.remove(&child);
                    let dir_value = store.get_value(&iter, Column::Path as i32);
                    if let Some(dir) = dir_value.get() {
                        populate_tree_nodes(&store, &state, dir, Some(iter));
                    }
                } else {
                    // This directory is already populated, i.e. it has been expanded and collapsed
                    // again. Rows further down the tree might have been silently collapsed without
                    // getting an event. Update their folder icon.
                    let mut tree_path = store.get_path(&child).unwrap();
                    while let Some(iter) = store.get_iter(&tree_path) {
                        tree_path.next();
                        let file_type = store
                            .get_value(&iter, Column::FileType as i32)
                            .get::<u8>();
                        if file_type == Some(FileType::Dir as u8) {
                            store.set(&iter, &[Column::IconName as u32], &[&ICON_FOLDER_CLOSED]);
                        }
                    }
                }
            }
            Inhibit(false)
        }));

        self.tree.connect_row_collapsed(clone!(store => move |_, iter, _| {
            store.set(&iter, &[Column::IconName as u32], &[&ICON_FOLDER_CLOSED]);
        }));


        let nvim_ref = self.nvim.as_ref().unwrap();
        let buf_list = &self.comps.buf_list;
        let buf_tree = &self.comps.buf_tree_view;
        self.comps.buf_tree_view.connect_row_activated(clone!(nvim_ref, buf_list, buf_tree => move |_, path, col| {
            let buf_num = buf_list.get_value(&buf_list.get_iter(path).unwrap(), 1).get::<u32>().unwrap();
            let mut nvim = nvim_ref.nvim().unwrap();
            if col.get_title().unwrap().as_str() == "close" {
                if ! (buf_list.get_value(&buf_list.get_iter(path).unwrap(), 3).get::<&str>().unwrap() == "edit-delete-symbolic") {
                    nvim.session.call("nvim_buf_delete", vec![neovim_lib::Value::from(buf_num), neovim_lib::Value::Map(vec![])]).unwrap();
                }
            } else {
               if let Err(_) = nvim.set_current_buf(&neovim_lib::neovim_api::Buffer::new(neovim_lib::Value::Integer(neovim_lib::Integer::from(buf_num)))) {
                    if let Ok(new_buf) = nvim.get_current_buf() {
                        if let Ok(new_buf_num) = new_buf.get_number(&mut nvim) {
                           let mut tree_path = gtk::TreePath::new();
                            tree_path.down();
                            while let Some(iter) = buf_list.get_iter(&tree_path) {
                                if new_buf_num as u32 == buf_list.get_value(&iter, 1).get::<u32>().unwrap() {
                                    buf_tree.set_cursor(&tree_path, Option::<&gtk::TreeViewColumn>::None, false);
                                    break;
                                }
                                tree_path.next();
                            }
                        }
                    }
                }
            }
        }));

        // Further initialization.
        self.init_actions();
        self.init_subscriptions(shell_state);
        self.connect_events();
    }

    fn init_actions(&self) {
        let actions = gio::SimpleActionGroup::new();

        let store = &self.store;
        let state_ref = &self.state;
        let nvim_ref = self.nvim.as_ref().unwrap();

        let reload_action = gio::SimpleAction::new("reload", None);
        reload_action.connect_activate(clone!(store, state_ref => move |_, _| {
            tree_reload(&store, &state_ref.borrow());
        }));
        actions.add_action(&reload_action);

        let cd_action = &self.comps.cd_action;
        cd_action.connect_activate(clone!(state_ref, nvim_ref => move |_, _| {
            let mut nvim = nvim_ref.nvim().unwrap();
            if let Some(ref path) = state_ref.borrow().selected_path {
                nvim.set_current_dir_async(&path)
                    .cb(|r| r.report_err())
                    .call();
            }
        }));
        actions.add_action(cd_action);

        self.comps
            .context_menu
            .insert_action_group("filebrowser", Some(&actions));
    }

    fn init_subscriptions(&self, shell_state: &shell::State) {
        // Always set the current working directory as the root of the file browser.
        let store = &self.store;
        let state_ref = &self.state;
        let dir_list_model = &self.comps.dir_list_model;
        let dir_list = &self.comps.dir_list;
        shell_state.subscribe(
            SubscriptionKey::from("DirChanged"),
            &["getcwd()"],
            clone!(store, state_ref, dir_list_model, dir_list => move |args| {
                let dir = args.into_iter().next().unwrap();
                if dir != state_ref.borrow().current_dir {
                    state_ref.borrow_mut().current_dir = dir.to_owned();
                    update_dir_list(&dir, &dir_list_model, &dir_list);
                    tree_reload(&store, &state_ref.borrow());
                }
            }),
        );

        // Reveal the file of an entered buffer in the file browser and select the entry.
        let tree = &self.tree;
        let subscription = shell_state.subscribe(
            SubscriptionKey::from("BufEnter"),
            &["getcwd()", "expand('%:p')"],
            clone!(tree, store => move |args| {
                let mut args_iter = args.into_iter();
                let dir = args_iter.next().unwrap();
                let file_path = args_iter.next().unwrap();
                let could_reveal =
                    if let Ok(rel_path) = Path::new(&file_path).strip_prefix(&Path::new(&dir)) {
                        reveal_path_in_tree(&store, &tree, &rel_path)
                    } else {
                        false
                    };
                if !could_reveal {
                    tree.get_selection().unselect_all();
                }
            }),
        );
        shell_state.run_now(&subscription);

        let buf_list = &self.comps.buf_list;
        let nvim_ref = self.nvim.as_ref().unwrap();
        shell_state.subscribe(SubscriptionKey::from("BufAdd"), &["expand('<abuf>')", "expand('<afile>')"], clone!(buf_list, nvim_ref => move |args| {
            let mut nvim = nvim_ref.nvim().unwrap();
            let mut args = args.into_iter();
            if let Some(buf_num) = args.next() {
                if let Ok(num) = buf_num.parse::<u32>() {
                    if let Some(name) = args.next() {
                        if let Ok(listed) = neovim_lib::neovim_api::Buffer::new(neovim_lib::Value::Integer(neovim_lib::Integer::from(num))).get_option(&mut nvim, "buflisted") {
                            if listed.as_bool().unwrap_or(false) {
                                update_buf_list_added(&buf_list, num, &name);
                            }
                        }
                    }
                }
            }
        }));

        let buf_list = &self.comps.buf_list;
        shell_state.subscribe(SubscriptionKey::from("BufDelete"), &["expand('<abuf>')"], clone!(buf_list => move |args| {
            let mut args = args.into_iter();
            if let Some(buf_num) = args.next() {
                if let Ok(num) = buf_num.parse::<u32>() {
                    update_buf_list_deleted(&buf_list, num);
                }
            }
        }));

        let buf_list = &self.comps.buf_list;
        shell_state.subscribe(SubscriptionKey::from("BufFilePost"), &["expand('<abuf>')", "expand('<afile>')"], clone!(buf_list => move |args| {
            let mut args = args.into_iter();
            if let Some(buf_num) = args.next() {
                if let Ok(num) = buf_num.parse::<u32>() {
                    if let Some(name) = args.next() {
                        update_buf_list_rename(&buf_list, num, &name);
                    }
                }
            }
        }));

        let buf_list = &self.comps.buf_list;
        let nvim_ref = self.nvim.as_ref().unwrap();
        shell_state.subscribe(SubscriptionKey::from("BufModifiedSet"), &["expand('<abuf>')"], clone!(buf_list, nvim_ref => move |args| {
            let mut nvim = nvim_ref.nvim().unwrap();
            let mut args = args.into_iter();
            if let Some(buf_num) = args.next() {
                if let Ok(num) = buf_num.parse::<u32>() {
                        let is_modified = if let Ok(neovim_lib::Value::Boolean(is_modified)) = neovim_lib::neovim_api::Buffer::new(neovim_lib::Value::Integer(neovim_lib::Integer::from(num))).get_option(&mut nvim, "modified") {is_modified} else {false};
                        update_buf_list_modified(&buf_list, num, is_modified);
                }
            }
        }));

        let buf_list = &self.comps.buf_list;
        let nvim_ref = self.nvim.as_ref().unwrap();
        shell_state.subscribe(SubscriptionKey::from("DirChanged"), &[], clone!(buf_list, nvim_ref => move |_| {
            build_buf_list(&buf_list, &mut nvim_ref.nvim().unwrap());
        }));

        let buf_tree = &self.comps.buf_tree_view;
        let buf_list = &self.comps.buf_list;
        shell_state.subscribe(SubscriptionKey::from("BufEnter,BufDelete,BufAdd,BufDelete,BufFilePost"), &["bufnr('%')"], clone!(buf_tree, buf_list => move |args| {
            if let Some(buf_num) = args.into_iter().next() {
                if let Ok(num) = buf_num.parse::<u32>() {
                    update_buf_list_selected(&buf_list, &buf_tree, num);
                }
            }
        }));
    }

    fn connect_events(&self) {
        // Open file / go to dir, when user clicks on an entry.
        let store = &self.store;
        let state_ref = &self.state;
        let nvim_ref = self.nvim.as_ref().unwrap();
        self.tree.connect_row_activated(clone!(store, state_ref, nvim_ref => move |tree, path, _| {
            let iter = store.get_iter(path).unwrap();
            let file_type = store
                .get_value(&iter, Column::FileType as i32)
                .get::<u8>()
                .unwrap();
            let file_path = store
                .get_value(&iter, Column::Path as i32)
                .get::<String>()
                .unwrap();
            if file_type == FileType::Dir as u8 {
                let expanded = tree.row_expanded(path);
                if expanded {
                    tree.collapse_row(path);
                } else {
                    tree.expand_row(path, false);
                }
            } else {
                // FileType::File
                let cwd = &state_ref.borrow().current_dir;
                let cwd = Path::new(cwd);
                let file_path = if let Some(rel_path) = Path::new(&file_path)
                    .strip_prefix(&cwd)
                    .ok()
                    .and_then(|p| p.to_str())
                {
                    rel_path
                } else {
                    &file_path
                };
                let file_path = escape_filename(file_path);
                nvim_ref.nvim().unwrap().command_async(&format!(":e {}", file_path))
                    .cb(|r| r.report_err())
                    .call();
            }
        }));

        // Connect directory list.
        let nvim_ref = self.nvim.as_ref().unwrap();
        self.comps.dir_list.connect_changed(clone!(nvim_ref, state_ref => move |dir_list| {
            if let Some(iter) = dir_list.get_active_iter() {
                let model = dir_list.get_model().unwrap();
                if let Some(dir) = model.get_value(&iter, 2).get::<&str>() {
                    if dir != state_ref.borrow().current_dir {
                        let mut nvim = nvim_ref.nvim().unwrap();
                        nvim.set_current_dir_async(dir)
                            .cb(|r| r.report_err())
                            .call();
                    }
                }
            }
        }));

        let context_menu = &self.comps.context_menu;
        let cd_action = &self.comps.cd_action;
        self.tree.connect_button_press_event(
            clone!(store, state_ref, context_menu, cd_action => move |tree, ev_btn| {
                // Open context menu on right click.
                if ev_btn.get_button() == 3 {
                    context_menu.popup_at_pointer(Some(&**ev_btn));
                    let (pos_x, pos_y) = ev_btn.get_position();
                    let iter = tree
                        .get_path_at_pos(pos_x as i32, pos_y as i32)
                        .and_then(|(path, _, _, _)| path)
                        .and_then(|path| store.get_iter(&path));
                    let file_type = iter
                        .as_ref()
                        .and_then(|iter| {
                            store
                                .get_value(&iter, Column::FileType as i32)
                                .get::<u8>()
                        });
                    // Enable the "Go To Directory" action only if the user clicked on a folder.
                    cd_action.set_enabled(file_type == Some(FileType::Dir as u8));
                    let path = iter
                        .and_then(|iter| {
                            store
                                .get_value(&iter, Column::Path as i32)
                                .get::<String>()
                        });
                    state_ref.borrow_mut().selected_path = path;
                }
                Inhibit(false)
            }),
        );

        // Show / hide hidden files when corresponding menu item is toggled.
        self.comps.show_hidden_checkbox.connect_toggled(clone!(state_ref, store => move |ev| {
            let mut state = state_ref.borrow_mut();
            state.show_hidden = ev.get_active();
            tree_reload(&store, &state);
        }));
    }

    pub fn set_enable_tree_lines(&self, setting: bool) {
        self.tree.set_enable_tree_lines(setting);
    }

    pub fn get_enable_tree_lines(&self) -> bool {
        self.tree.get_enable_tree_lines()
    }

    pub fn set_show_hidden_files(&self, setting: bool) {
        self.comps.show_hidden_checkbox.set_active(setting);
        let mut state = self.state.borrow_mut();
        state.show_hidden = setting;
        tree_reload(&self.store, &state);
    }

    pub fn get_show_hidden_files(&self) -> bool {
        self.state.borrow().show_hidden
    }

    pub fn update_buf_list(&self) {
        build_buf_list(&self.comps.buf_list, &mut self.nvim.as_ref().unwrap().nvim().unwrap());
    }
}

/// Compare function for dir entries.
///
/// Sorts directories above files.
fn cmp_dirs_first(lhs: &DirEntry, rhs: &DirEntry) -> io::Result<Ordering> {
    let lhs_metadata = fs::metadata(lhs.path())?;
    let rhs_metadata = fs::metadata(rhs.path())?;
    if lhs_metadata.is_dir() == rhs_metadata.is_dir() {
        Ok(lhs.path()
            .to_string_lossy()
            .to_lowercase()
            .cmp(&rhs.path().to_string_lossy().to_lowercase()))
    } else {
        if lhs_metadata.is_dir() {
            Ok(Ordering::Less)
        } else {
            Ok(Ordering::Greater)
        }
    }
}

/// Clears an repopulate the entire tree.
fn tree_reload(store: &gtk::TreeStore, state: &State) {
    let dir = &state.current_dir;
    store.clear();
    populate_tree_nodes(store, state, dir, None);
}

/// Updates the dirctory list on top of the file browser.
///
/// The list represents the path the the current working directory.  If the new cwd is a parent of
/// the old one, the list is kept and only the active entry is updated. Otherwise, the list is
/// replaced with the new path and the last entry is marked active.
fn update_dir_list(dir: &str, dir_list_model: &gtk::TreeStore, dir_list: &gtk::ComboBox) {
    // The current working directory path.
    let complete_path = Path::new(dir);
    let mut path = PathBuf::new();
    let mut components = complete_path.components();
    let mut next = components.next();

    // Iterator over existing dir_list model.
    let mut dir_list_iter = dir_list_model.get_iter_first();

    // Whether existing entries up to the current position of dir_list_iter are a prefix of the
    // new current working directory path.
    let mut is_prefix = true;

    // Iterate over components of the cwd. Simultaneously move dir_list_iter forward.
    while let Some(dir) = next {
        next = components.next();
        let dir_name = &*dir.as_os_str().to_string_lossy();
        // Assemble path up to current component.
        path.push(Path::new(&dir));
        let path_str = path.to_str().unwrap_or_else(|| {
            error!(
                "Could not convert path to string: {}\n
                    Directory chooser will not work for that entry.",
                path.to_string_lossy()
            );
            ""
        });
        // Use the current entry of dir_list, if any, otherwise append a new one.
        let current_iter = dir_list_iter.unwrap_or_else(|| dir_list_model.append(None));
        // Check if the current entry is still part of the new cwd.
        if is_prefix && dir_list_model.get_value(&current_iter, 0).get::<&str>() != Some(&dir_name)
        {
            is_prefix = false;
        }
        if next.is_some() {
            // Update dir_list entry.
            dir_list_model.set(
                &current_iter,
                &[0, 1, 2],
                &[&dir_name, &ICON_FOLDER_CLOSED, &path_str],
            );
        } else {
            // We reached the last component of the new cwd path. Set the active entry of dir_list
            // to this one.
            dir_list_model.set(
                &current_iter,
                &[0, 1, 2],
                &[&dir_name, &ICON_FOLDER_OPEN, &path_str],
            );
            dir_list.set_active_iter(Some(&current_iter));
        };
        // Advance dir_list_iter.
        dir_list_iter = if dir_list_model.iter_next(&current_iter) {
            Some(current_iter)
        } else {
            None
        }
    }
    // We updated the dir list to the point of the current working directory.
    if let Some(iter) = dir_list_iter {
        if is_prefix {
            // If we didn't change any entries to this point and the list contains further entries,
            // the remaining ones are subdirectories of the cwd and we keep them.
            loop {
                dir_list_model.set(&iter, &[1], &[&ICON_FOLDER_CLOSED]);
                if !dir_list_model.iter_next(&iter) {
                    break;
                }
            }
        } else {
            // If we needed to change entries, the following ones are not directories under the
            // cwd and we clear them.
            while dir_list_model.remove(&iter) {}
        }
    }
}

/// Populates one level, i.e. one directory of the file browser tree.
fn populate_tree_nodes(
    store: &gtk::TreeStore,
    state: &State,
    dir: &str,
    parent: Option<&gtk::TreeIter>,
) {
    let path = Path::new(dir);
    let read_dir = match path.read_dir() {
        Ok(read_dir) => read_dir,
        Err(err) => {
            error!("Couldn't populate tree: {}", err);
            return;
        }
    };
    let iter = read_dir.filter_map(Result::ok);
    let mut entries: Vec<DirEntry> = if state.show_hidden {
        iter.collect()
    } else {
        iter.filter(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
            .filter(|entry| !entry.file_name().to_string_lossy().ends_with('~'))
            .collect()
    };
    entries.sort_unstable_by(|lhs, rhs| cmp_dirs_first(lhs, rhs).unwrap_or(Ordering::Equal));
    for entry in entries {
        let path = if let Some(path) = entry.path().to_str() {
            path.to_owned()
        } else {
            // Skip paths that contain invalid unicode.
            continue;
        };
        let filename = entry.file_name().to_str().unwrap().to_owned();
        let file_type = if let Ok(metadata) = fs::metadata(entry.path()) {
            let file_type = metadata.file_type();
            if file_type.is_dir() {
                FileType::Dir
            } else if file_type.is_file() {
                FileType::File
            } else {
                continue;
            }
        } else {
            // In case of invalid symlinks, we cannot obtain metadata.
            continue;
        };
        let icon = match file_type {
            FileType::Dir => ICON_FOLDER_CLOSED.to_string(),
            FileType::File => {
                let file_name = path.split("/").last().unwrap().trim();
                get_icon(vec![file_name, file_name.split(".").last().unwrap()])
            },
        };
        // When we get until here, we want to show the entry. Append it to the tree.
        let iter = store.append(parent);
        store.set(
            &iter,
            &[0, 1, 2, 3],
            &[&filename, &path, &(file_type as u8), &icon],
        );
        // For directories, check whether the directory is empty. If not, append a single empty
        // entry, so the expand arrow is shown. Its contents are dynamically populated when
        // expanded (see `init`).
        if let FileType::Dir = file_type {
            let not_empty = if let Ok(mut dir) = entry.path().read_dir() {
                dir.next().is_some()
            } else {
                false
            };
            if not_empty {
                let iter = store.append(Some(&iter));
                store.set(&iter, &[], &[]);
            }
        }
    }
}

fn get_current_dir(nvim: &mut NeovimRef) -> Option<String> {
    match nvim.eval("getcwd()") {
        Ok(cwd) => cwd.as_str().map(|s| s.to_owned()),
        Err(err) => {
            error!("Couldn't get cwd: {}", err);
            None
        }
    }
}

/// Reveals and selects the given file in the file browser.
///
/// Returns `true` if the file could be successfully revealed.
fn reveal_path_in_tree(store: &gtk::TreeStore, tree: &gtk::TreeView, rel_file_path: &Path) -> bool {
    let mut tree_path = gtk::TreePath::new();
    'components: for component in rel_file_path.components() {
        if let Component::Normal(component) = component {
            tree_path.down();
            while let Some(iter) = store.get_iter(&tree_path) {
                let entry_value = store.get_value(&iter, Column::Filename as i32);
                let entry = entry_value.get::<&str>().unwrap();
                if component == entry {
                    tree.expand_row(&tree_path, false);
                    continue 'components;
                }
                tree_path.next();
            }
            return false;
        } else {
            return false;
        }
    }
    if tree_path.get_depth() == 0 {
        return false;
    }
    tree.set_cursor(&tree_path, Option::<&gtk::TreeViewColumn>::None, false);
    true
}

fn build_buf_list(buf_list: &gtk::TreeStore, nvim: &mut NeovimRef) {
    let buffers = nvim.list_bufs().unwrap();
    buf_list.clear();
    for buf in buffers {
        let buf_id = buf.get_number(nvim).unwrap();
        let name = nvim.eval(&format!("buffer_name({})", buf_id)).unwrap_or(neovim_lib::Value::from(""));
        let name = name.as_str().unwrap_or("");
        if let Ok(neovim_lib::Value::Boolean(is_listed)) = buf.get_option(nvim, "buflisted") {
            if ! is_listed {
                continue;
            }
        }
        let iter = buf_list.append(None);
        let file_name: String;
        let icon: String;
        if name == "" {
            file_name = String::from("[No Name]");
            icon = get_icon(vec![]);
        } else {
            file_name = name.split("/").last().unwrap().to_string();
            icon = get_icon(vec![&name[..], name.split(".").last().unwrap()]);
        }
        let is_modified = if let Ok(neovim_lib::Value::Boolean(is_modified)) = buf.get_option(nvim, "modified") {is_modified} else {false};

        let close_icon_name = if is_modified {"edit-delete-symbolic"} else {"window-close-symbolic"};

        let mut path: Vec<&str> = name.split("/").collect();
        let _e = path.pop();
        let path = path.join("/");
        let formatted_name = format!("{} <small>{}</small>", &encode_minimal(&file_name), substitute_home_for_tilde(&encode_minimal(&path)));

        buf_list.set(
            &iter,
            &[0, 1, 2, 3],
            &[&icon, &buf_id, &formatted_name, &close_icon_name],
        );
    }
}

fn update_buf_list_modified(buf_list: &gtk::TreeStore, id: u32, is_modified: bool) {
    let mut tree_path = gtk::TreePath::new();
    tree_path.down();
    while let Some(iter) = buf_list.get_iter(&tree_path) {
        if id == buf_list.get_value(&iter, 1).get::<u32>().unwrap() {
            let new_icon = gtk::Value::from(if is_modified {"edit-delete-symbolic"} else {"window-close-symbolic"});
            buf_list.set_value(&iter, 3, &new_icon);
            break;
        }
        tree_path.next();
    }
}

fn update_buf_list_selected(buf_list: &gtk::TreeStore, buf_tree: &gtk::TreeView, id: u32) {
    let mut tree_path = gtk::TreePath::new();
    tree_path.down();
    while let Some(iter) = buf_list.get_iter(&tree_path) {
        if let Some(model_num) = buf_list.get_value(&iter, 1).get::<u32>() {
            if id == model_num {
                buf_tree.set_cursor(&tree_path, Option::<&gtk::TreeViewColumn>::None, false);
                break;
            }
        }
        tree_path.next();
    }
}

fn update_buf_list_added(buf_list: &gtk::TreeStore, id: u32, name: &str) {
    let mut tree_path = gtk::TreePath::new();
    let mut tree_iter: Option<gtk::TreeIter> = None;
    tree_path.down();
    while let Some(iter) = buf_list.get_iter(&tree_path) { 
        if buf_list.get_value(&iter, 1).get::<u32>().unwrap() > id {
            tree_path.prev();
            break;
        }
        tree_iter = Some(iter);
        tree_path.next();
    }

    let iter = buf_list.insert_after(None, tree_iter.as_ref());
    let file_name = name.split("/").last().unwrap_or(name);
    let mut path: Vec<&str> = name.split("/").collect();
    let _e = path.pop();
    let path = path.join("/");
    let formatted_name = format!("{} <small>{}</small>", &encode_minimal(&file_name), substitute_home_for_tilde(&encode_minimal(&path)));
    let icon = get_icon(vec![file_name, file_name.split(".").last().unwrap_or("")]);
    buf_list.set(
        &iter,
        &[0, 1, 2, 3],
        &[&icon, &id, &if name == "" { "[No Name]" } else { &formatted_name }, &"window-close-symbolic"]
    );
}

fn update_buf_list_deleted(buf_list: &gtk::TreeStore, id: u32) {
    let mut tree_path = gtk::TreePath::new();
    tree_path.down();
    while let Some(iter) = buf_list.get_iter(&tree_path) {
        if id == buf_list.get_value(&iter, 1).get::<u32>().unwrap() {
            buf_list.remove(&iter);
            break;
        }
        tree_path.next();
    }
}

fn update_buf_list_rename(buf_list: &gtk::TreeStore, id: u32, name: &str) {
    let mut tree_path = gtk::TreePath::new();
    tree_path.down();
    while let Some(iter) = buf_list.get_iter(&tree_path) {
        if id == buf_list.get_value(&iter, 1).get::<u32>().unwrap() {
            buf_list.set_value(&iter, 2, &gtk::Value::from(name));
            break;
        }
        tree_path.next();
    }
}

pub fn get_icon(names: Vec<&str>) -> String {
    let icon_theme = gtk::IconTheme::get_default().unwrap();
    let lookup_flags = gtk::IconLookupFlags::empty();
    for name in names {
        let name = format!("{}-symbolic", name);
        if let Some(_) = icon_theme.lookup_icon(&name, 16, lookup_flags) {
            return name;
        }
    }
    return ICON_FILE.to_string();
}
