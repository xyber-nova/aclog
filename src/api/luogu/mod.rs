mod cache;
mod client;
mod mapper;

use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

use chrono::{DateTime, FixedOffset};
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info, warn};

use crate::{
    config::{AclogPaths, AppConfig},
    domain::{problem::ProblemMetadata, submission::SubmissionRecord},
};

use self::{
    cache::{
        now_in_luogu_timezone, read_cached_mappings, read_cached_tags, write_cached_mappings,
        write_cached_tags,
    },
    client::{
        build_http_client, fetch_shared_mappings_body, fetch_tags_value, get_problem_body,
        get_record_list_body,
    },
    mapper::{
        algorithm_tag_names, map_problem_difficulty, parse_json_response, parse_problem_tags,
        parse_provider_name, parse_shared_mappings, parse_submission_record, problem_object,
        records_array,
    },
};

const BASE_URL: &str = "https://www.luogu.com.cn";
const USER_AGENT_VALUE: &str = "aclog/0.1 (+https://github.com/xyber-nova/aclog)";
static TAG_CACHE: OnceLock<HashMap<i64, LuoguTagCacheEntry>> = OnceLock::new();
static SHARED_MAPPINGS: OnceLock<LuoguMappingsCache> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LuoguMappingsCache {
    pub(super) fetched_at: DateTime<FixedOffset>,
    #[serde(default)]
    pub(super) record_status: HashMap<String, LuoguRecordStatus>,
    #[serde(default)]
    pub(super) problem_difficulty: HashMap<String, LuoguProblemDifficulty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LuoguRecordStatus {
    pub(super) id: i64,
    pub(super) name: String,
    #[serde(rename = "shortName")]
    pub(super) short_name: String,
    #[serde(default)]
    pub(super) color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LuoguProblemDifficulty {
    pub(super) id: i64,
    pub(super) name: String,
    #[serde(default)]
    pub(super) color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct LuoguTagCacheEntry {
    pub(super) id: i64,
    pub(super) name: String,
    pub(super) tag_type: i64,
    pub(super) parent: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LuoguTagsCache {
    pub(super) fetched_at: DateTime<FixedOffset>,
    pub(super) entries: Vec<LuoguTagCacheEntry>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LuoguConfigResponse {
    #[serde(rename = "recordStatus", default)]
    pub(super) record_status: HashMap<String, LuoguRecordStatus>,
    #[serde(rename = "problemDifficulty", default)]
    pub(super) problem_difficulty: Vec<LuoguProblemDifficulty>,
}

pub struct LuoguClient {
    client: reqwest::Client,
    uid: String,
}

impl LuoguClient {
    pub fn new(config: &AppConfig) -> Result<Self> {
        let client = build_http_client(&config.user.luogu_cookie, USER_AGENT_VALUE)?;
        Ok(Self {
            client,
            uid: config.user.luogu_uid.clone(),
        })
    }

    pub async fn fetch_problem_metadata(
        &self,
        problem_id: &str,
        paths: &AclogPaths,
        ttl_days: i64,
    ) -> Result<Option<ProblemMetadata>> {
        let tag_cache = self.load_tag_cache(paths, ttl_days).await?;
        let mappings = self.load_shared_mappings(paths, ttl_days).await;
        let body = get_problem_body(&self.client, BASE_URL, problem_id).await?;
        let value: Value = parse_json_response(&body, "题目元数据")?;
        let problem = problem_object(&value)?;

        let title = problem
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(problem_id)
            .to_string();
        let difficulty = problem
            .get("difficulty")
            .and_then(Value::as_i64)
            .map(|value| map_problem_difficulty(value, mappings));
        let tags = problem
            .get("tags")
            .and_then(Value::as_array)
            .map(|items| parse_problem_tags(items, tag_cache))
            .unwrap_or_default();
        let source = problem.get("provider").and_then(parse_provider_name);

        Ok(Some(ProblemMetadata {
            id: problem_id.to_string(),
            title,
            difficulty,
            tags,
            source,
            url: format!("{BASE_URL}/problem/{problem_id}"),
            fetched_at: now_in_luogu_timezone(),
        }))
    }

    pub async fn fetch_problem_submissions(
        &self,
        problem_id: &str,
        paths: &AclogPaths,
        ttl_days: i64,
    ) -> Result<Vec<SubmissionRecord>> {
        let mappings = self.load_shared_mappings(paths, ttl_days).await;
        let body = get_record_list_body(&self.client, BASE_URL, &self.uid, problem_id).await?;
        let value: Value = parse_json_response(&body, "提交记录")?;
        let records = records_array(&value)
            .ok_or_else(|| color_eyre::eyre::eyre!("洛谷返回中缺少 records 字段"))?;

        records
            .iter()
            .map(|record| parse_submission_record(record, &self.uid, mappings))
            .collect()
    }

    pub async fn load_algorithm_tag_names(
        &self,
        paths: &AclogPaths,
        ttl_days: i64,
    ) -> Result<HashSet<String>> {
        let tag_cache = self.load_tag_cache(paths, ttl_days).await?;
        Ok(algorithm_tag_names(tag_cache))
    }

    async fn load_tag_cache(
        &self,
        paths: &AclogPaths,
        ttl_days: i64,
    ) -> Result<&'static HashMap<i64, LuoguTagCacheEntry>> {
        if let Some(tag_cache) = TAG_CACHE.get() {
            return Ok(tag_cache);
        }

        if let Some(tag_cache) = read_cached_tags(&paths.luogu_tags_file, ttl_days)? {
            let count = tag_cache.len();
            let _ = TAG_CACHE.set(tag_cache);
            let tag_cache = TAG_CACHE
                .get()
                .ok_or_else(|| color_eyre::eyre::eyre!("标签字典初始化失败"))?;
            debug!(tags = count, "标签字典已从缓存加载");
            return Ok(tag_cache);
        }

        let fetched = self.fetch_tags_from_remote().await?;
        if let Err(error) = write_cached_tags(&paths.luogu_tags_file, &fetched) {
            warn!(path = %paths.luogu_tags_file.display(), ?error, "写入洛谷标签缓存失败");
        } else {
            info!(path = %paths.luogu_tags_file.display(), "已缓存洛谷标签字典");
        }
        let tag_cache = fetched
            .entries
            .into_iter()
            .map(|entry| (entry.id, entry))
            .collect::<HashMap<_, _>>();
        let count = tag_cache.len();
        let _ = TAG_CACHE.set(tag_cache);
        let tag_cache = TAG_CACHE
            .get()
            .ok_or_else(|| color_eyre::eyre::eyre!("标签字典初始化失败"))?;
        debug!(tags = count, "标签字典已加载");
        Ok(tag_cache)
    }

    async fn fetch_tags_from_remote(&self) -> Result<LuoguTagsCache> {
        let value = fetch_tags_value(&self.client, BASE_URL).await?;
        let tags = value
            .get("tags")
            .and_then(Value::as_array)
            .ok_or_else(|| color_eyre::eyre::eyre!("标签字典响应缺少 tags 字段"))?;

        Ok(LuoguTagsCache {
            fetched_at: now_in_luogu_timezone(),
            entries: tags
                .iter()
                .filter_map(mapper::parse_tag_entry_value)
                .collect(),
        })
    }

    async fn load_shared_mappings(
        &self,
        paths: &AclogPaths,
        ttl_days: i64,
    ) -> Option<&'static LuoguMappingsCache> {
        if let Some(mappings) = SHARED_MAPPINGS.get() {
            return Some(mappings);
        }

        if let Some(mappings) = read_cached_mappings(&paths.luogu_mappings_file, ttl_days) {
            let _ = SHARED_MAPPINGS.set(mappings);
            return SHARED_MAPPINGS.get();
        }

        match self.fetch_shared_mappings_from_remote().await {
            Ok(mappings) => {
                if let Err(error) = write_cached_mappings(&paths.luogu_mappings_file, &mappings) {
                    warn!(path = %paths.luogu_mappings_file.display(), ?error, "写入洛谷映射缓存失败");
                } else {
                    info!(path = %paths.luogu_mappings_file.display(), "已缓存洛谷映射表");
                }
                let _ = SHARED_MAPPINGS.set(mappings);
                SHARED_MAPPINGS.get()
            }
            Err(error) => {
                warn!(?error, "加载洛谷映射表失败，将回退到原始编号显示");
                None
            }
        }
    }

    async fn fetch_shared_mappings_from_remote(&self) -> Result<LuoguMappingsCache> {
        let body = fetch_shared_mappings_body(&self.client, BASE_URL).await?;
        parse_shared_mappings(&body)
    }
}
