use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::core::workspaces::WorkspacePackage;

pub fn render_package_list(
    frame: &mut Frame,
    area: Rect,
    packages: &[WorkspacePackage],
    filtered_indices: &[usize],
    selected_index: usize,
    scroll_offset: usize,
) {
    let visible_height = area.height as usize;
    let mut lines: Vec<Line> = Vec::new();

    for (display_i, &pkg_i) in filtered_indices
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
    {
        let pkg = &packages[pkg_i];
        let is_selected = display_i == selected_index;
        let cursor = if is_selected { "❯ " } else { "  " };

        let line = Line::from(vec![
            Span::styled(
                cursor,
                if is_selected {
                    Style::default().bold()
                } else {
                    Style::default()
                },
            ),
            Span::styled(
                format!("{:<30}", &pkg.name),
                if is_selected {
                    Style::default().bold()
                } else {
                    Style::default()
                },
            ),
            Span::styled(&pkg.relative_path, Style::default().fg(Color::DarkGray)),
        ]);
        lines.push(line);
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, area);
}
