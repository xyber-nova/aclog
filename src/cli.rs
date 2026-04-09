use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::Result;

#[derive(Debug, Parser)]
#[command(name = "aclog", version, about = "用于 jj 工作区的 OI 训练日志工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
    },
    Sync {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, conflicts_with = "rebuild")]
        resume: bool,
        #[arg(long, conflicts_with = "resume")]
        rebuild: bool,
    },
    Stats {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        days: Option<i64>,
        #[arg(long)]
        review: bool,
        #[arg(long)]
        json: bool,
    },
    Record {
        #[command(subcommand)]
        command: RecordCommands,
    },
}

#[derive(Debug, Subcommand)]
enum RecordCommands {
    Bind {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        submission_id: Option<u64>,
    },
    Rebind {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long = "record-rev")]
        record_rev: Option<String>,
        #[arg(long)]
        submission_id: Option<u64>,
    },
    List {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long)]
        problem_id: Option<String>,
        #[arg(long)]
        file_name: Option<String>,
        #[arg(long)]
        verdict: Option<String>,
        #[arg(long)]
        difficulty: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Browse {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long, value_enum, default_value_t = BrowserViewArg::Files)]
        view: BrowserViewArg,
        #[arg(long)]
        problem_id: Option<String>,
        #[arg(long)]
        file_name: Option<String>,
        #[arg(long)]
        verdict: Option<String>,
        #[arg(long)]
        difficulty: Option<String>,
        #[arg(long)]
        tag: Option<String>,
        #[arg(long)]
        days: Option<i64>,
        #[arg(long)]
        json: bool,
    },
    Show {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long = "record-rev")]
        record_rev: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Edit {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        #[arg(long = "record-rev")]
        record_rev: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        mistakes: Option<String>,
        #[arg(long)]
        insight: Option<String>,
        #[arg(long)]
        confidence: Option<String>,
        #[arg(long = "source-kind")]
        source_kind: Option<String>,
        #[arg(long = "time-spent")]
        time_spent: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum BrowserViewArg {
    Files,
    Problems,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    run_command(cli.command).await
}

async fn run_command(command: Commands) -> Result<()> {
    match command {
        Commands::Init { workspace } => crate::app::run_init(workspace).await,
        Commands::Sync {
            workspace,
            dry_run,
            resume,
            rebuild,
        } => {
            crate::app::run_sync(
                workspace,
                crate::app::SyncOptions {
                    dry_run,
                    resume,
                    rebuild,
                },
            )
            .await
        }
        Commands::Stats {
            workspace,
            days,
            review,
            json,
        } => {
            crate::app::run_stats_with_options(
                workspace,
                crate::app::StatsOptions { days, review, json },
            )
            .await
        }
        Commands::Record { command } => run_record_command(command).await,
    }
}

async fn run_record_command(command: RecordCommands) -> Result<()> {
    match command {
        RecordCommands::Bind {
            file,
            workspace,
            submission_id,
        } => crate::app::run_record_bind(workspace, file, submission_id).await,
        RecordCommands::Rebind {
            file,
            workspace,
            record_rev,
            submission_id,
        } => crate::app::run_record_rebind(workspace, file, record_rev, submission_id).await,
        RecordCommands::List {
            workspace,
            problem_id,
            file_name,
            verdict,
            difficulty,
            tag,
            json,
        } => {
            crate::app::run_record_list(
                workspace,
                crate::app::RecordListQuery {
                    problem_id,
                    file_name,
                    verdict,
                    difficulty,
                    tag,
                    json,
                },
            )
            .await
        }
        RecordCommands::Browse {
            workspace,
            view,
            problem_id,
            file_name,
            verdict,
            difficulty,
            tag,
            days,
            json,
        } => {
            crate::app::run_record_browse(
                workspace,
                crate::app::BrowserQuery {
                    root_view: match view {
                        BrowserViewArg::Files => crate::app::BrowserRootView::Files,
                        BrowserViewArg::Problems => crate::app::BrowserRootView::Problems,
                    },
                    problem_id,
                    file_name,
                    verdict,
                    difficulty,
                    tag,
                    days,
                    timeline_file: None,
                    timeline_problem: None,
                    json,
                },
            )
            .await
        }
        RecordCommands::Show {
            file,
            workspace,
            record_rev,
            json,
        } => crate::app::run_record_show(workspace, file, record_rev, json).await,
        RecordCommands::Edit {
            file,
            workspace,
            record_rev,
            note,
            mistakes,
            insight,
            confidence,
            source_kind,
            time_spent,
        } => {
            crate::app::run_record_edit(
                workspace,
                file,
                record_rev,
                crate::app::TrainingFieldsPatch {
                    note,
                    mistakes,
                    insight,
                    confidence,
                    source_kind,
                    time_spent,
                },
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser as _;

    use super::{BrowserViewArg, Cli, Commands, RecordCommands};

    #[test]
    fn cli_parses_stats_subcommand_with_default_workspace() {
        let cli = Cli::parse_from(["aclog", "stats"]);

        match cli.command {
            Commands::Stats { workspace, .. } => assert_eq!(workspace, PathBuf::from(".")),
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_stats_review_flags() {
        let cli = Cli::parse_from(["aclog", "stats", "--days", "14", "--review", "--json"]);

        match cli.command {
            Commands::Stats {
                days, review, json, ..
            } => {
                assert_eq!(days, Some(14));
                assert!(review);
                assert!(json);
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_sync_recovery_flags() {
        let cli = Cli::parse_from(["aclog", "sync", "--resume", "--dry-run"]);

        match cli.command {
            Commands::Sync {
                dry_run,
                resume,
                rebuild,
                ..
            } => {
                assert!(dry_run);
                assert!(resume);
                assert!(!rebuild);
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_record_bind_with_submission_id() {
        let cli = Cli::parse_from([
            "aclog",
            "record",
            "bind",
            "P1001.cpp",
            "--submission-id",
            "42",
        ]);

        match cli.command {
            Commands::Record {
                command:
                    RecordCommands::Bind {
                        file,
                        workspace,
                        submission_id,
                    },
            } => {
                assert_eq!(file, PathBuf::from("P1001.cpp"));
                assert_eq!(workspace, PathBuf::from("."));
                assert_eq!(submission_id, Some(42));
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_record_rebind_with_non_interactive_flags() {
        let cli = Cli::parse_from([
            "aclog",
            "record",
            "rebind",
            "P1001.cpp",
            "--record-rev",
            "abc123",
            "--submission-id",
            "42",
        ]);

        match cli.command {
            Commands::Record {
                command:
                    RecordCommands::Rebind {
                        file,
                        workspace,
                        record_rev,
                        submission_id,
                    },
            } => {
                assert_eq!(file, PathBuf::from("P1001.cpp"));
                assert_eq!(workspace, PathBuf::from("."));
                assert_eq!(record_rev.as_deref(), Some("abc123"));
                assert_eq!(submission_id, Some(42));
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_record_browse_filters() {
        let cli = Cli::parse_from([
            "aclog", "record", "browse", "--view", "problems", "--tag", "模拟", "--days", "14",
        ]);

        match cli.command {
            Commands::Record {
                command:
                    RecordCommands::Browse {
                        view, tag, days, ..
                    },
            } => {
                assert_eq!(view, BrowserViewArg::Problems);
                assert_eq!(tag.as_deref(), Some("模拟"));
                assert_eq!(days, Some(14));
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_record_list_with_default_workspace() {
        let cli = Cli::parse_from(["aclog", "record", "list"]);

        match cli.command {
            Commands::Record {
                command: RecordCommands::List { workspace, .. },
            } => assert_eq!(workspace, PathBuf::from(".")),
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_record_show_with_record_rev_and_json() {
        let cli = Cli::parse_from([
            "aclog",
            "record",
            "show",
            "P1001.cpp",
            "--record-rev",
            "abc123",
            "--json",
        ]);

        match cli.command {
            Commands::Record {
                command:
                    RecordCommands::Show {
                        file,
                        workspace,
                        record_rev,
                        json,
                    },
            } => {
                assert_eq!(file, PathBuf::from("P1001.cpp"));
                assert_eq!(workspace, PathBuf::from("."));
                assert_eq!(record_rev.as_deref(), Some("abc123"));
                assert!(json);
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }

    #[test]
    fn cli_parses_record_edit_training_fields() {
        let cli = Cli::parse_from([
            "aclog",
            "record",
            "edit",
            "P1001.cpp",
            "--note",
            "复习一下",
            "--confidence",
            "medium",
        ]);

        match cli.command {
            Commands::Record {
                command:
                    RecordCommands::Edit {
                        file,
                        note,
                        confidence,
                        ..
                    },
            } => {
                assert_eq!(file, PathBuf::from("P1001.cpp"));
                assert_eq!(note.as_deref(), Some("复习一下"));
                assert_eq!(confidence.as_deref(), Some("medium"));
            }
            command => panic!("unexpected command: {command:?}"),
        }
    }
}
