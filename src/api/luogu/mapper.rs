use std::collections::{HashMap, HashSet};

use chrono::{DateTime, FixedOffset, TimeZone};
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, WrapErr, eyre};
use serde_json::Value;

use crate::domain::submission::SubmissionRecord;

use super::{
    LuoguConfigResponse, LuoguMappingsCache, LuoguRecordStatus, LuoguTagCacheEntry,
    cache::now_in_luogu_timezone,
};

pub fn parse_json_response(body: &str, label: &str) -> Result<Value> {
    serde_json::from_str(body).wrap_err_with(|| {
        let snippet = body.trim().chars().take(120).collect::<String>();
        format!("解析{label}失败，响应不是 JSON：{snippet}")
    })
}

pub fn parse_shared_mappings(body: &str) -> Result<LuoguMappingsCache> {
    let config: LuoguConfigResponse = serde_json::from_str(body).wrap_err_with(|| {
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

pub fn parse_submission_record(
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

pub fn parse_verdict(value: &Value, mappings: Option<&LuoguMappingsCache>) -> String {
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

pub fn map_problem_difficulty(value: i64, mappings: Option<&LuoguMappingsCache>) -> String {
    mappings
        .and_then(|items| items.problem_difficulty.get(&value.to_string()))
        .map(|entry| entry.name.clone())
        .unwrap_or_else(|| value.to_string())
}

pub fn problem_object(value: &Value) -> Result<&serde_json::Map<String, Value>> {
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

pub fn records_array(value: &Value) -> Option<&Vec<Value>> {
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

pub fn parse_problem_tags(
    items: &[Value],
    tag_cache: &HashMap<i64, LuoguTagCacheEntry>,
) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| parse_problem_tag(item, tag_cache))
        .collect()
}

pub fn parse_tag_entry_value(value: &Value) -> Option<LuoguTagCacheEntry> {
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

pub fn algorithm_tag_names(tag_cache: &HashMap<i64, LuoguTagCacheEntry>) -> HashSet<String> {
    tag_cache
        .values()
        .filter(|entry| entry.tag_type == 2)
        .map(|entry| entry.name.clone())
        .collect()
}

pub fn parse_provider_name(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.to_string()),
        Value::Object(object) => object
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::json;

    use super::{
        algorithm_tag_names, map_problem_difficulty, parse_json_response, parse_problem_tags,
        parse_shared_mappings, parse_submission_record, parse_tag_entry_value, parse_verdict,
    };
    use crate::api::luogu::{
        LuoguMappingsCache, LuoguProblemDifficulty, LuoguRecordStatus, LuoguTagCacheEntry,
        LuoguTagsCache,
        cache::{
            now_in_luogu_timezone, read_cached_tags, write_cached_mappings, write_cached_tags,
        },
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
            "user": { "name": "xyber-nova", "uid": 123456 }
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
        assert_eq!(parse_verdict(&json!({"status": 99}), None), "Status-99");
    }

    #[test]
    fn parse_json_response_reports_non_json_body() {
        let error = parse_json_response("<html>not json</html>", "题目元数据").unwrap_err();
        assert!(error.to_string().contains("响应不是 JSON"));
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
        let value = json!({"id": 45, "name": "二分", "type": 2, "parent": 110});
        let tag = parse_tag_entry_value(&value).unwrap();
        assert_eq!(
            tag,
            LuoguTagCacheEntry {
                id: 45,
                name: "二分".to_string(),
                tag_type: 2,
                parent: Some(110)
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
    fn shared_mappings_roundtrip_to_disk() {
        let dir = temp_test_dir("mappings-cache");
        let path = dir.join("luogu-mappings.toml");
        let mappings = parse_shared_mappings(r#"{"recordStatus":{"14":{"id":14,"name":"Unaccepted","shortName":"Unaccepted"}},"problemDifficulty":[{"id":1,"name":"入门"}]}"#).unwrap();
        write_cached_mappings(&path, &mappings).unwrap();
        let loaded = crate::api::luogu::cache::read_cached_mappings(&path, 7);
        assert!(loaded.is_some());
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
