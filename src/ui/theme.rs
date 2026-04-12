use ratatui::style::{Color, Modifier, Style};

pub fn tab_active() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn tab_inactive() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn selected_row() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn normal_row() -> Style {
    Style::default()
}

pub fn unread_row() -> Style {
    Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn header() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub fn dim() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn age_old() -> Style {
    Style::default().fg(Color::Red)
}

pub fn age_medium() -> Style {
    Style::default().fg(Color::Yellow)
}

pub fn age_fresh() -> Style {
    Style::default().fg(Color::Green)
}

pub fn additions() -> Style {
    Style::default().fg(Color::Green)
}

pub fn deletions() -> Style {
    Style::default().fg(Color::Red)
}

pub fn error() -> Style {
    Style::default()
        .fg(Color::White)
        .bg(Color::Red)
        .add_modifier(Modifier::BOLD)
}

pub fn ci_pass() -> Style {
    Style::default().fg(Color::Green)
}

pub fn ci_fail() -> Style {
    Style::default().fg(Color::Red)
}

pub fn ci_pending() -> Style {
    Style::default().fg(Color::Yellow)
}
