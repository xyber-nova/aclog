mod luogu;

use std::{collections::HashSet, fs, path::Path};

use chrono::{Duration, FixedOffset, Utc};
use color_eyre::Result;
use tracing::{debug, info, instrument};

use crate::{
    config::{AclogPaths, AppConfig},
    models::{ProblemMetadata, SubmissionRecord},
};

#[instrument(
    level = "info",
    skip_all,
    fields(problem_id, cache_file = %paths.problems_dir.join(format!("{problem_id}.toml")).display())
)]
pub async fn resolve_problem_metadata(
    config: &AppConfig,
    paths: &AclogPaths,
    problem_id: &str,
) -> Result<Option<ProblemMetadata>> {
    let cache_file = paths.problems_dir.join(format!("{problem_id}.toml"));
    if let Some(metadata) = read_cached_metadata(&cache_file, config.settings.metadata_ttl_days)? {
        info!("元数据缓存命中");
        return Ok(Some(metadata));
    }
    info!("元数据缓存未命中，转为远端获取");

    let client = luogu::LuoguClient::new(config)?;
    let metadata = client
        .fetch_problem_metadata(problem_id, paths, config.settings.metadata_ttl_days)
        .await?;
    if let Some(metadata) = &metadata {
        fs::write(
            &cache_file,
            format!("{}\n", toml::to_string_pretty(metadata)?),
        )?;
        info!(path = %cache_file.display(), "已缓存拉取到的元数据");
    } else {
        info!("未找到题目元数据");
    }
    Ok(metadata)
}

#[instrument(level = "info", skip_all, fields(problem_id))]
pub async fn fetch_problem_submissions(
    config: &AppConfig,
    paths: &AclogPaths,
    problem_id: &str,
) -> Result<Vec<SubmissionRecord>> {
    let client = luogu::LuoguClient::new(config)?;
    let submissions = client
        .fetch_problem_submissions(problem_id, paths, config.settings.metadata_ttl_days)
        .await?;
    info!(submissions = submissions.len(), "已获取提交记录");
    Ok(submissions)
}

#[instrument(level = "info", skip_all)]
pub async fn load_algorithm_tag_names(
    config: &AppConfig,
    paths: &AclogPaths,
) -> Result<HashSet<String>> {
    let client = luogu::LuoguClient::new(config)?;
    let names = client
        .load_algorithm_tag_names(paths, config.settings.metadata_ttl_days)
        .await?;
    info!(algorithm_tags = names.len(), "已加载算法标签集合");
    Ok(names)
}

fn read_cached_metadata(path: &Path, ttl_days: i64) -> Result<Option<ProblemMetadata>> {
    if !path.exists() {
        debug!(path = %path.display(), "元数据缓存文件不存在");
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    let metadata: ProblemMetadata = toml::from_str(&raw)?;
    if metadata.tags.iter().any(|tag| is_legacy_numeric_tag(tag)) {
        debug!(path = %path.display(), "检测到旧版数字标签缓存，准备刷新");
        return Ok(None);
    }
    let now =
        Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).expect("固定时区偏移应当有效"));
    if now - metadata.fetched_at > Duration::days(ttl_days) {
        debug!(
            path = %path.display(),
            fetched_at = %metadata.fetched_at,
            ttl_days,
            "元数据缓存已过期"
        );
        return Ok(None);
    }
    debug!(
        path = %path.display(),
        fetched_at = %metadata.fetched_at,
        ttl_days,
        "元数据缓存仍然有效"
    );
    Ok(Some(metadata))
}

fn is_legacy_numeric_tag(tag: &str) -> bool {
    let Some(trimmed) = tag.strip_prefix('#') else {
        return false;
    };
    !trimmed.is_empty() && trimmed.chars().all(|ch| ch.is_ascii_digit())
}
