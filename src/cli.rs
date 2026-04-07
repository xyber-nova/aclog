use std::path::PathBuf;

use clap::{Parser, Subcommand};
use color_eyre::Result;

#[derive(Debug, Parser)]
#[command(
    name = "aclog",
    version,
    about = "OI training log tool for jj repositories"
)]
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
    },
    Stats {
        #[arg(long, default_value = ".")]
        workspace: PathBuf,
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
    },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    run_command(cli.command).await
}

async fn run_command(command: Commands) -> Result<()> {
    match command {
        Commands::Init { workspace } => crate::app::run_init(workspace).await,
        Commands::Sync { workspace } => crate::app::run_sync(workspace).await,
        Commands::Stats { workspace } => crate::app::run_stats(workspace).await,
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
        RecordCommands::List { workspace } => crate::app::run_record_list(workspace).await,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser as _;

    use super::{Cli, Commands, RecordCommands};

    #[test]
    fn cli_parses_stats_subcommand_with_default_workspace() {
        let cli = Cli::parse_from(["aclog", "stats"]);

        match cli.command {
            Commands::Stats { workspace } => assert_eq!(workspace, PathBuf::from(".")),
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
    fn cli_parses_record_list_with_default_workspace() {
        let cli = Cli::parse_from(["aclog", "record", "list"]);

        match cli.command {
            Commands::Record {
                command: RecordCommands::List { workspace },
            } => assert_eq!(workspace, PathBuf::from(".")),
            command => panic!("unexpected command: {command:?}"),
        }
    }
}
