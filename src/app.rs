use crate::gh::{PrDetails, PrId, PullRequest};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Personal = 0,
    Team = 1,
    Mentioned = 2,
    Assigned = 3,
}

impl Tab {
    pub fn label(&self) -> &'static str {
        match self {
            Tab::Personal => "Personal",
            Tab::Team => "Team",
            Tab::Mentioned => "Mentioned",
            Tab::Assigned => "Assigned",
        }
    }
}

impl From<usize> for Tab {
    fn from(i: usize) -> Self {
        match i % 4 {
            0 => Tab::Personal,
            1 => Tab::Team,
            2 => Tab::Mentioned,
            _ => Tab::Assigned,
        }
    }
}

#[derive(Debug, Default)]
pub struct TabState {
    pub prs: Vec<PullRequest>,
    pub selected: usize,
    pub details_cache: HashMap<PrId, PrDetails>,
    pub loading: bool,
    pub loading_detail: bool,
}

impl TabState {
    pub fn selected_pr(&self) -> Option<&PullRequest> {
        self.prs.get(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.prs.is_empty() && self.selected < self.prs.len() - 1 {
            self.selected += 1;
        }
    }
}

#[derive(Debug)]
pub struct AppState {
    pub tabs: [TabState; 4],
    pub active_tab: usize,
    pub last_refresh: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tabs: [
                TabState { loading: true, ..Default::default() },
                TabState { loading: true, ..Default::default() },
                TabState { loading: true, ..Default::default() },
                TabState { loading: true, ..Default::default() },
            ],
            active_tab: 0,
            last_refresh: None,
            error: None,
        }
    }
}

impl AppState {
    pub fn active_tab_state(&self) -> &TabState {
        &self.tabs[self.active_tab]
    }

    pub fn active_tab_state_mut(&mut self) -> &mut TabState {
        &mut self.tabs[self.active_tab]
    }

    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % 4;
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = (self.active_tab + 3) % 4;
    }

    pub fn tab_label(&self, idx: usize) -> String {
        let tab = Tab::from(idx);
        let count = self.tabs[idx].prs.len();
        if count > 0 {
            format!("{} ({})", tab.label(), count)
        } else {
            tab.label().to_string()
        }
    }
}
