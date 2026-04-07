use color_eyre::Result;
use color_eyre::eyre::WrapErr;
use reqwest::{
    Client,
    header::{COOKIE, HeaderMap, HeaderValue, USER_AGENT},
};
use serde_json::Value;
use tracing::debug;

pub fn build_http_client(cookie: &str, user_agent: &str) -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_str(user_agent)?);
    headers.insert(
        COOKIE,
        HeaderValue::from_str(cookie).wrap_err("luogu_cookie 请求头无效")?,
    );
    Ok(Client::builder().default_headers(headers).build()?)
}

pub async fn get_problem_body(client: &Client, base_url: &str, problem_id: &str) -> Result<String> {
    let url = format!("{base_url}/problem/{problem_id}");
    debug!(%url, "请求题目元数据");
    client
        .get(url)
        .header("x-lentille-request", "content-only")
        .query(&[("_contentOnly", "1")])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
        .wrap_err("读取题目元数据响应失败")
}

pub async fn get_record_list_body(
    client: &Client,
    base_url: &str,
    uid: &str,
    problem_id: &str,
) -> Result<String> {
    let url = format!("{base_url}/record/list");
    debug!(%url, uid, "请求题目提交记录");
    client
        .get(url)
        .header("x-lentille-request", "content-only")
        .query(&[("user", uid), ("pid", problem_id), ("_contentOnly", "1")])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
        .wrap_err("读取提交记录响应失败")
}

pub async fn fetch_tags_value(client: &Client, base_url: &str) -> Result<Value> {
    let url = format!("{base_url}/_lfe/tags");
    debug!(%url, "请求标签字典");
    client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .wrap_err("解析标签字典失败")
}

pub async fn fetch_shared_mappings_body(client: &Client, base_url: &str) -> Result<String> {
    let url = format!("{base_url}/_lfe/config");
    debug!(%url, "请求洛谷共享映射表");
    client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
        .wrap_err("读取洛谷共享映射响应失败")
}
