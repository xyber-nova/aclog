use std::{borrow::Cow, sync::OnceLock};

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProblemProvider {
    #[default]
    Luogu,
    AtCoder,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedProblemTarget {
    pub provider: ProblemProvider,
    pub raw_id: String,
    pub global_id: String,
}

pub fn extract_problem_target(file_name: &str) -> Option<ParsedProblemTarget> {
    extract_luogu_problem_id(file_name)
        .map(|raw_id| ParsedProblemTarget {
            provider: ProblemProvider::Luogu,
            global_id: global_problem_id(ProblemProvider::Luogu, &raw_id),
            raw_id,
        })
        .or_else(|| {
            extract_atcoder_task_id(file_name).map(|raw_id| ParsedProblemTarget {
                provider: ProblemProvider::AtCoder,
                global_id: global_problem_id(ProblemProvider::AtCoder, &raw_id),
                raw_id,
            })
        })
}

pub fn extract_luogu_problem_id(file_name: &str) -> Option<String> {
    let captures = luogu_problem_file_regex().captures(file_name)?;
    Some(captures.get(1)?.as_str().to_uppercase())
}

pub fn extract_atcoder_task_id(file_name: &str) -> Option<String> {
    if let Some(captures) = atcoder_contest_task_file_regex().captures(file_name) {
        let contest = captures.get(1)?.as_str().to_ascii_lowercase();
        let suffix = captures.get(2)?.as_str().to_ascii_lowercase();
        return Some(format!("{contest}_{suffix}"));
    }
    let captures = atcoder_task_file_regex().captures(file_name)?;
    Some(captures.get(1)?.as_str().to_ascii_lowercase())
}

pub fn is_luogu_problem_id(problem_id: &str) -> bool {
    luogu_problem_id_regex().is_match(problem_id)
}

pub fn is_atcoder_task_id(problem_id: &str) -> bool {
    atcoder_task_id_regex().is_match(problem_id)
}

pub fn global_problem_id(provider: ProblemProvider, raw_id: &str) -> String {
    match provider {
        ProblemProvider::Luogu => format!("luogu:{}", raw_id.to_ascii_uppercase()),
        ProblemProvider::AtCoder => format!("atcoder:{}", raw_id.to_ascii_lowercase()),
        ProblemProvider::Unknown => format!("unknown:{raw_id}"),
    }
}

pub fn provider_key(provider: ProblemProvider) -> &'static str {
    match provider {
        ProblemProvider::Luogu => "luogu",
        ProblemProvider::AtCoder => "atcoder",
        ProblemProvider::Unknown => "unknown",
    }
}

pub fn provider_label(provider: ProblemProvider) -> &'static str {
    match provider {
        ProblemProvider::Luogu => "Luogu",
        ProblemProvider::AtCoder => "AtCoder",
        ProblemProvider::Unknown => "Unknown",
    }
}

pub fn provider_from_source(source: &str) -> Option<ProblemProvider> {
    let normalized = source.trim();
    if normalized.eq_ignore_ascii_case("luogu") {
        return Some(ProblemProvider::Luogu);
    }
    if normalized.eq_ignore_ascii_case("atcoder") {
        return Some(ProblemProvider::AtCoder);
    }
    None
}

pub fn split_global_problem_id(problem_id: &str) -> Option<(ProblemProvider, &str)> {
    let (provider, raw_id) = problem_id.split_once(':')?;
    let provider = match provider.trim().to_ascii_lowercase().as_str() {
        "luogu" => ProblemProvider::Luogu,
        "atcoder" => ProblemProvider::AtCoder,
        "unknown" => ProblemProvider::Unknown,
        _ => return None,
    };
    if raw_id.trim().is_empty() {
        return None;
    }
    Some((provider, raw_id.trim()))
}

pub fn normalize_problem_id_with_source(problem_id: &str, source: Option<&str>) -> String {
    let trimmed = problem_id.trim();
    if trimmed.is_empty() {
        return global_problem_id(ProblemProvider::Unknown, "missing-problem-id");
    }
    if let Some((provider, raw_id)) = split_global_problem_id(trimmed) {
        return global_problem_id(provider, raw_id);
    }
    if is_luogu_problem_id(trimmed) {
        if matches!(
            source.and_then(provider_from_source),
            Some(ProblemProvider::AtCoder)
        ) {
            return global_problem_id(ProblemProvider::Unknown, trimmed);
        }
        return global_problem_id(ProblemProvider::Luogu, trimmed);
    }
    if is_atcoder_task_id(trimmed)
        && matches!(
            source.and_then(provider_from_source),
            Some(ProblemProvider::AtCoder)
        )
    {
        return global_problem_id(ProblemProvider::AtCoder, trimmed);
    }
    global_problem_id(ProblemProvider::Unknown, trimmed)
}

pub fn provider_from_problem_id(problem_id: &str) -> ProblemProvider {
    split_global_problem_id(problem_id)
        .map(|(provider, _)| provider)
        .unwrap_or(ProblemProvider::Unknown)
}

pub fn raw_problem_id(problem_id: &str) -> Cow<'_, str> {
    split_global_problem_id(problem_id)
        .map(|(_, raw_id)| Cow::Borrowed(raw_id))
        .unwrap_or_else(|| Cow::Borrowed(problem_id))
}

pub fn human_problem_id(problem_id: &str) -> String {
    raw_problem_id(problem_id).into_owned()
}

pub fn metadata_cache_file_name(problem_id: &str) -> String {
    let encoded = problem_id
        .chars()
        .map(|ch| match ch {
            ':' | '/' | '\\' => '_',
            _ => ch,
        })
        .collect::<String>();
    format!("{encoded}.toml")
}

fn luogu_problem_file_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)^((?:P|B|U)\d{3,6}[A-Z0-9]*)\.[^.]+$")
            .expect("luogu problem file regex should compile")
    })
}

fn luogu_problem_id_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)^(?:P|B|U)\d{3,6}[A-Z0-9]*$")
            .expect("luogu problem id regex should compile")
    })
}

fn atcoder_contest_task_file_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)^((?:abc|arc|agc|ahc)\d{3,4})_?([a-z0-9]+)\.[^.]+$")
            .expect("atcoder contest task file regex should compile")
    })
}

fn atcoder_task_file_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)^((?:typical90|dp|math_and_algorithm|practice|tessoku_book|past\d{4})_[a-z0-9]+)\.[^.]+$",
        )
        .expect("atcoder task file regex should compile")
    })
}

fn atcoder_task_id_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"(?i)^(?:(?:abc|arc|agc|ahc)\d{3,4}|typical90|dp|math_and_algorithm|practice|tessoku_book|past\d{4})_[a-z0-9]+$",
        )
        .expect("atcoder task id regex should compile")
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ProblemProvider, extract_atcoder_task_id, extract_luogu_problem_id, extract_problem_target,
        global_problem_id, human_problem_id, is_atcoder_task_id, is_luogu_problem_id,
        metadata_cache_file_name, normalize_problem_id_with_source, provider_from_problem_id,
        split_global_problem_id,
    };

    #[test]
    fn extracts_supported_problem_targets() {
        let luogu = extract_problem_target("P1000.cpp").unwrap();
        assert_eq!(luogu.provider, ProblemProvider::Luogu);
        assert_eq!(luogu.raw_id, "P1000");
        assert_eq!(luogu.global_id, "luogu:P1000");

        let atcoder = extract_problem_target("abc350_a.rs").unwrap();
        assert_eq!(atcoder.provider, ProblemProvider::AtCoder);
        assert_eq!(atcoder.raw_id, "abc350_a");
        assert_eq!(atcoder.global_id, "atcoder:abc350_a");
    }

    #[test]
    fn extracts_supported_provider_specific_ids() {
        assert_eq!(
            extract_luogu_problem_id("b2001.rs"),
            Some("B2001".to_string())
        );
        assert_eq!(
            extract_atcoder_task_id("Typical90_001.py"),
            Some("typical90_001".to_string())
        );
        assert_eq!(
            extract_atcoder_task_id("abc447a.cpp"),
            Some("abc447_a".to_string())
        );
        assert_eq!(
            extract_atcoder_task_id("abc447_a.cpp"),
            Some("abc447_a".to_string())
        );
    }

    #[test]
    fn rejects_non_problem_patterns() {
        assert_eq!(extract_problem_target("CF1234A.cpp"), None);
        assert_eq!(extract_problem_target("SP1.cpp"), None);
        assert_eq!(extract_problem_target("abc447.cpp"), None);
        assert_eq!(extract_problem_target("notes.txt"), None);
    }

    #[test]
    fn validates_stored_problem_ids_with_shared_rules() {
        assert!(is_luogu_problem_id("P1000"));
        assert!(is_atcoder_task_id("abc350_a"));
        assert!(!is_luogu_problem_id("CF1234A"));
        assert!(!is_atcoder_task_id("contest_notes"));
    }

    #[test]
    fn normalizes_legacy_problem_ids_using_source_hint() {
        assert_eq!(
            normalize_problem_id_with_source("P1001", Some("Luogu")),
            "luogu:P1001"
        );
        assert_eq!(
            normalize_problem_id_with_source("abc350_a", Some("AtCoder")),
            "atcoder:abc350_a"
        );
        assert_eq!(
            normalize_problem_id_with_source("custom-task", None),
            "unknown:custom-task"
        );
    }

    #[test]
    fn splits_and_formats_global_problem_ids() {
        assert_eq!(
            split_global_problem_id("luogu:P1001"),
            Some((ProblemProvider::Luogu, "P1001"))
        );
        assert_eq!(
            provider_from_problem_id("atcoder:abc350_a"),
            ProblemProvider::AtCoder
        );
        assert_eq!(human_problem_id("luogu:P1001"), "P1001");
        assert_eq!(
            global_problem_id(ProblemProvider::AtCoder, "ABC350_A"),
            "atcoder:abc350_a"
        );
        assert_eq!(metadata_cache_file_name("luogu:P1001"), "luogu_P1001.toml");
    }
}
