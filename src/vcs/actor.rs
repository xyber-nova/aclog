use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, Weak},
};

use color_eyre::{Result, eyre::eyre};
use tokio::sync::{mpsc, oneshot};

use crate::domain::record_index::RecordIndex;

use super::{
    ProblemFileChange,
    read::{self},
    write,
};

/// `jj` 仓库访问的共享 handle。
///
/// 设计目的有两层：
/// 1. 向 app 层提供单一、语义化的仓库访问把手，让上层不必显式区分
///    某个操作应该走 `jj-lib` 还是 `jj` CLI。
/// 2. 用单 mailbox 串行化同一次 CLI 命令中的 live 仓库请求，避免未来在
///    更复杂 workflow 中把读写顺序打乱。
///
/// 唯一性约束：
/// - 在同一个 `aclog` 进程内，同一个 `workspace` 最多只会对应一个 live actor
/// - 如果调用方再次请求同一工作区的 handle，会复用已经存在的 actor
/// - 这里的“唯一”只承诺进程内语义，不试图覆盖外部 `jj` CLI 或其他进程
///
/// 这个 handle 本身不持有仓库快照，也不做业务编排。它只负责把请求发给
/// 后台 actor，并把结果原样返回给调用方。
#[derive(Debug, Clone)]
pub struct JjRepoActorHandle {
    shared: Arc<JjRepoActorShared>,
}

#[derive(Debug)]
struct JjRepoActorShared {
    sender: mpsc::Sender<JjRepoActorMsg>,
}

impl JjRepoActorHandle {
    /// 获取当前工作区对应的共享 actor handle。
    ///
    /// 如果同一进程里已经为该工作区创建过 actor，这里会直接复用已有实例；
    /// 否则才会真正启动一个新 actor。
    ///
    /// 作用域边界：
    /// - 一个 actor 只服务一个 `workspace_root`
    /// - 同一进程内，同一工作区只能有一个 live actor
    /// - 它不是跨进程锁，也不是面向外部的后台服务
    ///
    /// 实现上这里会维护一个很小的进程内 registry，仅用于保证“同 workspace
    /// 唯一 actor”这一约束；它不是通用调度中心。
    pub fn for_workspace(workspace_root: PathBuf) -> Self {
        let workspace_key = workspace_registry_key(&workspace_root);
        let registry = actor_registry();
        let mut registry = registry.lock().expect("jj actor registry poisoned");

        if let Some(existing) = registry.get(&workspace_key).and_then(Weak::upgrade) {
            return Self { shared: existing };
        }

        let (sender, receiver) = mpsc::channel(32);
        let shared = Arc::new(JjRepoActorShared { sender });
        registry.insert(workspace_key, Arc::downgrade(&shared));

        std::thread::Builder::new()
            .name("aclog-jj-repo-actor".to_string())
            .spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build jj repo actor runtime")
                    .block_on(run_actor(workspace_root, receiver));
            })
            .expect("failed to spawn jj repo actor thread");
        Self { shared }
    }

    /// 校验目标目录是否已经是可用的 `jj` 工作区。
    pub async fn ensure_workspace(&self) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.shared
            .sender
            .send(JjRepoActorMsg::EnsureWorkspace { reply: reply_tx })
            .await
            .map_err(|_| eyre!("jj 仓库 actor 已停止，无法校验工作区"))?;
        reply_rx
            .await
            .map_err(|_| eyre!("jj 仓库 actor 在返回工作区校验结果前已停止"))?
    }

    /// 读取当前 working copy 相对父提交的题解文件变更。
    ///
    /// 这是“当前工作副本派生”语义，主要服务 `sync`。
    pub async fn detect_working_copy_changes(&self) -> Result<Vec<ProblemFileChange>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.shared
            .sender
            .send(JjRepoActorMsg::DetectWorkingCopyChanges { reply: reply_tx })
            .await
            .map_err(|_| eyre!("jj 仓库 actor 已停止，无法读取工作区变更"))?;
        reply_rx
            .await
            .map_err(|_| eyre!("jj 仓库 actor 在返回工作区变更前已停止"))?
    }

    /// 从整个本地 `jj` 图构建统一的训练记录索引。
    ///
    /// 这是“全图历史派生”语义，供 `record list/show/edit/rebind`、browser、
    /// stats 等历史视图共享。
    pub async fn load_record_index(&self) -> Result<RecordIndex> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.shared
            .sender
            .send(JjRepoActorMsg::LoadRecordIndex { reply: reply_tx })
            .await
            .map_err(|_| eyre!("jj 仓库 actor 已停止，无法加载记录索引"))?;
        reply_rx
            .await
            .map_err(|_| eyre!("jj 仓库 actor 在返回记录索引前已停止"))?
    }

    /// 把调用方传入的 revset 解析成唯一 revision。
    pub async fn resolve_revision(&self, revset_str: &str) -> Result<String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.shared
            .sender
            .send(JjRepoActorMsg::ResolveRevision {
                revset_str: revset_str.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| eyre!("jj 仓库 actor 已停止，无法解析 revision"))?;
        reply_rx
            .await
            .map_err(|_| eyre!("jj 仓库 actor 在返回 revision 前已停止"))?
    }

    /// 判断工作区中的某个仓库内路径当前是否仍被 `jj` 跟踪。
    pub async fn is_tracked_file(&self, repo_relative_path: &str) -> Result<bool> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.shared
            .sender
            .send(JjRepoActorMsg::IsTrackedFile {
                repo_relative_path: repo_relative_path.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| eyre!("jj 仓库 actor 已停止，无法判断文件跟踪状态"))?;
        reply_rx
            .await
            .map_err(|_| eyre!("jj 仓库 actor 在返回文件跟踪状态前已停止"))?
    }

    /// 为一批文件创建结构化训练记录 commit。
    ///
    /// actor 只负责顺序与调度，不负责生成 commit message；消息内容由更上层
    /// workflow 按领域协议先准备好。
    pub async fn create_commits(&self, commits: &[(String, String)]) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.shared
            .sender
            .send(JjRepoActorMsg::CreateCommits {
                commits: commits.to_vec(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| eyre!("jj 仓库 actor 已停止，无法创建提交"))?;
        reply_rx
            .await
            .map_err(|_| eyre!("jj 仓库 actor 在返回创建提交结果前已停止"))?
    }

    /// 重写既有 commit 的描述文本。
    ///
    /// 这对应 `record edit` / `record rebind` 的“改写事实本体”语义，而不是
    /// 追加一条独立修正记录。
    pub async fn rewrite_commit_description(&self, revision: &str, message: &str) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.shared
            .sender
            .send(JjRepoActorMsg::RewriteCommitDescription {
                revision: revision.to_string(),
                message: message.to_string(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| eyre!("jj 仓库 actor 已停止，无法重写提交描述"))?;
        reply_rx
            .await
            .map_err(|_| eyre!("jj 仓库 actor 在返回重写提交结果前已停止"))?
    }
}

fn actor_registry() -> &'static Mutex<HashMap<PathBuf, Weak<JjRepoActorShared>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<PathBuf, Weak<JjRepoActorShared>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn workspace_registry_key(workspace_root: &Path) -> PathBuf {
    workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf())
}

/// actor 内部请求协议。
///
/// 这里按“仓库语义操作”建模，而不是按 `jj-lib` API 或 CLI 子命令建模。
/// 这样上层不会感知底层实现细节，actor 内部仍可以继续保持
/// “只读优先 `jj-lib`、写操作使用 `jj` CLI” 的分工。
#[derive(Debug)]
enum JjRepoActorMsg {
    EnsureWorkspace {
        reply: oneshot::Sender<Result<()>>,
    },
    DetectWorkingCopyChanges {
        reply: oneshot::Sender<Result<Vec<ProblemFileChange>>>,
    },
    LoadRecordIndex {
        reply: oneshot::Sender<Result<RecordIndex>>,
    },
    ResolveRevision {
        revset_str: String,
        reply: oneshot::Sender<Result<String>>,
    },
    IsTrackedFile {
        repo_relative_path: String,
        reply: oneshot::Sender<Result<bool>>,
    },
    CreateCommits {
        commits: Vec<(String, String)>,
        reply: oneshot::Sender<Result<()>>,
    },
    RewriteCommitDescription {
        revision: String,
        message: String,
        reply: oneshot::Sender<Result<()>>,
    },
}

/// actor 主循环。
///
/// 职责边界：
/// - 负责按进入队列的顺序串行执行仓库请求
/// - 负责在读路径委托给 `read.rs`，在写路径委托给 `write.rs`
/// - 负责把执行结果通过 `oneshot` 回给请求方
/// - 负责承载单个工作区的唯一 live actor 实例
///
/// 刻意不负责：
/// - 不缓存长期 repo 视图
/// - 不做业务级决策或数据整形
/// - 不管理跨进程唯一性
async fn run_actor(workspace_root: PathBuf, mut receiver: mpsc::Receiver<JjRepoActorMsg>) {
    while let Some(message) = receiver.recv().await {
        match message {
            JjRepoActorMsg::EnsureWorkspace { reply } => {
                let _ = reply.send(read::ensure_jj_workspace(&workspace_root));
            }
            JjRepoActorMsg::DetectWorkingCopyChanges { reply } => {
                let _ = reply.send(read::collect_changed_problem_files(&workspace_root).await);
            }
            JjRepoActorMsg::LoadRecordIndex { reply } => {
                let _ = reply.send(read::load_record_index(&workspace_root).await);
            }
            JjRepoActorMsg::ResolveRevision { revset_str, reply } => {
                let _ =
                    reply.send(read::resolve_single_commit_id(&workspace_root, &revset_str).await);
            }
            JjRepoActorMsg::IsTrackedFile {
                repo_relative_path,
                reply,
            } => {
                let _ =
                    reply.send(read::is_tracked_file(&workspace_root, &repo_relative_path).await);
            }
            JjRepoActorMsg::CreateCommits { commits, reply } => {
                let _ =
                    reply.send(write::create_commits_for_files(&workspace_root, &commits).await);
            }
            JjRepoActorMsg::RewriteCommitDescription {
                revision,
                message,
                reply,
            } => {
                let _ = reply.send(
                    write::rewrite_commit_description(&workspace_root, &revision, &message).await,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;
    use tokio::sync::oneshot;

    use super::{JjRepoActorHandle, JjRepoActorMsg};
    use crate::vcs::write;

    async fn init_workspace() -> TempDir {
        let dir = TempDir::new().unwrap();
        write::init_repo(dir.path()).await.unwrap();
        dir
    }

    #[tokio::test]
    async fn actor_surfaces_request_errors() {
        let dir = TempDir::new().unwrap();
        let handle = JjRepoActorHandle::for_workspace(dir.path().to_path_buf());

        let error = handle.ensure_workspace().await.unwrap_err();
        assert!(error.to_string().contains("未找到 jj 工作区"));
    }

    #[tokio::test]
    async fn actor_write_then_read_sees_new_history() {
        let dir = init_workspace().await;
        fs::write(dir.path().join("P1001.cpp"), "int main() {}\n").unwrap();
        let handle = JjRepoActorHandle::for_workspace(dir.path().to_path_buf());

        handle
            .create_commits(&[(
                "P1001.cpp".to_string(),
                "solve(P1001): A\n\nSubmission-ID: 1\nFile: P1001.cpp".to_string(),
            )])
            .await
            .unwrap();

        let index = handle.load_record_index().await.unwrap();
        assert_eq!(index.current_by_file().len(), 1);
        assert_eq!(index.current_by_file()[0].problem_id, "luogu:P1001");
    }

    #[tokio::test]
    async fn actor_processes_queued_requests_in_fifo_order() {
        let dir = init_workspace().await;
        fs::write(dir.path().join("P1002.cpp"), "int main() {}\n").unwrap();
        let handle = JjRepoActorHandle::for_workspace(dir.path().to_path_buf());

        handle
            .create_commits(&[(
                "P1002.cpp".to_string(),
                "solve(P1002): Old\n\nSubmission-ID: 1\nFile: P1002.cpp".to_string(),
            )])
            .await
            .unwrap();
        let initial_index = handle.load_record_index().await.unwrap();
        let revision = initial_index.current_by_file()[0].revision.clone();

        let (rewrite_tx, rewrite_rx) = oneshot::channel();
        handle
            .shared
            .sender
            .send(JjRepoActorMsg::RewriteCommitDescription {
                revision,
                message: "solve(P1002): New\n\nSubmission-ID: 2\nFile: P1002.cpp".to_string(),
                reply: rewrite_tx,
            })
            .await
            .unwrap();
        let (index_tx, index_rx) = oneshot::channel();
        handle
            .shared
            .sender
            .send(JjRepoActorMsg::LoadRecordIndex { reply: index_tx })
            .await
            .unwrap();

        rewrite_rx.await.unwrap().unwrap();
        let updated_index = index_rx.await.unwrap().unwrap();
        assert_eq!(updated_index.current_by_file()[0].title, "New");
        assert_eq!(updated_index.current_by_file()[0].submission_id, Some(2));
    }

    #[tokio::test]
    async fn same_workspace_reuses_single_actor_instance() {
        let dir = init_workspace().await;

        let first = JjRepoActorHandle::for_workspace(dir.path().to_path_buf());
        let second = JjRepoActorHandle::for_workspace(dir.path().to_path_buf());

        assert!(std::sync::Arc::ptr_eq(&first.shared, &second.shared));
    }
}
