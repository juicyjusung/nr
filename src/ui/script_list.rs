use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::sort::SortableScript;
use std::collections::HashSet;

pub fn render_script_list(
    frame: &mut Frame,
    area: Rect,
    scripts: &[SortableScript],
    filtered_indices: &[usize],
    selected_index: usize,
    scroll_offset: usize,
    favorites: &HashSet<String>,
) {
    let visible_height = area.height as usize;
    let mut lines: Vec<Line> = Vec::new();

    for (display_i, &script_i) in filtered_indices
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
    {
        let script = &scripts[script_i];
        let is_selected = display_i == selected_index;
        let is_favorite = favorites.contains(&script.key);

        let star = if is_favorite { "★ " } else { "  " };
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
            Span::styled(star, Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:<20}", &script.name),
                if is_selected {
                    Style::default().bold()
                } else {
                    Style::default()
                },
            ),
            Span::styled(&script.command, Style::default().fg(Color::DarkGray)),
        ]);
        lines.push(line);
    }

    let paragraph = Paragraph::new(Text::from(lines));
    frame.render_widget(paragraph, area);
}
