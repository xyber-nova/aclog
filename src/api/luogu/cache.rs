use std::{collections::HashMap, fs, path::Path};

use chrono::{DateTime, Duration, FixedOffset, Utc};
use color_eyre::Result;
use tracing::{debug, warn};

use super::{LuoguMappingsCache, LuoguTagCacheEntry, LuoguTagsCache};

pub fn read_cached_tags(
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

pub fn write_cached_tags(path: &Path, cache: &LuoguTagsCache) -> Result<()> {
    let raw = toml::to_string_pretty(cache)?;
    fs::write(path, format!("{raw}\n"))?;
    Ok(())
}

pub fn read_cached_mappings(path: &Path, ttl_days: i64) -> Option<LuoguMappingsCache> {
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

pub fn write_cached_mappings(path: &Path, mappings: &LuoguMappingsCache) -> Result<()> {
    let raw = toml::to_string_pretty(mappings)?;
    fs::write(path, format!("{raw}\n"))?;
    Ok(())
}

pub fn now_in_luogu_timezone() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).expect("固定时区偏移应当有效"))
}
