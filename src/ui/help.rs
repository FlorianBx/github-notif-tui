use crate::ui::theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

const KEYBINDINGS: &[(&str, &[(&str, &str)])] = &[
    (
        "Navigation",
        &[
            ("j / ↓", "Move down"),
            ("k / ↑", "Move up"),
            ("gg", "Jump to first"),
            ("G", "Jump to last"),
            ("h / Shift+Tab", "Previous tab"),
            ("l / Tab", "Next tab"),
        ],
    ),
    (
        "Actions",
        &[
            ("Enter / o", "Open in browser"),
            ("r", "Refresh all tabs"),
            ("q / Ctrl+C", "Quit"),
        ],
    ),
    (
        "Search & Sort",
        &[
            ("/", "Search (text, @user, repo:name)"),
            ("s", "Cycle sort (age/size/reviews/priority)"),
            ("S", "Toggle sort direction"),
        ],
    ),
    (
        "Selection",
        &[
            ("v", "Toggle select + move down"),
            ("V", "Select / deselect all"),
            ("Esc", "Clear selection"),
            ("o", "Open all selected (max 10)"),
        ],
    ),
];

pub struct HelpOverlay;

impl Widget for HelpOverlay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup = centered_rect(area, 56, 24);
        Clear.render(popup, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Keybindings — ? to close ")
            .border_style(theme::header());

        let inner = block.inner(popup);
        block.render(popup, buf);

        let mut lines: Vec<Line> = Vec::new();

        for (i, (section, bindings)) in KEYBINDINGS.iter().enumerate() {
            if i > 0 {
                lines.push(Line::raw(""));
            }
            lines.push(Line::from(Span::styled(
                format!("  {}", section),
                theme::header(),
            )));
            for (key, desc) in *bindings {
                lines.push(Line::from(vec![
                    Span::styled(format!("    {:<20}", key), theme::tab_active()),
                    Span::raw(*desc),
                ]));
            }
        }

        Paragraph::new(lines).render(inner, buf);
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width.saturating_sub(4));
    let h = height.min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}
