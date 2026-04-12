use crate::gh::{PrDetails, PrId, PullRequest};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum FilterPreset {
    #[default]
    All,
    Ready,
    NeedsReview,
    NeedsWork,
    Draft,
}

impl FilterPreset {
    pub fn label(&self) -> &'static str {
        match self {
            FilterPreset::All => "All",
            FilterPreset::Ready => "Ready",
            FilterPreset::NeedsReview => "Review",
            FilterPreset::NeedsWork => "Attention",
            FilterPreset::Draft => "Draft",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            FilterPreset::All => FilterPreset::Ready,
            FilterPreset::Ready => FilterPreset::NeedsReview,
            FilterPreset::NeedsReview => FilterPreset::NeedsWork,
            FilterPreset::NeedsWork => FilterPreset::Draft,
            FilterPreset::Draft => FilterPreset::All,
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum PrStatus {
    Ready,
    #[default]
    InProgress,
    NeedsWork,
    Draft,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub enum SortKey {
    #[default]
    Default,
    Age,
    Size,
    Reviews,
    Priority,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub enum SortDir {
    #[default]
    Asc,
    Desc,
}

#[derive(Default, Clone, Debug)]
pub struct SortState {
    pub key: SortKey,
    pub dir: SortDir,
}

impl SortKey {
    pub fn label(&self) -> &'static str {
        match self {
            SortKey::Default => "default",
            SortKey::Age => "age",
            SortKey::Size => "size",
            SortKey::Reviews => "reviews",
            SortKey::Priority => "priority",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            SortKey::Default => SortKey::Age,
            SortKey::Age => SortKey::Size,
            SortKey::Size => SortKey::Reviews,
            SortKey::Reviews => SortKey::Priority,
            SortKey::Priority => SortKey::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Personal = 0,
    Team = 1,
    Mentioned = 2,
    Assigned = 3,
    Mine = 4,
}

impl Tab {
    pub fn label(&self) -> &'static str {
        match self {
            Tab::Personal => "Personal",
            Tab::Team => "Team",
            Tab::Mentioned => "Mentioned",
            Tab::Assigned => "Assigned",
            Tab::Mine => "Mine",
        }
    }
}

impl From<usize> for Tab {
    fn from(i: usize) -> Self {
        match i % 5 {
            0 => Tab::Personal,
            1 => Tab::Team,
            2 => Tab::Mentioned,
            3 => Tab::Assigned,
            _ => Tab::Mine,
        }
    }
}

#[derive(Debug, Default)]
pub struct TabState {
    pub prs: Vec<PullRequest>,
    pub selected: usize,
    pub details_cache: HashMap<PrId, PrDetails>,
    pub failed_details: HashSet<PrId>,
    pub loading: bool,
    pub loading_detail: bool,
    pub selected_set: HashSet<usize>,
}

fn matches_query(pr: &PullRequest, query: &str) -> bool {
    let q = query.to_lowercase();
    if let Some(user) = q.strip_prefix('@') {
        return pr.author.login.to_lowercase().contains(user);
    }
    if let Some(repo) = q.strip_prefix("repo:") {
        return pr.repository.name_with_owner.to_lowercase().contains(repo);
    }
    pr.title.to_lowercase().contains(&q)
        || pr.author.login.to_lowercase().contains(&q)
        || pr.repository.name_with_owner.to_lowercase().contains(&q)
}

impl TabState {
    pub fn pr_status(&self, pr: &PullRequest) -> PrStatus {
        if pr.is_draft {
            return PrStatus::Draft;
        }
        let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
        let Some(d) = self.details_cache.get(&pr_id) else {
            return PrStatus::InProgress;
        };
        let ci_fail = d.ci_status == crate::gh::CiStatus::Fail;
        let changes = crate::review::has_active_changes(
            &crate::review::analyze_reviewers(d, &pr.author.login),
        );
        if ci_fail || changes {
            return PrStatus::NeedsWork;
        }
        let approved = d.review_decision.as_deref() == Some("APPROVED")
            && d.ci_status == crate::gh::CiStatus::Pass;
        if approved {
            PrStatus::Ready
        } else {
            PrStatus::InProgress
        }
    }

    pub fn visible_prs<'a>(
        &'a self,
        query: &str,
        sort: &SortState,
        filter: FilterPreset,
    ) -> Vec<&'a PullRequest> {
        let filtered: Vec<&'a PullRequest> = self
            .prs
            .iter()
            .filter(|pr| query.is_empty() || matches_query(pr, query))
            .filter(|pr| match filter {
                FilterPreset::All => true,
                FilterPreset::Ready => self.pr_status(pr) == PrStatus::Ready,
                FilterPreset::NeedsReview => self.pr_status(pr) == PrStatus::InProgress,
                FilterPreset::NeedsWork => self.pr_status(pr) == PrStatus::NeedsWork,
                FilterPreset::Draft => self.pr_status(pr) == PrStatus::Draft,
            })
            .collect();

        if sort.key == SortKey::Default {
            return filtered;
        }

        let mut sorted = filtered;
        sorted.sort_by_key(|pr| {
            let pr_id = (pr.repository.name_with_owner.clone(), pr.number);
            match sort.key {
                SortKey::Default => 0i64,
                SortKey::Age => pr.created_at.timestamp(),
                SortKey::Size => {
                    self.details_cache
                        .get(&pr_id)
                        .map(|d| (d.additions + d.deletions) as i64)
                        .unwrap_or(0)
                }
                SortKey::Reviews => {
                    self.details_cache
                        .get(&pr_id)
                        .map(|d| d.reviews.len() as i64)
                        .unwrap_or(0)
                }
                SortKey::Priority => {
                    let details = self.details_cache.get(&pr_id);
                    crate::score::compute_priority(pr, details) as i64
                }
            }
        });

        if sort.dir == SortDir::Desc {
            sorted.reverse();
        }

        sorted
    }

    pub fn selected_pr<'a>(
        &'a self,
        query: &str,
        sort: &SortState,
        filter: FilterPreset,
    ) -> Option<&'a PullRequest> {
        self.visible_prs(query, sort, filter)
            .into_iter()
            .nth(self.selected)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self, query: &str, sort: &SortState, filter: FilterPreset) {
        let count = self.visible_prs(query, sort, filter).len();
        if count > 0 && self.selected < count - 1 {
            self.selected += 1;
        }
    }

    pub fn go_to_first(&mut self) {
        self.selected = 0;
    }

    pub fn go_to_last(&mut self, query: &str, sort: &SortState, filter: FilterPreset) {
        let count = self.visible_prs(query, sort, filter).len();
        if count > 0 {
            self.selected = count - 1;
        }
    }

    pub fn toggle_selection(&mut self, idx: usize) {
        if !self.selected_set.remove(&idx) {
            self.selected_set.insert(idx);
        }
    }

    pub fn select_all_visible(&mut self, count: usize) {
        if self.selected_set.len() == count {
            self.selected_set.clear();
        } else {
            self.selected_set = (0..count).collect();
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_set.clear();
    }

    pub fn has_selection(&self) -> bool {
        !self.selected_set.is_empty()
    }
}

#[derive(Debug)]
pub struct AppState {
    pub tabs: [TabState; 5],
    pub active_tab: usize,
    pub last_refresh: Option<DateTime<Utc>>,
    pub error: Option<String>,
    pub search_mode: bool,
    pub search_query: String,
    pub pending_g: bool,
    pub sort: SortState,
    pub filter: FilterPreset,
    pub show_help: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            tabs: [
                TabState { loading: true, ..Default::default() },
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
            sort: SortState::default(),
            filter: FilterPreset::All,
            show_help: false,
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
        self.active_tab = (self.active_tab + 1) % 5;
        self.reset_search();
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = (self.active_tab + 4) % 5;
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
        self.active_tab_state_mut().clear_selection();
    }
}
