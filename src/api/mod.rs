mod atcoder;
mod luogu;

use std::{collections::HashSet, fs, path::Path};

use chrono::{Duration, FixedOffset, Utc};
use color_eyre::Result;
use tracing::{debug, info, instrument};

use crate::{
    config::{AclogPaths, AppConfig},
    domain::{problem::ProblemMetadata, submission::SubmissionRecord},
    problem::{ProblemProvider, metadata_cache_file_name, provider_key, split_global_problem_id},
};

#[instrument(
    level = "info",
    skip_all,
    fields(problem_id, cache_file = %paths.problems_dir.join(metadata_cache_file_name(problem_id)).display())
)]
pub async fn resolve_problem_metadata(
    config: &AppConfig,
    paths: &AclogPaths,
    problem_id: &str,
) -> Result<Option<ProblemMetadata>> {
    let cache_file = paths
        .problems_dir
        .join(metadata_cache_file_name(problem_id));
    if let Some(metadata) =
        read_cached_metadata(&cache_file, config.settings.problem_metadata_ttl_days())?
    {
        info!("元数据缓存命中");
        return Ok(Some(metadata));
    }
    info!("元数据缓存未命中，转为远端获取");

    let metadata = match split_global_problem_id(problem_id) {
        Some((ProblemProvider::Luogu, raw_id)) => {
            let client = luogu::LuoguClient::new(config)?;
            client
                .fetch_problem_metadata(raw_id, paths, config.settings.problem_metadata_ttl_days())
                .await?
        }
        Some((ProblemProvider::AtCoder, raw_id)) => {
            let client = atcoder::AtCoderProblemsClient::new()?;
            client.fetch_problem_metadata(raw_id).await?
        }
        Some((ProblemProvider::Unknown, _)) | None => None,
    };

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
    let submissions = match split_global_problem_id(problem_id) {
        Some((ProblemProvider::Luogu, raw_id)) => {
            let client = luogu::LuoguClient::new(config)?;
            client
                .fetch_problem_submissions(raw_id, paths, config.settings.luogu_mappings_ttl_days())
                .await?
        }
        Some((ProblemProvider::AtCoder, raw_id)) => {
            let client = atcoder::AtCoderProblemsClient::new()?;
            client.fetch_problem_submissions(config, raw_id).await?
        }
        Some((ProblemProvider::Unknown, _)) | None => Vec::new(),
    };
    info!(submissions = submissions.len(), "已获取提交记录");
    Ok(submissions)
}

#[instrument(level = "info", skip_all, fields(provider = provider_key(provider)))]
pub async fn load_algorithm_tag_names(
    config: &AppConfig,
    paths: &AclogPaths,
    provider: ProblemProvider,
) -> Result<HashSet<String>> {
    let names = match provider {
        ProblemProvider::Luogu => {
            let client = luogu::LuoguClient::new(config)?;
            client
                .load_algorithm_tag_names(paths, config.settings.luogu_tags_ttl_days())
                .await?
        }
        ProblemProvider::AtCoder | ProblemProvider::Unknown => HashSet::new(),
    };
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
