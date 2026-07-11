use std::collections::HashSet;

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

/// Display data for one task in the unified Scripts list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListItem {
    pub key: String,
    pub scope_label: String,
    pub name: String,
    pub command: String,
}

pub fn render_script_list(
    frame: &mut Frame,
    area: Rect,
    scripts: &[TaskListItem],
    filtered_indices: &[usize],
    selected_index: usize,
    scroll_offset: usize,
    favorites: &HashSet<String>,
) {
    let visible_height = area.height as usize;

    let lines: Vec<_> = filtered_indices
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(display_i, &script_i)| {
            let script = &scripts[script_i];
            task_line(
                script,
                display_i == selected_index,
                favorites.contains(&script.key),
                area.width as usize,
            )
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}

fn task_line(
    task: &TaskListItem,
    is_selected: bool,
    is_favorite: bool,
    row_width: usize,
) -> Line<'static> {
    let selected_background = is_selected.then_some(Color::DarkGray);
    let base_style = selected_background
        .map(|background| Style::default().bg(background))
        .unwrap_or_default();
    let cursor_style = if is_selected {
        base_style.fg(Color::Cyan)
    } else {
        base_style
    };
    let favorite_style = base_style.fg(Color::Yellow);
    let scope_style = base_style.fg(Color::Cyan);
    let script_style = if is_selected {
        base_style.bold()
    } else {
        base_style
    };
    let command_style = if is_selected {
        base_style.fg(Color::Gray)
    } else {
        base_style.fg(Color::DarkGray)
    };

    let mut spans = vec![
        Span::styled(if is_selected { "▎" } else { " " }, cursor_style),
        Span::styled(if is_favorite { "★ " } else { "  " }, favorite_style),
    ];
    let available_width = row_width.saturating_sub(3);
    if available_width == 0 {
        return Line::from(spans);
    }

    let scope_width = display_width(&task.scope_label);
    let script_width = display_width(&task.name);
    let identity_width = scope_width + script_width + 3; // brackets plus separator

    if identity_width <= available_width {
        spans.push(Span::styled(
            format!("[{}] ", task.scope_label),
            scope_style,
        ));
        spans.push(Span::styled(task.name.clone(), script_style));

        let command_width = available_width.saturating_sub(identity_width);
        if !task.command.is_empty() && command_width >= 4 {
            spans.push(Span::styled("  ", base_style));
            spans.push(Span::styled(
                truncate_display(&task.command, command_width - 2),
                command_style,
            ));
        }
    } else {
        let (scope, script, bracketed) =
            compact_identity(&task.scope_label, &task.name, available_width);
        if bracketed {
            spans.push(Span::styled(format!("[{scope}] "), scope_style));
        } else if !scope.is_empty() {
            spans.push(Span::styled(format!("{scope} "), scope_style));
        }
        spans.push(Span::styled(script, script_style));
    }

    Line::from(spans)
}

fn compact_identity(scope: &str, script: &str, available_width: usize) -> (String, String, bool) {
    // `[s] n` is the smallest bracketed form that still distinguishes both fields.
    if available_width >= 5 {
        let (scope_width, script_width) = allocate_field_widths(
            display_width(scope),
            display_width(script),
            available_width - 3,
        );
        return (
            truncate_display(scope, scope_width),
            truncate_display(script, script_width),
            true,
        );
    }

    // Drop decoration before dropping either identity field on very narrow rows.
    if available_width >= 3 {
        let (scope_width, script_width) = allocate_field_widths(
            display_width(scope),
            display_width(script),
            available_width - 1,
        );
        return (
            truncate_display(scope, scope_width),
            truncate_display(script, script_width),
            false,
        );
    }

    (
        String::new(),
        truncate_display(script, available_width),
        false,
    )
}

fn allocate_field_widths(scope_width: usize, script_width: usize, budget: usize) -> (usize, usize) {
    debug_assert!(budget >= 2);

    let mut scope_budget = scope_width.min(budget / 2).max(1);
    let mut script_budget = script_width.min(budget - scope_budget).max(1);

    let remaining = budget.saturating_sub(scope_budget + script_budget);
    let script_extra = remaining.min(script_width.saturating_sub(script_budget));
    script_budget += script_extra;
    scope_budget += (remaining - script_extra).min(scope_width.saturating_sub(scope_budget));

    (scope_budget, script_budget)
}

fn truncate_display(value: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(value) <= max_width {
        return value.to_string();
    }

    let content_width = max_width - 1;
    let line = Line::raw(value);
    let mut result = String::new();
    let mut used_width = 0;
    for grapheme in line.styled_graphemes(Style::default()) {
        let grapheme_width = display_width(grapheme.symbol);
        if used_width + grapheme_width > content_width {
            break;
        }
        result.push_str(grapheme.symbol);
        used_width += grapheme_width;
    }
    if result.is_empty() {
        return line
            .styled_graphemes(Style::default())
            .next()
            .filter(|first| display_width(first.symbol) <= max_width)
            .map_or_else(|| "…".to_string(), |first| first.symbol.to_string());
    }
    result.push('…');
    result
}

fn display_width(value: &str) -> usize {
    Span::raw(value).width()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn render_task(width: u16, task: TaskListItem) -> String {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                render_script_list(
                    frame,
                    Rect::new(0, 0, width, 1),
                    &[task],
                    &[0],
                    0,
                    0,
                    &HashSet::new(),
                );
            })
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    fn task(scope: &str, name: &str, command: &str) -> TaskListItem {
        TaskListItem {
            key: "task".to_string(),
            scope_label: scope.to_string(),
            name: name.to_string(),
            command: command.to_string(),
        }
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn command_truncates_before_full_identity() {
        let rendered = render_task(28, task("@acme/app", "test", "vitest --coverage --watch"));

        assert!(rendered.contains("[@acme/app] test"));
        assert!(rendered.contains("vitest"));
        assert!(!rendered.contains("--coverage --watch"));
    }

    #[test]
    fn long_scope_and_script_both_remain_identifiable_on_narrow_row() {
        let rendered = render_task(
            20,
            task(
                "@very-long-company/application",
                "integration:watch:coverage",
                "vitest --coverage",
            ),
        );

        assert!(rendered.contains("[@very-"), "rendered row: {rendered:?}");
        assert!(rendered.contains("] integr"), "rendered row: {rendered:?}");
        assert_eq!(rendered.matches('…').count(), 2);
        assert!(!rendered.contains("vitest"));
    }

    #[test]
    fn unicode_identity_is_truncated_at_grapheme_and_display_width_boundaries() {
        let line = task_line(
            &task("패키지-아주긴이름", "테스트:감시모드", "vitest --watch"),
            true,
            false,
            22,
        );
        let rendered = line_text(&line);

        assert!(rendered.contains("[패키"), "rendered row: {rendered:?}");
        assert!(rendered.contains("] 테스"), "rendered row: {rendered:?}");
        assert_eq!(rendered.matches('…').count(), 2);
        assert!(!rendered.contains("vitest"));
        assert!(line.width() <= 22);
    }

    #[test]
    fn decoration_yields_before_identity_on_extremely_narrow_row() {
        let rendered = render_task(7, task("root", "test", "echo test"));

        assert!(!rendered.contains('['));
        assert!(rendered.contains("r t…"), "rendered row: {rendered:?}");
    }

    #[test]
    fn truncation_preserves_combining_graphemes_and_wide_identity_glyphs() {
        assert_eq!(truncate_display("e\u{301}clair", 2), "e\u{301}…");
        assert_eq!(truncate_display("패키지", 2), "패");
    }
}
