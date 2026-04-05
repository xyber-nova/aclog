use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    sync::OnceLock,
};

use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, WrapErr, eyre};
use reqwest::{
    Client,
    header::{COOKIE, HeaderMap, HeaderValue, USER_AGENT},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, info, instrument, warn};

use crate::{
    config::{AclogPaths, AppConfig},
    models::{ProblemMetadata, SubmissionRecord},
};

const BASE_URL: &str = "https://www.luogu.com.cn";
const USER_AGENT_VALUE: &str = "aclog/0.1 (+https://github.com/xyber-nova/aclog)";
static TAG_CACHE: OnceLock<HashMap<i64, LuoguTagCacheEntry>> = OnceLock::new();
static SHARED_MAPPINGS: OnceLock<LuoguMappingsCache> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LuoguMappingsCache {
    fetched_at: DateTime<FixedOffset>,
    #[serde(default)]
    record_status: HashMap<String, LuoguRecordStatus>,
    #[serde(default)]
    problem_difficulty: HashMap<String, LuoguProblemDifficulty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LuoguRecordStatus {
    id: i64,
    name: String,
    #[serde(rename = "shortName")]
    short_name: String,
    #[serde(default)]
    color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LuoguProblemDifficulty {
    id: i64,
    name: String,
    #[serde(default)]
    color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct LuoguTagCacheEntry {
    id: i64,
    name: String,
    tag_type: i64,
    parent: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LuoguTagsCache {
    fetched_at: DateTime<FixedOffset>,
    entries: Vec<LuoguTagCacheEntry>,
}

#[derive(Debug, Deserialize)]
struct LuoguConfigResponse {
    #[serde(rename = "recordStatus", default)]
    record_status: HashMap<String, LuoguRecordStatus>,
    #[serde(rename = "problemDifficulty", default)]
    problem_difficulty: Vec<LuoguProblemDifficulty>,
}

pub struct LuoguClient {
    client: Client,
    uid: String,
}

impl LuoguClient {
    pub fn new(config: &AppConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&config.user.luogu_cookie).wrap_err("luogu_cookie 请求头无效")?,
        );
        let client = Client::builder().default_headers(headers).build()?;
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
        let url = format!("{BASE_URL}/problem/{problem_id}");
        debug!(%url, "请求题目元数据");
        let value: Value = self
            .client
            .get(url)
            .header("x-lentille-request", "content-only")
            .query(&[("_contentOnly", "1")])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
            .wrap_err("读取题目元数据响应失败")
            .and_then(|body| parse_json_response(&body, "题目元数据"))?;
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

        let fetched_at = now_in_luogu_timezone();

        Ok(Some(ProblemMetadata {
            id: problem_id.to_string(),
            title,
            difficulty,
            tags,
            source,
            url: format!("{BASE_URL}/problem/{problem_id}"),
            fetched_at,
        }))
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
            let tag_cache = TAG_CACHE.get().ok_or_else(|| eyre!("标签字典初始化失败"))?;
            debug!(tags = count, "标签字典已从缓存加载");
            return Ok(tag_cache);
        }

        let fetched = self.fetch_tags_from_remote().await?;
        if let Err(error) = write_cached_tags(&paths.luogu_tags_file, &fetched) {
            warn!(
                path = %paths.luogu_tags_file.display(),
                ?error,
                "写入洛谷标签缓存失败"
            );
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
        let tag_cache = TAG_CACHE.get().ok_or_else(|| eyre!("标签字典初始化失败"))?;
        debug!(tags = count, "标签字典已加载");
        Ok(tag_cache)
    }

    pub async fn load_algorithm_tag_names(
        &self,
        paths: &AclogPaths,
        ttl_days: i64,
    ) -> Result<HashSet<String>> {
        let tag_cache = self.load_tag_cache(paths, ttl_days).await?;
        Ok(algorithm_tag_names(tag_cache))
    }

    async fn fetch_tags_from_remote(&self) -> Result<LuoguTagsCache> {
        let url = format!("{BASE_URL}/_lfe/tags");
        debug!(%url, "请求标签字典");
        let value: Value = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .wrap_err("解析标签字典失败")?;
        let tags = value
            .get("tags")
            .and_then(Value::as_array)
            .ok_or_else(|| eyre!("标签字典响应缺少 tags 字段"))?;

        Ok(LuoguTagsCache {
            fetched_at: now_in_luogu_timezone(),
            entries: tags.iter().filter_map(parse_tag_entry_value).collect(),
        })
    }

    pub async fn fetch_problem_submissions(
        &self,
        problem_id: &str,
        paths: &AclogPaths,
        ttl_days: i64,
    ) -> Result<Vec<SubmissionRecord>> {
        let mappings = self.load_shared_mappings(paths, ttl_days).await;
        let url = format!("{BASE_URL}/record/list");
        debug!(%url, uid = self.uid, "请求题目提交记录");
        let value: Value = self
            .client
            .get(url)
            .header("x-lentille-request", "content-only")
            .query(&[
                ("user", self.uid.as_str()),
                ("pid", problem_id),
                ("_contentOnly", "1"),
            ])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
            .wrap_err("读取提交记录响应失败")
            .and_then(|body| parse_json_response(&body, "提交记录"))?;
        let records = records_array(&value).ok_or_else(|| eyre!("洛谷返回中缺少 records 字段"))?;

        records
            .iter()
            .map(|record| parse_submission_record(record, &self.uid, mappings))
            .collect()
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
                match toml::to_string_pretty(&mappings) {
                    Ok(raw) => {
                        if let Err(error) =
                            fs::write(&paths.luogu_mappings_file, format!("{raw}\n"))
                        {
                            warn!(
                                path = %paths.luogu_mappings_file.display(),
                                ?error,
                                "写入洛谷映射缓存失败"
                            );
                        } else {
                            info!(path = %paths.luogu_mappings_file.display(), "已缓存洛谷映射表");
                        }
                    }
                    Err(error) => warn!(?error, "序列化洛谷映射缓存失败"),
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
        let url = format!("{BASE_URL}/_lfe/config");
        debug!(%url, "请求洛谷共享映射表");
        let body = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
            .wrap_err("读取洛谷共享映射响应失败")?;
        let config: LuoguConfigResponse = serde_json::from_str(&body).wrap_err_with(|| {
            let snippet = body.trim().chars().take(120).collect::<String>();
            format!("解析洛谷共享映射失败，响应不是预期 JSON：{snippet}")
        })?;

        Ok(LuoguMappingsCache {
            fetched_at: now_in_luogu_timezone(),
            record_status: config.record_status.into_iter().collect(),
            problem_difficulty: config
                .problem_difficulty
                .into_iter()
                .map(|entry| (entry.id.to_string(), entry))
                .collect(),
        })
    }
}

fn parse_json_response(body: &str, label: &str) -> Result<Value> {
    serde_json::from_str(body).wrap_err_with(|| {
        let snippet = body.trim().chars().take(120).collect::<String>();
        format!("解析{label}失败，响应不是 JSON：{snippet}")
    })
}

#[instrument(level = "debug", skip_all)]
fn parse_submission_record(
    value: &Value,
    fallback_submitter: &str,
    mappings: Option<&LuoguMappingsCache>,
) -> Result<SubmissionRecord> {
    let submission_id = value
        .get("id")
        .or_else(|| value.get("rid"))
        .and_then(Value::as_u64)
        .ok_or_else(|| eyre!("缺少提交编号"))?;
    let verdict = parse_verdict(value, mappings);
    let score = value.get("score").and_then(Value::as_i64);
    let time_ms = value
        .get("time")
        .or_else(|| value.get("timeCost"))
        .and_then(Value::as_u64);
    let memory_mb = value
        .get("memory")
        .or_else(|| value.get("memoryCost"))
        .and_then(Value::as_f64)
        .map(|value| value / 1024.0);
    let submitted_at = value
        .get("submitTime")
        .or_else(|| value.get("submit_time"))
        .and_then(Value::as_i64)
        .and_then(parse_timestamp);
    let submitter = parse_submitter(value).unwrap_or_else(|| fallback_submitter.to_string());

    Ok(SubmissionRecord {
        submission_id,
        submitter,
        verdict,
        score,
        time_ms,
        memory_mb,
        submitted_at,
    })
}

fn parse_verdict(value: &Value, mappings: Option<&LuoguMappingsCache>) -> String {
    let text = [
        value.get("statusName"),
        value.get("resultName"),
        value.get("status"),
        value.get("result"),
    ]
    .into_iter()
    .flatten()
    .find_map(|item| item.as_str())
    .map(str::trim)
    .filter(|item| !item.is_empty());

    if let Some(text) = text {
        return text.to_string();
    }

    let code = [
        value.get("status"),
        value.get("result"),
        value.get("statusCode"),
    ]
    .into_iter()
    .flatten()
    .find_map(Value::as_i64);

    code.map(|code| map_record_status(code, mappings))
        .unwrap_or_else(|| "UNKNOWN".to_string())
}

fn map_record_status(code: i64, mappings: Option<&LuoguMappingsCache>) -> String {
    if let Some(mapped) = mappings
        .and_then(|items| items.record_status.get(&code.to_string()))
        .map(preferred_status_name)
    {
        return mapped;
    }
    format!("Status-{code}")
}

fn preferred_status_name(entry: &LuoguRecordStatus) -> String {
    if !entry.short_name.trim().is_empty() {
        return entry.short_name.trim().to_string();
    }
    entry.name.trim().to_string()
}

fn map_problem_difficulty(value: i64, mappings: Option<&LuoguMappingsCache>) -> String {
    mappings
        .and_then(|items| items.problem_difficulty.get(&value.to_string()))
        .map(|entry| entry.name.clone())
        .unwrap_or_else(|| value.to_string())
}

fn parse_submitter(value: &Value) -> Option<String> {
    let direct = [
        value.get("userName"),
        value.get("username"),
        value.get("uname"),
        value.get("user").filter(|item| item.is_string()),
    ]
    .into_iter()
    .flatten()
    .find_map(Value::as_str)
    .map(str::trim)
    .filter(|item| !item.is_empty())
    .map(ToOwned::to_owned);

    if direct.is_some() {
        return direct;
    }

    let user = value.get("user")?.as_object()?;
    let name = user
        .get("name")
        .or_else(|| user.get("username"))
        .or_else(|| user.get("uname"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned);
    let uid = user
        .get("uid")
        .or_else(|| user.get("id"))
        .and_then(|item| match item {
            Value::String(text) => Some(text.trim().to_string()),
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        })
        .filter(|item| !item.is_empty());

    match (name, uid) {
        (Some(name), Some(uid)) if name != uid => Some(format!("{name} ({uid})")),
        (Some(name), _) => Some(name),
        (_, Some(uid)) => Some(uid),
        _ => None,
    }
}

fn parse_timestamp(timestamp: i64) -> Option<DateTime<FixedOffset>> {
    let offset = FixedOffset::east_opt(8 * 3600)?;
    offset.timestamp_opt(timestamp, 0).single()
}

fn records_array(value: &Value) -> Option<&Vec<Value>> {
    let records = value
        .get("records")
        .or_else(|| value.get("data").and_then(|item| item.get("records")))
        .or_else(|| {
            value
                .get("currentData")
                .and_then(|item| item.get("records"))
        })?;

    records
        .as_array()
        .or_else(|| records.get("result").and_then(Value::as_array))
}

fn problem_object(value: &Value) -> Result<&serde_json::Map<String, Value>> {
    value
        .get("data")
        .and_then(|data| data.get("problem"))
        .or_else(|| {
            value
                .get("currentData")
                .and_then(|data| data.get("problem"))
        })
        .or_else(|| value.get("problem"))
        .and_then(Value::as_object)
        .ok_or_eyre("缺少题目数据")
}

fn parse_problem_tags(
    items: &[Value],
    tag_cache: &HashMap<i64, LuoguTagCacheEntry>,
) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| parse_problem_tag(item, tag_cache))
        .collect()
}

fn parse_problem_tag(item: &Value, tag_cache: &HashMap<i64, LuoguTagCacheEntry>) -> Option<String> {
    match item {
        Value::Number(number) => number
            .as_i64()
            .and_then(|id| tag_cache.get(&id))
            .map(|tag| tag.name.clone()),
        Value::Object(object) => {
            if let Some(id) = object.get("id").and_then(Value::as_i64) {
                return tag_cache.get(&id).map(|tag| tag.name.clone());
            }

            object
                .get("name")
                .and_then(Value::as_str)
                .or_else(|| object.get("fullName").and_then(Value::as_str))
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
        }
        Value::String(text) => Some(text.to_string()),
        _ => None,
    }
}

fn parse_tag_entry(value: &Value) -> Option<(i64, LuoguTagCacheEntry)> {
    let entry = parse_tag_entry_value(value)?;
    Some((entry.id, entry))
}

fn parse_tag_entry_value(value: &Value) -> Option<LuoguTagCacheEntry> {
    let object = value.as_object()?;
    let id = object.get("id")?.as_i64()?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| object.get("fullName").and_then(Value::as_str))?
        .trim();
    if name.is_empty() {
        return None;
    }
    Some(LuoguTagCacheEntry {
        id,
        name: name.to_string(),
        tag_type: object
            .get("type")
            .and_then(Value::as_i64)
            .unwrap_or_default(),
        parent: object.get("parent").and_then(Value::as_i64),
    })
}

fn algorithm_tag_names(tag_cache: &HashMap<i64, LuoguTagCacheEntry>) -> HashSet<String> {
    tag_cache
        .values()
        .filter(|entry| entry.tag_type == 2)
        .map(|entry| entry.name.clone())
        .collect()
}

fn read_cached_tags(
    path: &Path,
    ttl_days: i64,
) -> Result<Option<HashMap<i64, LuoguTagCacheEntry>>> {
    if !path.exists() {
        debug!(path = %path.display(), "洛谷标签缓存文件不存在");
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    let cache: LuoguTagsCache = toml::from_str(&raw)?;
    if now_in_luogu_timezone() - cache.fetched_at > Duration::days(ttl_days) {
        debug!(
            path = %path.display(),
            fetched_at = %cache.fetched_at,
            ttl_days,
            "洛谷标签缓存已过期"
        );
        return Ok(None);
    }
    Ok(Some(
        cache
            .entries
            .into_iter()
            .map(|entry| (entry.id, entry))
            .collect(),
    ))
}

fn write_cached_tags(path: &Path, cache: &LuoguTagsCache) -> Result<()> {
    let raw = toml::to_string_pretty(cache)?;
    fs::write(path, format!("{raw}\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use chrono::{FixedOffset, TimeZone};

    use serde_json::json;

    use super::{
        LuoguMappingsCache, LuoguProblemDifficulty, LuoguRecordStatus, LuoguTagCacheEntry,
        LuoguTagsCache, algorithm_tag_names, map_problem_difficulty, now_in_luogu_timezone,
        parse_json_response, parse_problem_tags, parse_submission_record, parse_submitter,
        parse_tag_entry, parse_verdict, read_cached_tags, write_cached_tags,
    };

    #[test]
    fn parse_submission_record_prefers_textual_verdict_and_submitter() {
        let value = json!({
            "rid": 123456,
            "statusName": "AC",
            "score": 100,
            "time": 50,
            "memory": 1024.0,
            "submitTime": 1705300320,
            "user": {
                "name": "xyber-nova",
                "uid": 123456
            }
        });

        let record = parse_submission_record(&value, "fallback-user", None).unwrap();

        assert_eq!(record.submission_id, 123456);
        assert_eq!(record.verdict, "AC");
        assert_eq!(record.submitter, "xyber-nova (123456)");
    }

    #[test]
    fn parse_submission_record_falls_back_to_status_code_and_requested_user() {
        let value = json!({ "id": 42, "status": 12 });
        let mappings = LuoguMappingsCache {
            fetched_at: now_in_luogu_timezone(),
            record_status: HashMap::from([(
                "12".to_string(),
                LuoguRecordStatus {
                    id: 12,
                    name: "Accepted".to_string(),
                    short_name: "AC".to_string(),
                    color: Some("green-3".to_string()),
                },
            )]),
            problem_difficulty: HashMap::new(),
        };

        let record = parse_submission_record(&value, "fallback-user", Some(&mappings)).unwrap();

        assert_eq!(record.verdict, "AC");
        assert_eq!(record.submitter, "fallback-user");
    }

    #[test]
    fn parse_verdict_handles_unknown_numeric_codes() {
        let value = json!({
            "status": 99
        });

        assert_eq!(parse_verdict(&value, None), "Status-99");
    }

    #[test]
    fn parse_submitter_accepts_direct_string_fields() {
        let value = json!({
            "userName": "xyber-nova"
        });

        assert_eq!(parse_submitter(&value), Some("xyber-nova".to_string()));
    }

    #[test]
    fn parse_json_response_reports_non_json_body() {
        let error = parse_json_response("<html>not json</html>", "题目元数据").unwrap_err();

        assert!(error.to_string().contains("响应不是 JSON"));
        assert!(error.to_string().contains("<html>not json</html>"));
    }

    #[test]
    fn parse_submission_record_uses_config_mapping_for_unaccepted_status() {
        let value = json!({ "id": 42, "status": 14 });
        let mappings = LuoguMappingsCache {
            fetched_at: now_in_luogu_timezone(),
            record_status: HashMap::from([(
                "14".to_string(),
                LuoguRecordStatus {
                    id: 14,
                    name: "Unaccepted".to_string(),
                    short_name: "Unaccepted".to_string(),
                    color: Some("red-3".to_string()),
                },
            )]),
            problem_difficulty: HashMap::new(),
        };

        let record = parse_submission_record(&value, "fallback-user", Some(&mappings)).unwrap();

        assert_eq!(record.verdict, "Unaccepted");
    }

    #[test]
    fn map_problem_difficulty_prefers_config_mapping_and_falls_back_to_raw_id() {
        let mappings = LuoguMappingsCache {
            fetched_at: now_in_luogu_timezone(),
            record_status: HashMap::new(),
            problem_difficulty: HashMap::from([(
                "1".to_string(),
                LuoguProblemDifficulty {
                    id: 1,
                    name: "入门".to_string(),
                    color: Some("pink-3".to_string()),
                },
            )]),
        };

        assert_eq!(map_problem_difficulty(1, Some(&mappings)), "入门");
        assert_eq!(map_problem_difficulty(6, None), "6");
    }

    #[test]
    fn parse_tag_entry_keeps_name_type_and_parent() {
        let value = json!({
            "id": 45,
            "name": "二分",
            "type": 2,
            "parent": 110
        });

        let (id, tag) = parse_tag_entry(&value).unwrap();

        assert_eq!(id, 45);
        assert_eq!(
            tag,
            LuoguTagCacheEntry {
                id: 45,
                name: "二分".to_string(),
                tag_type: 2,
                parent: Some(110),
            }
        );
    }

    #[test]
    fn parse_problem_tags_keeps_raw_tags_from_cache() {
        let cache = HashMap::from([
            (
                45,
                LuoguTagCacheEntry {
                    id: 45,
                    name: "二分".to_string(),
                    tag_type: 2,
                    parent: Some(110),
                },
            ),
            (
                82,
                LuoguTagCacheEntry {
                    id: 82,
                    name: "NOIP 普及组".to_string(),
                    tag_type: 3,
                    parent: Some(426),
                },
            ),
        ]);

        let tags = parse_problem_tags(&[json!(45), json!(82)], &cache);

        assert_eq!(tags, vec!["二分".to_string(), "NOIP 普及组".to_string()]);
    }

    #[test]
    fn parse_problem_tags_accepts_raw_tag_objects() {
        let cache = HashMap::new();
        let tags = parse_problem_tags(
            &[
                json!({"name": "模拟", "type": 2}),
                json!({"name": "NOI", "type": 3}),
            ],
            &cache,
        );

        assert_eq!(tags, vec!["模拟".to_string(), "NOI".to_string()]);
    }

    #[test]
    fn algorithm_tag_names_only_returns_type_two_entries() {
        let cache = HashMap::from([
            (
                45,
                LuoguTagCacheEntry {
                    id: 45,
                    name: "二分".to_string(),
                    tag_type: 2,
                    parent: Some(110),
                },
            ),
            (
                82,
                LuoguTagCacheEntry {
                    id: 82,
                    name: "NOIP 普及组".to_string(),
                    tag_type: 3,
                    parent: Some(426),
                },
            ),
        ]);

        let names = algorithm_tag_names(&cache);

        assert_eq!(names, HashSet::from([String::from("二分")]));
    }

    #[test]
    fn read_cached_tags_returns_fresh_entries_from_disk() {
        let dir = temp_test_dir("fresh-tags-cache");
        let path = dir.join("luogu-tags.toml");
        let cache = LuoguTagsCache {
            fetched_at: now_in_luogu_timezone(),
            entries: vec![LuoguTagCacheEntry {
                id: 45,
                name: "二分".to_string(),
                tag_type: 2,
                parent: Some(110),
            }],
        };

        write_cached_tags(&path, &cache).unwrap();
        let loaded = read_cached_tags(&path, 7).unwrap().unwrap();

        assert_eq!(loaded.get(&45).unwrap().name, "二分");
        cleanup_temp_test_dir(dir);
    }

    #[test]
    fn read_cached_tags_ignores_expired_cache() {
        let dir = temp_test_dir("expired-tags-cache");
        let path = dir.join("luogu-tags.toml");
        let cache = LuoguTagsCache {
            fetched_at: FixedOffset::east_opt(8 * 3600)
                .unwrap()
                .with_ymd_and_hms(2024, 1, 1, 0, 0, 0)
                .single()
                .unwrap(),
            entries: vec![LuoguTagCacheEntry {
                id: 45,
                name: "二分".to_string(),
                tag_type: 2,
                parent: Some(110),
            }],
        };

        write_cached_tags(&path, &cache).unwrap();
        let loaded = read_cached_tags(&path, 1).unwrap();

        assert!(loaded.is_none());
        cleanup_temp_test_dir(dir);
    }

    fn temp_test_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("aclog-{label}-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup_temp_test_dir(dir: PathBuf) {
        let _ = fs::remove_dir_all(dir);
    }
}

fn parse_provider_name(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.to_string()),
        Value::Object(object) => object
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        _ => None,
    }
}

fn read_cached_mappings(path: &std::path::Path, ttl_days: i64) -> Option<LuoguMappingsCache> {
    if !path.exists() {
        debug!(path = %path.display(), "洛谷映射缓存文件不存在");
        return None;
    }

    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            warn!(path = %path.display(), ?error, "读取洛谷映射缓存失败");
            return None;
        }
    };
    let mappings: LuoguMappingsCache = match toml::from_str(&raw) {
        Ok(mappings) => mappings,
        Err(error) => {
            warn!(path = %path.display(), ?error, "解析洛谷映射缓存失败，将忽略旧缓存");
            return None;
        }
    };

    if now_in_luogu_timezone() - mappings.fetched_at > Duration::days(ttl_days) {
        debug!(
            path = %path.display(),
            fetched_at = %mappings.fetched_at,
            ttl_days,
            "洛谷映射缓存已过期"
        );
        return None;
    }

    debug!(
        path = %path.display(),
        fetched_at = %mappings.fetched_at,
        ttl_days,
        "洛谷映射缓存仍然有效"
    );
    Some(mappings)
}

fn now_in_luogu_timezone() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).expect("固定时区偏移应当有效"))
}
