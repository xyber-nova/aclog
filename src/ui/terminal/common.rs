//! 终端 UI 共享布局与交互辅助。
//!
//! 这一层只放跨页面复用的“展示骨架”和“输入语义”：
//! - 页面三段式布局
//! - 面板封装
//! - 帮助区切换
//! - 通用列表选中逻辑
//!
//! 页面特有的文案、表格列和业务判断不应继续下沉到这里。

use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::theme;

/// 所有工作台页面统一使用的三段式纵向布局。
///
/// 返回顺序固定为：`[header, content, footer]`。
pub(crate) fn root_vertical_layout(area: Rect, header_height: u16) -> [Rect; 3] {
    // 所有工作台页面都复用同一套阅读结构：
    // 上方上下文、中间主体内容、下方紧凑的操作/帮助区。
    let sections = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Min(8),
        Constraint::Length(3),
    ])
    .split(area);
    [sections[0], sections[1], sections[2]]
}

/// 把主体内容切成“主列表/表格 + 右侧摘要”两栏。
///
/// 百分比由页面自己控制，这样既保持统一阅读结构，
/// 也允许不同页面按信息密度做细调。
pub(crate) fn split_main_with_summary(
    area: Rect,
    left_percent: u16,
    right_percent: u16,
) -> [Rect; 2] {
    let parts = Layout::horizontal([
        Constraint::Percentage(left_percent),
        Constraint::Percentage(right_percent),
    ])
    .split(area);
    [parts[0], parts[1]]
}

/// 统一面板外观。
pub(crate) fn panel(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style())
        .title(title.into())
        .title_style(theme::title_style())
}

/// 渲染多行文本面板。
pub(crate) fn lines_panel(
    title: impl Into<String>,
    lines: Vec<Line<'static>>,
) -> Paragraph<'static> {
    Paragraph::new(lines)
        .block(panel(title))
        .wrap(Wrap { trim: false })
}

/// 渲染普通字符串面板。
pub(crate) fn text_panel(title: impl Into<String>, text: impl Into<String>) -> Paragraph<'static> {
    Paragraph::new(text.into())
        .block(panel(title))
        .wrap(Wrap { trim: false })
}

/// 统一 footer / 帮助区样式。
///
/// `core` 负责默认态下的高频键位摘要，
/// `help_lines` 则是展开帮助时展示的完整动作说明。
pub(crate) fn footer_panel(
    core: &str,
    help_lines: &[&str],
    help_visible: bool,
) -> Paragraph<'static> {
    // 默认 footer 只展示高频操作，保证扫读成本低；
    // `?` 展开后再补充完整说明，但不改变当前工作流状态。
    if help_visible {
        let mut lines = vec![
            Line::from("核心操作"),
            Line::from(core.to_string()),
            Line::from(""),
        ];
        lines.push(Line::from("完整帮助"));
        lines.extend(
            help_lines
                .iter()
                .map(|line| Line::from((*line).to_string())),
        );
        lines_panel("帮助", lines)
    } else {
        text_panel("操作", core)
    }
}

/// 根据集合大小决定初始选中项。
pub(crate) fn initial_selection_for_count(count: usize) -> Option<usize> {
    (count > 0).then_some(0)
}

/// 把当前选中下标收敛到合法区间。
pub(crate) fn clamp_selection(selected: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        selected.min(count - 1)
    }
}

/// 统一的列表移动语义。
///
/// 这里只负责把按键翻译成“新的选中项”，
/// 不负责决定某个页面是否真的允许移动。
pub(crate) fn move_selection(
    key: crossterm::event::KeyCode,
    selected: Option<usize>,
    count: usize,
) -> Option<Option<usize>> {
    match key {
        crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
            let next = selected.unwrap_or(0).saturating_sub(1);
            Some(initial_selection_for_count(count).map(|_| next))
        }
        crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
            let Some(current) = selected else {
                return None;
            };
            Some(Some((current + 1).min(count.saturating_sub(1))))
        }
        _ => None,
    }
}

/// 判断是否是帮助面板切换键。
pub(crate) fn is_help(key: crossterm::event::KeyCode) -> bool {
    matches!(key, crossterm::event::KeyCode::Char('?'))
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use super::{
        clamp_selection, footer_panel, initial_selection_for_count, is_help, move_selection,
    };

    #[test]
    fn initial_selection_and_clamp_follow_collection_size() {
        assert_eq!(initial_selection_for_count(0), None);
        assert_eq!(initial_selection_for_count(2), Some(0));
        assert_eq!(clamp_selection(10, 0), 0);
        assert_eq!(clamp_selection(10, 2), 1);
    }

    #[test]
    fn move_selection_supports_arrow_keys_and_jk() {
        assert_eq!(move_selection(KeyCode::Down, Some(0), 3), Some(Some(1)));
        assert_eq!(
            move_selection(KeyCode::Char('j'), Some(0), 3),
            Some(Some(1))
        );
        assert_eq!(move_selection(KeyCode::Up, Some(1), 3), Some(Some(0)));
        assert_eq!(
            move_selection(KeyCode::Char('k'), Some(1), 3),
            Some(Some(0))
        );
    }

    #[test]
    fn help_footer_switches_between_core_and_full_help() {
        let collapsed = format!("{:?}", footer_panel("Enter 确认", &["Esc 返回"], false));
        let expanded = format!("{:?}", footer_panel("Enter 确认", &["Esc 返回"], true));

        assert!(collapsed.contains("Enter 确认"));
        assert!(expanded.contains("完整帮助"));
        assert!(expanded.contains("Esc 返回"));
        assert!(is_help(KeyCode::Char('?')));
    }
}
