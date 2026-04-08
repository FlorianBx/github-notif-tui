use crate::app::AppState;
use crate::ui::theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Tabs, Widget},
};

pub struct TabsBar<'a> {
    pub state: &'a AppState,
}

impl Widget for TabsBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let titles: Vec<Line> = (0..5)
            .map(|i| {
                let label = self.state.tab_label(i);
                if i == self.state.active_tab {
                    Line::from(Span::styled(label, theme::tab_active()))
                } else {
                    Line::from(Span::styled(label, theme::tab_inactive()))
                }
            })
            .collect();

        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::BOTTOM))
            .select(self.state.active_tab)
            .highlight_style(theme::tab_active());

        tabs.render(area, buf);
    }
}
