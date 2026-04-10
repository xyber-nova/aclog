use std::io::{self, IsTerminal};

use crossterm::style::{Attribute, Stylize};

use crate::domain::sync_batch::{SyncChangeKind, SyncItemStatus, SyncSessionItem};
use crate::utils::normalize_verdict;

fn color_enabled() -> bool {
    io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn maybe_colorize(text: &str, style: impl FnOnce(&str) -> String) -> String {
    if color_enabled() {
        style(text)
    } else {
        text.to_string()
    }
}

pub(crate) fn header(text: &str) -> String {
    maybe_colorize(text, |value| {
        format!("{}", value.cyan().attribute(Attribute::Bold))
    })
}

pub(crate) fn label(text: &str) -> String {
    maybe_colorize(text, |value| {
        format!("{}", value.blue().attribute(Attribute::Bold))
    })
}

pub(crate) fn muted(text: &str) -> String {
    maybe_colorize(text, |value| format!("{}", value.dark_grey()))
}

pub(crate) fn warning(text: &str) -> String {
    maybe_colorize(text, |value| {
        format!("{}", value.yellow().attribute(Attribute::Bold))
    })
}

pub(crate) fn verdict(text: &str) -> String {
    let normalized = normalize_verdict(text);
    let display = normalized.as_ref();
    match display.to_ascii_uppercase().as_str() {
        "AC" => maybe_colorize(display, |value| {
            format!("{}", value.green().attribute(Attribute::Bold))
        }),
        "WA" => maybe_colorize(display, |value| {
            format!("{}", value.red().attribute(Attribute::Bold))
        }),
        "TLE" | "RE" | "MLE" | "CE" => warning(display),
        _ => display.to_string(),
    }
}

pub(crate) fn sync_change_kind(kind: SyncChangeKind) -> String {
    match kind {
        SyncChangeKind::Active => label("已修改"),
        SyncChangeKind::Deleted => maybe_colorize("已删除", |value| {
            format!("{}", value.red().attribute(Attribute::Bold))
        }),
    }
}

pub(crate) fn sync_status(item: &SyncSessionItem) -> String {
    let label_text = match item.status {
        SyncItemStatus::Pending => match item.kind {
            SyncChangeKind::Deleted => "等待确认删除",
            SyncChangeKind::Active if item.submissions.unwrap_or(0) == 0 => "未找到提交记录",
            SyncChangeKind::Active => "等待选择提交记录",
        },
        SyncItemStatus::Planned => "已决待提交",
        SyncItemStatus::Skipped => "已跳过",
        SyncItemStatus::Committed => "已提交",
        SyncItemStatus::Invalid => "已失效",
    };
    match item.status {
        SyncItemStatus::Pending => label(label_text),
        SyncItemStatus::Planned | SyncItemStatus::Committed => {
            maybe_colorize(label_text, |value| {
                format!("{}", value.green().attribute(Attribute::Bold))
            })
        }
        SyncItemStatus::Skipped => muted(label_text),
        SyncItemStatus::Invalid => maybe_colorize(label_text, |value| {
            format!("{}", value.red().attribute(Attribute::Bold))
        }),
    }
}
