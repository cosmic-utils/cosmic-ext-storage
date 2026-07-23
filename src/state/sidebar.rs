use crate::models::UiDrive;
use cosmic::widget::nav_bar;
use std::collections::{HashMap, HashSet};

fn compare_drive_sort_keys(left: &UiDrive, right: &UiDrive) -> std::cmp::Ordering {
    left.device()
        .cmp(right.device())
        .then_with(|| left.name().cmp(&right.name()))
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SidebarNodeKey {
    Drive(String),
    Volume(String),
}

#[derive(Debug, Default)]
pub struct SidebarState {
    /// Latest drive models used to render the tree.
    pub drives: Vec<UiDrive>,

    /// Mapping from drive `block_path` to the corresponding `nav_bar::Id` in `app.nav`.
    pub drive_entities: HashMap<String, nav_bar::Id>,

    /// Expanded nodes in the tree.
    pub expanded: HashSet<SidebarNodeKey>,

    /// Selected (focused) child node. Drive selection is still managed via `app.nav`.
    pub selected_child: Option<SidebarNodeKey>,
    pub logical_loading: bool,
    pub drives_loading: bool,
    pub network_loading: bool,
    pub drive_builds_pending: usize,
}

impl SidebarState {
    pub fn active_drive_block_path(&self, app_nav: &nav_bar::Model) -> Option<String> {
        app_nav
            .active_data::<UiDrive>()
            .map(|d| d.device().to_string())
    }

    pub fn set_drives(&mut self, drives: Vec<UiDrive>) {
        self.drives = drives;
    }

    pub fn set_drive_entities(&mut self, entities: HashMap<String, nav_bar::Id>) {
        self.drive_entities = entities;
    }

    pub fn set_logical_loading(&mut self, loading: bool) {
        self.logical_loading = loading;
    }

    pub fn set_network_loading(&mut self, loading: bool) {
        self.network_loading = loading;
    }

    pub fn start_drive_loading(&mut self, total: usize) {
        self.drives_loading = true;
        self.drive_builds_pending = total;
        self.drives.clear();
    }

    pub fn finish_drive_loading(&mut self) {
        self.drives_loading = false;
        self.drive_builds_pending = 0;
    }

    pub fn mark_drive_build_finished(&mut self) -> bool {
        self.drive_builds_pending = self.drive_builds_pending.saturating_sub(1);
        self.drive_builds_pending == 0
    }

    pub fn upsert_drive_sorted(&mut self, drive: UiDrive) {
        if let Some(index) = self
            .drives
            .iter()
            .position(|current| current.device() == drive.device())
        {
            self.drives[index] = drive;
        } else {
            self.drives.push(drive);
        }
        self.drives.sort_by(compare_drive_sort_keys);
    }

    pub fn is_expanded(&self, key: &SidebarNodeKey) -> bool {
        self.expanded.contains(key)
    }

    pub fn toggle_expanded(&mut self, key: SidebarNodeKey) {
        if !self.expanded.insert(key.clone()) {
            self.expanded.remove(&key);
        }
    }

    pub fn find_drive(&self, device: &str) -> Option<&UiDrive> {
        self.drives.iter().find(|d| d.device() == device)
    }
}
