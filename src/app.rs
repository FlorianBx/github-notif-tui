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
    pub failed_details: std::collections::HashSet<PrId>,
    pub loading: bool,
    pub loading_detail: bool,
}

impl TabState {
    pub fn visible_prs<'a>(&'a self, query: &str) -> Vec<&'a PullRequest> {
        if query.is_empty() {
            self.prs.iter().collect()
        } else {
            let q = query.to_lowercase();
            self.prs
                .iter()
                .filter(|pr| pr.title.to_lowercase().contains(&q))
                .collect()
        }
    }

    pub fn selected_pr<'a>(&'a self, query: &str) -> Option<&'a PullRequest> {
        self.visible_prs(query).into_iter().nth(self.selected)
    }

    pub fn move_up(&mut self, query: &str) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        let _ = query;
    }

    pub fn move_down(&mut self, query: &str) {
        let count = self.visible_prs(query).len();
        if count > 0 && self.selected < count - 1 {
            self.selected += 1;
        }
    }

    pub fn go_to_first(&mut self) {
        self.selected = 0;
    }

    pub fn go_to_last(&mut self, query: &str) {
        let count = self.visible_prs(query).len();
        if count > 0 {
            self.selected = count - 1;
        }
    }
}

#[derive(Debug)]
pub struct AppState {
    pub tabs: [TabState; 4],
    pub active_tab: usize,
    pub last_refresh: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub search_mode: bool,
    pub search_query: String,
    pub pending_g: bool,
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
            search_mode: false,
            search_query: String::new(),
            pending_g: false,
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
        self.reset_search();
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = (self.active_tab + 3) % 4;
        self.reset_search();
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

    pub fn reset_search(&mut self) {
        self.search_mode = false;
        self.search_query.clear();
        self.active_tab_state_mut().selected = 0;
    }
}
