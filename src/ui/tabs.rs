use ratatui::prelude::*;
use ratatui::widgets::Tabs as RatatuiTabs;

pub fn render_tabs(frame: &mut Frame, area: Rect, tab_labels: &[&str], active: usize) {
    let tabs = RatatuiTabs::new(tab_labels.to_vec())
        .select(active)
        .style(Style::default().dim())
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan).bold());
    frame.render_widget(tabs, area);
}
