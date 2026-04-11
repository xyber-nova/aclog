use chrono::{DateTime, FixedOffset, Utc};
use color_eyre::{
    Result,
    eyre::{OptionExt, eyre},
};
use reqwest::{
    Client,
    header::{ACCEPT, ACCEPT_ENCODING, HeaderMap, HeaderValue},
};
use serde_json::Value;
use tracing::warn;

use crate::{
    config::AppConfig,
    domain::{problem::ProblemMetadata, submission::SubmissionRecord},
    problem::{ProblemProvider, global_problem_id, provider_label},
};

const RESOURCES_BASE_URL: &str = "https://kenkoooo.com/atcoder/resources";
const API_BASE_URL: &str = "https://kenkoooo.com/atcoder/atcoder-api/v3";
const USER_AGENT_VALUE: &str = "aclog/0.1 (+https://github.com/xyber-nova/aclog)";

pub struct AtCoderProblemsClient {
    client: Client,
}

impl AtCoderProblemsClient {
    pub fn new() -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip"));

        Ok(Self {
            client: Client::builder()
                // `kenkoooo.com` currently returns 403 for these dataset endpoints over HTTP/2
                // in some environments, while the same requests succeed over HTTP/1.1.
                .http1_only()
                .default_headers(headers)
                .user_agent(USER_AGENT_VALUE)
                .build()?,
        })
    }

    pub async fn fetch_problem_metadata(&self, task_id: &str) -> Result<Option<ProblemMetadata>> {
        let problems = self
            .get_json_array(format!("{RESOURCES_BASE_URL}/merged-problems.json"))
            .await?;
        let item = problems.into_iter().find(|value| {
            value
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|value| value.eq_ignore_ascii_case(task_id))
        });
        let Some(item) = item else {
            return Ok(None);
        };

        let contest_id = item
            .get("contest_id")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let contest_lookup = self.resolve_contest_title(contest_id.as_deref()).await;
        if let Err(error) = &contest_lookup {
            warn!(
                task_id,
                contest_id = contest_id.as_deref().unwrap_or("-"),
                error = %error,
                "AtCoder 比赛标题解析失败，回退到 contest id"
            );
        }
        let contest = choose_contest_context(contest_id.clone(), contest_lookup);
        let title = item
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or(task_id)
            .to_string();
        let url = match contest_id.as_deref() {
            Some(contest_id) => format!("https://atcoder.jp/contests/{contest_id}/tasks/{task_id}"),
            None => format!("https://atcoder.jp/tasks/{task_id}"),
        };

        Ok(Some(ProblemMetadata {
            id: global_problem_id(ProblemProvider::AtCoder, task_id),
            provider: ProblemProvider::AtCoder,
            title,
            difficulty: item
                .get("difficulty")
                .and_then(Value::as_i64)
                .map(|value| value.to_string()),
            tags: Vec::new(),
            source: Some(provider_label(ProblemProvider::AtCoder).to_string()),
            contest,
            url,
            fetched_at: now_in_provider_timezone(),
        }))
    }

    pub async fn fetch_problem_submissions(
        &self,
        config: &AppConfig,
        task_id: &str,
    ) -> Result<Vec<SubmissionRecord>> {
        let user_id = config
            .user
            .atcoder_user_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                eyre!("配置项 user.atcoder_user_id 不能为空，才能拉取 AtCoder 提交记录")
            })?;

        let mut from_second = 0i64;
        let mut all = Vec::new();
        loop {
            let url =
                format!("{API_BASE_URL}/user/submissions?user={user_id}&from_second={from_second}");
            let page = self.get_json_array(url).await?;
            let page_len = page.len();
            for item in &page {
                if item
                    .get("problem_id")
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.eq_ignore_ascii_case(task_id))
                {
                    all.push(parse_submission_record(item)?);
                }
            }
            if page_len < 500 {
                break;
            }
            let last_second = page
                .last()
                .and_then(|item| item.get("epoch_second"))
                .and_then(Value::as_i64)
                .unwrap_or(from_second);
            if last_second <= from_second {
                break;
            }
            from_second = last_second + 1;
        }
        Ok(all)
    }

    async fn resolve_contest_title(&self, contest_id: Option<&str>) -> Result<Option<String>> {
        let Some(contest_id) = contest_id else {
            return Ok(None);
        };
        let contests = self
            .get_json_array(format!("{RESOURCES_BASE_URL}/contests.json"))
            .await?;
        Ok(contests.into_iter().find_map(|value| {
            if value
                .get("id")
                .and_then(Value::as_str)
                .is_some_and(|value| value.eq_ignore_ascii_case(contest_id))
            {
                value
                    .get("title")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .or_else(|| Some(contest_id.to_string()))
            } else {
                None
            }
        }))
    }

    async fn get_json_array(&self, url: String) -> Result<Vec<Value>> {
        let body = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let value: Value = serde_json::from_str(&body)?;
        value
            .as_array()
            .cloned()
            .ok_or_eyre("AtCoder Problems 响应不是数组")
    }
}

fn parse_submission_record(value: &Value) -> Result<SubmissionRecord> {
    let epoch_second = value
        .get("epoch_second")
        .and_then(Value::as_i64)
        .ok_or_else(|| eyre!("AtCoder 提交记录缺少 epoch_second"))?;
    let submitted_at = DateTime::from_timestamp(epoch_second, 0)
        .map(|value| value.with_timezone(&FixedOffset::east_opt(9 * 3600).expect("JST 应有效")));
    let verdict = value
        .get("result")
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN")
        .to_string();

    Ok(SubmissionRecord {
        submission_id: value
            .get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| eyre!("AtCoder 提交记录缺少 id"))?,
        problem_id: value
            .get("problem_id")
            .and_then(Value::as_str)
            .map(|task_id| global_problem_id(ProblemProvider::AtCoder, task_id)),
        provider: ProblemProvider::AtCoder,
        submitter: value
            .get("user_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        verdict,
        score: value.get("score").and_then(Value::as_i64),
        time_ms: value.get("execution_time").and_then(Value::as_u64),
        memory_mb: value
            .get("memory")
            .and_then(Value::as_u64)
            .map(|value| value as f64 / 1024.0),
        submitted_at,
    })
}

fn now_in_provider_timezone() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(9 * 3600).expect("JST 应有效"))
}

fn choose_contest_context(
    contest_id: Option<String>,
    contest_lookup: Result<Option<String>>,
) -> Option<String> {
    match contest_lookup {
        Ok(Some(title)) if !title.trim().is_empty() => Some(title),
        Ok(_) | Err(_) => contest_id,
    }
}

#[cfg(test)]
mod tests {
    use color_eyre::eyre::eyre;

    use super::choose_contest_context;

    #[test]
    fn contest_lookup_errors_fall_back_to_contest_id() {
        assert_eq!(
            choose_contest_context(
                Some("abc350".to_string()),
                Err(eyre!("contest lookup failed")),
            ),
            Some("abc350".to_string())
        );
        assert_eq!(
            choose_contest_context(Some("abc350".to_string()), Ok(None)),
            Some("abc350".to_string())
        );
        assert_eq!(
            choose_contest_context(
                Some("abc350".to_string()),
                Ok(Some("AtCoder Beginner Contest 350".to_string())),
            ),
            Some("AtCoder Beginner Contest 350".to_string())
        );
    }
}
