//! 终端 UI 的共享视觉 token。
//!
//! 这里集中定义语义色、选中态和少量共享符号，
//! 让各页面只表达“这是成功/危险/提示/焦点”，
//! 不在页面代码里散落具体颜色常量。

use ratatui::style::{Color, Modifier, Style};

use crate::domain::sync_batch::{SyncChangeKind, SyncItemStatus};
use crate::utils::normalize_verdict;

/// 当前焦点项的前缀符号，用来和高亮背景一起强调选中态。
pub(crate) const FOCUS_SYMBOL: &str = "› ";
/// 摘要区和告警列表里的统一提示符号。
pub(crate) const WARNING_SYMBOL: &str = "! ";

// 这里故意采用保守配色，避免在不同终端主题下过亮或过暗，
// 同时仍然让颜色承担稳定的语义提示职责。
pub(crate) fn title_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

/// 通用边框样式。
pub(crate) fn border_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// 次级信息样式，用于弱化非关键上下文。
pub(crate) fn muted_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// 强调信息样式，用于标题、高频表头和当前模式标签。
pub(crate) fn accent_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

/// 成功语义样式，例如 AC、已提交、已决状态。
pub(crate) fn success_style() -> Style {
    Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD)
}

/// 危险语义样式，例如 WA、删除、显式失败状态。
pub(crate) fn danger_style() -> Style {
    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
}

/// 警告语义样式，用于需要注意但不等于失败的状态。
pub(crate) fn warning_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// 失效语义样式，用于 invalid 或明显异常的内容。
pub(crate) fn invalid_style() -> Style {
    Style::default()
        .fg(Color::LightRed)
        .add_modifier(Modifier::BOLD)
}

/// 列表选中态样式。
///
/// 这里刻意同时使用背景色、前景色和粗体，避免只靠反相导致可读性不稳定。
pub(crate) fn selected_row_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

// verdict 颜色只覆盖我们明确要强调的几类语义；
// 其他结果回退为中性文本，避免暗示并不存在的含义。
pub(crate) fn verdict_style(verdict: &str) -> Style {
    match normalize_verdict(verdict)
        .as_ref()
        .to_ascii_uppercase()
        .as_str()
    {
        "AC" => success_style(),
        "WA" => danger_style(),
        "TLE" | "RE" | "MLE" | "CE" => warning_style(),
        _ => Style::default(),
    }
}

/// sync 批次项状态的语义配色。
pub(crate) fn sync_status_style(status: SyncItemStatus) -> Style {
    match status {
        SyncItemStatus::Pending => accent_style(),
        SyncItemStatus::Planned => success_style(),
        SyncItemStatus::Skipped => muted_style(),
        SyncItemStatus::Committed => success_style(),
        SyncItemStatus::Invalid => invalid_style(),
    }
}

/// sync 变更类型的语义配色。
pub(crate) fn change_kind_style(kind: SyncChangeKind) -> Style {
    match kind {
        SyncChangeKind::Active => accent_style(),
        SyncChangeKind::Deleted => danger_style(),
    }
}

#[cfg(test)]
mod tests {
    use ratatui::style::{Color, Modifier, Style};

    use super::{
        danger_style, selected_row_style, success_style, sync_status_style, verdict_style,
        warning_style,
    };
    use crate::domain::sync_batch::SyncItemStatus;

    #[test]
    fn verdict_styles_follow_semantic_palette() {
        assert_eq!(verdict_style("AC"), success_style());
        assert_eq!(verdict_style("WA"), danger_style());
        assert_eq!(verdict_style("TLE"), warning_style());
        assert_eq!(verdict_style("unknown"), Style::default());
    }

    #[test]
    fn sync_status_styles_cover_pending_planned_and_invalid() {
        assert_eq!(
            sync_status_style(SyncItemStatus::Pending),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            sync_status_style(SyncItemStatus::Planned),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            sync_status_style(SyncItemStatus::Invalid),
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn selected_row_style_uses_background_and_bold_foreground() {
        assert_eq!(
            selected_row_style(),
            Style::default()
                .bg(Color::DarkGray)
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        );
    }
}
