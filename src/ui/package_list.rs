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

    // Calculate dynamic name column width from filtered packages
    let name_width = filtered_indices
        .iter()
        .map(|&i| packages[i].name.len())
        .max()
        .unwrap_or(20)
        .max(12)
        + 2;

    let mut lines: Vec<Line> = Vec::new();

    for (display_i, &pkg_i) in filtered_indices
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
    {
        let pkg = &packages[pkg_i];
        let is_selected = display_i == selected_index;

        let line = if is_selected {
            Line::from(vec![
                Span::styled("▎", Style::default().fg(Color::Cyan).bg(Color::DarkGray)),
                Span::styled(
                    format!("{:<width$}", &pkg.name, width = name_width),
                    Style::default().bold().bg(Color::DarkGray),
                ),
                Span::styled(
                    &pkg.relative_path,
                    Style::default().fg(Color::Gray).bg(Color::DarkGray),
                ),
            ])
        } else {
            Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!("{:<width$}", &pkg.name, width = name_width),
                    Style::default(),
                ),
                Span::styled(&pkg.relative_path, Style::default().fg(Color::DarkGray)),
            ])
        };
        lines.push(line);
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, area);
}
