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
    #[serde(default)]
    pub atcoder_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsConfig {
    #[serde(default = "default_metadata_ttl_days")]
    pub metadata_ttl_days: i64,
    #[serde(default)]
    pub problem_metadata_ttl_days: Option<i64>,
    #[serde(default)]
    pub luogu_mappings_ttl_days: Option<i64>,
    #[serde(default)]
    pub luogu_tags_ttl_days: Option<i64>,
    #[serde(default = "default_review_problem_interval_days")]
    pub review_problem_interval_days: i64,
    #[serde(default = "default_practice_tag_window_days")]
    pub practice_tag_window_days: i64,
    #[serde(default = "default_practice_tag_target_problems")]
    pub practice_tag_target_problems: usize,
}

#[derive(Debug, Clone)]
pub struct AclogPaths {
    pub workspace_root: PathBuf,
    pub config_file: PathBuf,
    pub problems_dir: PathBuf,
    pub luogu_mappings_file: PathBuf,
    pub luogu_tags_file: PathBuf,
    pub sync_session_file: PathBuf,
}

fn default_metadata_ttl_days() -> i64 {
    7
}

fn default_review_problem_interval_days() -> i64 {
    21
}

fn default_practice_tag_window_days() -> i64 {
    60
}

fn default_practice_tag_target_problems() -> usize {
    5
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            user: UserConfig {
                luogu_uid: String::new(),
                luogu_cookie: String::new(),
                atcoder_user_id: None,
            },
            settings: SettingsConfig {
                metadata_ttl_days: default_metadata_ttl_days(),
                problem_metadata_ttl_days: Some(default_metadata_ttl_days()),
                luogu_mappings_ttl_days: Some(default_metadata_ttl_days()),
                luogu_tags_ttl_days: Some(default_metadata_ttl_days()),
                review_problem_interval_days: default_review_problem_interval_days(),
                practice_tag_window_days: default_practice_tag_window_days(),
                practice_tag_target_problems: default_practice_tag_target_problems(),
            },
        }
    }
}

impl SettingsConfig {
    pub fn problem_metadata_ttl_days(&self) -> i64 {
        self.problem_metadata_ttl_days
            .unwrap_or(self.metadata_ttl_days)
    }

    pub fn luogu_mappings_ttl_days(&self) -> i64 {
        self.luogu_mappings_ttl_days
            .unwrap_or(self.metadata_ttl_days)
    }

    pub fn luogu_tags_ttl_days(&self) -> i64 {
        self.luogu_tags_ttl_days.unwrap_or(self.metadata_ttl_days)
    }

    pub fn review_problem_interval_days(&self) -> i64 {
        self.review_problem_interval_days.max(1)
    }

    pub fn practice_tag_window_days(&self) -> i64 {
        self.practice_tag_window_days.max(1)
    }

    pub fn practice_tag_target_problems(&self) -> usize {
        self.practice_tag_target_problems.max(1)
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
        let sync_session_file = aclog_dir.join("sync-session.toml");
        Ok(Self {
            workspace_root,
            config_file,
            problems_dir,
            luogu_mappings_file,
            luogu_tags_file,
            sync_session_file,
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
        review_problem_interval_days = config.settings.review_problem_interval_days(),
        practice_tag_window_days = config.settings.practice_tag_window_days(),
        practice_tag_target_problems = config.settings.practice_tag_target_problems(),
        "配置已加载并校验通过"
    );
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn config_defaults_include_review_settings() {
        let config = AppConfig::default();

        assert_eq!(config.settings.review_problem_interval_days(), 21);
        assert_eq!(config.settings.practice_tag_window_days(), 60);
        assert_eq!(config.settings.practice_tag_target_problems(), 5);
    }

    #[test]
    fn parsing_legacy_config_uses_review_setting_defaults() {
        let raw = r#"
[user]
luogu_uid = "123"
luogu_cookie = "cookie"

[settings]
metadata_ttl_days = 7
problem_metadata_ttl_days = 7
luogu_mappings_ttl_days = 7
luogu_tags_ttl_days = 7
"#;

        let config: AppConfig = toml::from_str(raw).unwrap();

        assert_eq!(config.settings.review_problem_interval_days(), 21);
        assert_eq!(config.settings.practice_tag_window_days(), 60);
        assert_eq!(config.settings.practice_tag_target_problems(), 5);
    }
}
