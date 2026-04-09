use std::path::PathBuf;

use color_eyre::Result;
use tracing::{info, instrument};

use crate::{
    config::AclogPaths,
    domain::stats::{StatsDashboard, build_review_candidates, summarize_solve_records_with_window},
    ui::interaction::UserInterface,
};

use super::{deps::AppDeps, support::load_record_index};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatsOptions {
    pub days: Option<i64>,
    pub review: bool,
    pub json: bool,
}

#[instrument(level = "info", skip_all, fields(workspace = %workspace.display()))]
pub async fn run(
    workspace: PathBuf,
    options: &StatsOptions,
    deps: &impl AppDeps,
    ui: &impl UserInterface,
) -> Result<()> {
    info!("开始统计");

    let paths = AclogPaths::new(workspace)?;
    deps.ensure_workspace().await?;
    let config = crate::config::load_config(&paths)?;
    let index = load_record_index(deps).await?;
    let records = index
        .all_records()
        .iter()
        .map(|entry| entry.record.clone())
        .collect::<Vec<_>>();
    let algorithm_tag_names = deps.load_algorithm_tag_names(&config, &paths).await?;
    let candidates = build_review_candidates(&records, options.days, Some(&algorithm_tag_names));
    let summary =
        summarize_solve_records_with_window(&records, options.days, Some(&algorithm_tag_names));

    if options.review {
        let output = if options.json {
            format!("{}\n", serde_json::to_string_pretty(&candidates)?)
        } else {
            let dashboard = StatsDashboard {
                summary,
                review_candidates: candidates.clone(),
                start_in_review: true,
            };
            ui.show_stats_dashboard(&paths.workspace_root, &dashboard, &index)?;
            info!(review_candidates = candidates.len(), "复习建议已输出");
            return Ok(());
        };
        deps.write_output(&output)?;
        info!(review_candidates = candidates.len(), "复习建议已输出");
        return Ok(());
    }

    if options.json {
        deps.write_output(&format!("{}\n", serde_json::to_string_pretty(&summary)?))?;
    } else {
        ui.show_stats_dashboard(
            &paths.workspace_root,
            &StatsDashboard {
                summary: summary.clone(),
                review_candidates: candidates,
                start_in_review: false,
            },
            &index,
        )?;
    }

    info!(
        total_solve_records = summary.total_solve_records,
        unique_problem_count = summary.unique_problem_count,
        "统计完成"
    );
    Ok(())
}

#[cfg(test)]
fn render_review_candidates(candidates: &[crate::domain::stats::ReviewCandidate]) -> String {
    if candidates.is_empty() {
        return "当前没有可用的复习建议\n".to_string();
    }

    let mut lines = vec!["类型\t标签\t结果\t原因".to_string()];
    for item in candidates {
        lines.push(format!(
            "{}\t{}\t{}\t{}",
            review_kind_label(&item.kind),
            item.label,
            item.verdict.as_deref().unwrap_or("-"),
            item.reason
        ));
    }
    format!("{}\n", lines.join("\n"))
}

#[cfg(test)]
fn review_kind_label(kind: &str) -> &str {
    match kind {
        "stale" => "久未复习",
        "retry" => "建议重做",
        "weakness" => "薄弱点",
        _ => kind,
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::stats::ReviewCandidate;

    use super::render_review_candidates;

    #[test]
    fn render_review_candidates_outputs_table() {
        let text = render_review_candidates(&[ReviewCandidate {
            kind: "retry".to_string(),
            label: "P1001".to_string(),
            problem_id: Some("P1001".to_string()),
            title: Some("A".to_string()),
            verdict: Some("WA".to_string()),
            last_submission_time: None,
            reason: "最近状态仍为 WA".to_string(),
        }]);

        assert!(text.contains("类型\t标签"));
        assert!(text.contains("建议重做"));
    }
}
