use std::{
    fs,
    path::{Path, PathBuf},
};

use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub user: UserConfig,
    pub settings: SettingsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub luogu_uid: String,
    pub luogu_cookie: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsConfig {
    #[serde(default = "default_metadata_ttl_days")]
    pub metadata_ttl_days: i64,
}

#[derive(Debug, Clone)]
pub struct AclogPaths {
    pub workspace_root: PathBuf,
    pub config_file: PathBuf,
    pub problems_dir: PathBuf,
    pub luogu_mappings_file: PathBuf,
    pub luogu_tags_file: PathBuf,
}

fn default_metadata_ttl_days() -> i64 {
    7
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            user: UserConfig {
                luogu_uid: String::new(),
                luogu_cookie: String::new(),
            },
            settings: SettingsConfig {
                metadata_ttl_days: default_metadata_ttl_days(),
            },
        }
    }
}

impl AclogPaths {
    pub fn new(workspace_root: PathBuf) -> Result<Self> {
        let workspace_root = workspace_root.canonicalize().unwrap_or(workspace_root);
        let aclog_dir = workspace_root.join(".aclog");
        let config_file = aclog_dir.join("config.toml");
        let problems_dir = aclog_dir.join("problems");
        let luogu_mappings_file = aclog_dir.join("luogu-mappings.toml");
        let luogu_tags_file = aclog_dir.join("luogu-tags.toml");
        Ok(Self {
            workspace_root,
            config_file,
            problems_dir,
            luogu_mappings_file,
            luogu_tags_file,
        })
    }
}

pub async fn init_workspace(workspace_root: &Path) -> Result<()> {
    let paths = AclogPaths::new(workspace_root.to_path_buf())?;
    info!(workspace = %paths.workspace_root.display(), "正在初始化工作区");
    fs::create_dir_all(&paths.problems_dir).wrap_err("创建 .aclog 目录失败")?;
    debug!(path = %paths.problems_dir.display(), "已确保 .aclog 题目目录存在");
    if !paths.config_file.exists() {
        let content = toml::to_string_pretty(&AppConfig::default())?;
        fs::write(&paths.config_file, format!("{content}\n")).wrap_err("写入配置文件失败")?;
        info!(path = %paths.config_file.display(), "已创建默认配置文件");
    } else {
        info!(path = %paths.config_file.display(), "配置文件已存在");
    }
    crate::vcs::init_repo(&paths.workspace_root).await?;
    info!(workspace = %paths.workspace_root.display(), "工作区初始化完成");
    Ok(())
}

#[instrument(level = "debug", skip_all, fields(config = %paths.config_file.display()))]
pub fn load_config(paths: &AclogPaths) -> Result<AppConfig> {
    let raw = fs::read_to_string(&paths.config_file)
        .wrap_err_with(|| format!("读取配置文件 {} 失败", paths.config_file.display()))?;
    let config: AppConfig = toml::from_str(&raw).wrap_err("config.toml 格式无效")?;
    if config.user.luogu_uid.trim().is_empty() {
        return Err(eyre!("配置项 user.luogu_uid 不能为空"));
    }
    if config.user.luogu_cookie.trim().is_empty() {
        return Err(eyre!("配置项 user.luogu_cookie 不能为空"));
    }
    debug!(
        metadata_ttl_days = config.settings.metadata_ttl_days,
        "配置已加载并校验通过"
    );
    Ok(config)
}
