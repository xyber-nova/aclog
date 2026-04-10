use std::path::PathBuf;

use clap::{
    Parser, Subcommand, ValueEnum,
    builder::Styles,
    builder::styling::{AnsiColor, Effects},
};
use color_eyre::Result;

#[derive(Debug, Parser)]
#[command(
    name = "aclog",
    version,
    about = "用于 jj 工作区的 OI 训练日志工具",
    long_about = "用于在本地 jj 工作区中记录、同步、浏览和复盘算法训练过程的 CLI 工具。",
    styles = cli_styles()
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// 初始化 `.aclog/` 配置目录，并在当前目录创建或接管一个 jj 工作区
    Init {
        /// 工作区根目录
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
    },
    /// 检测当前工作区改动，并把每个题目文件整理成一条可恢复的同步批次
    Sync {
        /// 工作区根目录
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// 只输出当前批次预览，不创建 commit
        #[arg(long)]
        dry_run: bool,
        /// 恢复已有 `.aclog/sync-session.toml` 批次
        #[arg(long, conflicts_with = "rebuild")]
        resume: bool,
        /// 丢弃已有恢复状态，并按当前工作区重建批次
        #[arg(long, conflicts_with = "resume")]
        rebuild: bool,
    },
    /// 汇总训练历史，打开统计工作台或输出结构化结果
    Stats {
        /// 工作区根目录
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// 仅统计最近 N 天的记录
        #[arg(long)]
        days: Option<i64>,
        /// 直接进入 review 建议模式
        #[arg(long)]
        review: bool,
        /// 以 JSON 输出结果，而不是打开交互界面
        #[arg(long)]
        json: bool,
    },
    /// 记录维护与浏览命令
    Record {
        #[command(subcommand)]
        command: RecordCommands,
    },
}

#[derive(Debug, Subcommand)]
enum RecordCommands {
    /// 为一个已跟踪文件新建 solve 记录；缺少 `--submission-id` 时会进入选择器
    Bind {
        /// 题解文件路径
        file: PathBuf,
        /// 工作区根目录
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// 直接绑定指定提交编号，跳过选择器
        #[arg(long)]
        submission_id: Option<u64>,
    },
    /// 重写已有 solve 记录绑定的提交信息
    Rebind {
        /// 题解文件路径
        file: PathBuf,
        /// 工作区根目录
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// 指定要重写的记录 revision；缺失时进入记录选择器
        #[arg(long = "record-rev")]
        record_rev: Option<String>,
        /// 指定新的提交编号；缺失时进入提交选择器
        #[arg(long)]
        submission_id: Option<u64>,
    },
    /// 列出当前工作区仍被跟踪的已记录题解
    List {
        /// 工作区根目录
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// 按题号过滤
        #[arg(long)]
        problem_id: Option<String>,
        /// 按文件名子串过滤
        #[arg(long)]
        file_name: Option<String>,
        /// 按结果过滤，例如 `AC`、`WA`
        #[arg(long)]
        verdict: Option<String>,
        /// 按难度过滤
        #[arg(long)]
        difficulty: Option<String>,
        /// 按标签过滤
        #[arg(long)]
        tag: Option<String>,
        /// 以 JSON 输出结果
        #[arg(long)]
        json: bool,
    },
    /// 打开记录浏览工作台，或按筛选条件输出 JSON
    Browse {
        /// 工作区根目录
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// 根视角，`files` 表示文件视角，`problems` 表示题目视角
        #[arg(long, value_enum, default_value_t = BrowserViewArg::Files)]
        view: BrowserViewArg,
        /// 按题号过滤
        #[arg(long)]
        problem_id: Option<String>,
        /// 按文件名子串过滤
        #[arg(long)]
        file_name: Option<String>,
        /// 按结果过滤
        #[arg(long)]
        verdict: Option<String>,
        /// 按难度过滤
        #[arg(long)]
        difficulty: Option<String>,
        /// 按标签过滤
        #[arg(long)]
        tag: Option<String>,
        /// 仅查看最近 N 天
        #[arg(long)]
        days: Option<i64>,
        /// 以 JSON 输出当前筛选结果
        #[arg(long)]
        json: bool,
    },
    /// 查看某个文件当前记录或指定历史记录的完整详情
    Show {
        /// 题解文件路径
        file: PathBuf,
        /// 工作区根目录
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// 指定历史记录 revision
        #[arg(long = "record-rev")]
        record_rev: Option<String>,
        /// 以 JSON 输出记录详情
        #[arg(long)]
        json: bool,
    },
    /// 只改写已有 solve 记录里的训练字段，不改题解文件内容
    Edit {
        /// 题解文件路径
        file: PathBuf,
        /// 工作区根目录
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
        /// 指定要改写的历史记录 revision
        #[arg(long = "record-rev")]
        record_rev: Option<String>,
        /// 更新训练笔记
        #[arg(long)]
        note: Option<String>,
        /// 更新卡点 / mistakes
        #[arg(long)]
        mistakes: Option<String>,
        /// 更新收获 / insight
        #[arg(long)]
        insight: Option<String>,
        /// 更新熟练度
        #[arg(long)]
        confidence: Option<String>,
        /// 更新完成方式
        #[arg(long = "source-kind")]
        source_kind: Option<String>,
        /// 更新训练耗时
        #[arg(long = "time-spent")]
        time_spent: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum BrowserViewArg {
    Files,
    Problems,
}

fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .usage(AnsiColor::Blue.on_default() | Effects::BOLD)
        .literal(AnsiColor::Green.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Yellow.on_default())
        .valid(AnsiColor::Green.on_default() | Effects::BOLD)
        .invalid(AnsiColor::Red.on_default() | Effects::BOLD)
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
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
                    return_to_caller_on_escape: false,
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
