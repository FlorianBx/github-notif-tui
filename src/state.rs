use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LocalState {
    #[serde(default)]
    pub done: HashSet<(String, u64)>,
    #[serde(default)]
    pub snoozed: HashMap<(String, u64), DateTime<Utc>>,
    #[serde(default)]
    pub pinned: HashSet<(String, u64)>,
}

fn state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".config")
        .join("github-notif-tui")
        .join("state.json")
}

pub fn load_state() -> LocalState {
    let path = state_path();
    match fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => LocalState::default(),
    }
}

pub fn save_state(state: &LocalState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string_pretty(state) {
        let _ = fs::write(&path, data);
    }
}
