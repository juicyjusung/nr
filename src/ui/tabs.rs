use ratatui::prelude::*;
use ratatui::widgets::Tabs as RatatuiTabs;

pub fn render_tabs(frame: &mut Frame, area: Rect, tab_labels: &[&str], active: usize) {
    let tabs = RatatuiTabs::new(tab_labels.to_vec())
        .select(active)
        .highlight_style(Style::default().bold().underlined());
    frame.render_widget(tabs, area);
}
