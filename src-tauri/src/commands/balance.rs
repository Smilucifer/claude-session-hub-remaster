use crate::models::{BalanceCacheEntry, BalanceHelperSettings};
use crate::storage;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, USER_AGENT};
use serde_json::Value;
use std::time::Duration;

const DEEPSEEK_BALANCE_BASE_URL: &str = "https://api.deepseek.com";
const PACKY_API_BASE_URL: &str = "https://www.packyapi.com";
const MIMO_API_BASE_URL: &str = "https://platform.xiaomimimo.com";
const PACKY_QUOTA_PER_UNIT: f64 = 500_000.0;
const PACKY_DISPLAY_CURRENCY: &str = "USD";

fn balance_cache_entry(source: &str, result: Result<String, String>) -> BalanceCacheEntry {
    match result {
        Ok(balance_text) => BalanceCacheEntry {
            source: source.to_string(),
            status: "ok".to_string(),
            balance_text: Some(balance_text),
            error: None,
            refreshed_at: crate::models::now_iso(),
        },
        Err(error) => BalanceCacheEntry {
            source: source.to_string(),
            status: "failed".to_string(),
            balance_text: None,
            error: Some(redacted_operational_error(&error)),
            refreshed_at: crate::models::now_iso(),
        },
    }
}

fn redacted_operational_error(input: &str) -> String {
    input
        .split_whitespace()
        .map(|part| {
            let lower = part.to_ascii_lowercase();
            if lower.starts_with("sk-")
                || lower.contains("api_key")
                || lower.contains("apikey")
                || lower.contains("authorization")
                || lower.contains("cookie")
                || lower.contains("session")
                || lower.contains("token=")
            {
                "[redacted]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_deepseek_balance(body: &Value) -> Result<String, String> {
    if body
        .get("is_available")
        .and_then(Value::as_bool)
        .is_some_and(|available| !available)
    {
        return Err("DeepSeek balance is unavailable".to_string());
    }

    let infos = body
        .get("balance_infos")
        .and_then(Value::as_array)
        .ok_or_else(|| "DeepSeek response did not include balance info".to_string())?;

    let formatted = infos
        .iter()
        .filter_map(|info| {
            let currency = info.get("currency")?.as_str()?.trim();
            let balance = info
                .get("total_balance")
                .or_else(|| info.get("granted_balance"))
                .or_else(|| info.get("topped_up_balance"))?
                .as_str()?
                .trim();
            if currency.is_empty() || balance.is_empty() {
                None
            } else {
                Some(format!("{currency} {balance}"))
            }
        })
        .collect::<Vec<_>>();

    if formatted.is_empty() {
        Err("DeepSeek response did not include a readable balance".to_string())
    } else {
        Ok(formatted.join(", "))
    }
}

async fn query_deepseek_balance(
    client: &reqwest::Client,
    api_key: &str,
    base_url: &str,
) -> Result<String, String> {
    let trimmed_key = api_key.trim();
    if trimmed_key.is_empty() {
        return Err("DeepSeek API key is not configured".to_string());
    }

    let url = format!("{}/user/balance", base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .bearer_auth(trimmed_key)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "DeepSeek balance request timed out".to_string()
            } else {
                format!("DeepSeek balance request failed: {e}")
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => "DeepSeek authentication failed".to_string(),
            429 => "DeepSeek balance request was rate limited".to_string(),
            code => format!("DeepSeek balance request failed with HTTP {code}"),
        });
    }

    let body = response
        .json::<Value>()
        .await
        .map_err(|_| "DeepSeek balance response was not valid JSON".to_string())?;
    format_deepseek_balance(&body)
}

fn format_packy_balance(body: &Value) -> Result<String, String> {
    let data = body
        .get("data")
        .ok_or_else(|| "Packy user response did not include data".to_string())?;

    let quota = data
        .get("quota")
        .and_then(Value::as_i64)
        .or_else(|| data.get("quota").and_then(Value::as_u64).map(|v| v as i64))
        .ok_or_else(|| "Packy user response did not include quota".to_string())?;

    let amount = quota as f64 / PACKY_QUOTA_PER_UNIT;
    Ok(format!("{} {:.2}", PACKY_DISPLAY_CURRENCY, amount))
}

fn build_packy_headers(
    session: &str,
    tdc_itoken: &str,
    user_id: &str,
) -> Result<HeaderMap, String> {
    let trimmed_session = session.trim();
    let trimmed_itoken = tdc_itoken.trim();
    let trimmed_user_id = user_id.trim();

    if trimmed_session.is_empty() {
        return Err("Packy session is not configured".to_string());
    }
    if trimmed_itoken.is_empty() {
        return Err("Packy TDC_itoken is not configured".to_string());
    }
    if trimmed_user_id.is_empty() {
        return Err("Packy user id is not configured".to_string());
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!(
            "session={}; TDC_itoken={}",
            trimmed_session, trimmed_itoken
        ))
        .map_err(|_| "Packy credentials contain invalid header characters".to_string())?,
    );
    headers.insert(
        "New-API-User",
        HeaderValue::from_str(trimmed_user_id)
            .map_err(|_| "Packy user id contains invalid header characters".to_string())?,
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
        ),
    );
    headers.insert("Accept", HeaderValue::from_static("application/json, text/plain, */*"));
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://www.packyapi.com/console"),
    );
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://www.packyapi.com"),
    );
    Ok(headers)
}

async fn query_packy_balance(
    client: &reqwest::Client,
    session: &str,
    tdc_itoken: &str,
    user_id: &str,
) -> Result<String, String> {
    let headers = build_packy_headers(session, tdc_itoken, user_id)?;
    let response = client
        .get(format!("{}/api/user/self", PACKY_API_BASE_URL))
        .headers(headers)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Packy balance request timed out".to_string()
            } else {
                format!("Packy balance request failed: {e}")
            }
        })?;

    let status = response.status();
    let body = response
        .json::<Value>()
        .await
        .map_err(|_| "Packy balance response was not valid JSON".to_string())?;

    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => body
                .get("message")
                .and_then(Value::as_str)
                .map(|s| format!("Packy authentication failed: {s}"))
                .unwrap_or_else(|| "Packy authentication failed".to_string()),
            code => format!("Packy balance request failed with HTTP {code}"),
        });
    }

    if body.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(body
            .get("message")
            .and_then(Value::as_str)
            .map(|s| format!("Packy balance request failed: {s}"))
            .unwrap_or_else(|| "Packy balance request failed".to_string()));
    }

    format_packy_balance(&body)
}

fn build_mimo_headers(
    service_token: &str,
    user_id: &str,
    slh: &str,
    ph: &str,
) -> Result<HeaderMap, String> {
    let trimmed_token = service_token.trim();
    let trimmed_user_id = user_id.trim();

    if trimmed_token.is_empty() {
        return Err("MiMo service token is not configured".to_string());
    }
    if trimmed_user_id.is_empty() {
        return Err("MiMo user id is not configured".to_string());
    }

    let cookie_value = format!(
        "api-platform_serviceToken={}; userId={}; api-platform_slh={}; api-platform_ph={}",
        trimmed_token, trimmed_user_id, slh.trim(), ph.trim()
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&cookie_value)
            .map_err(|_| "MiMo credentials contain invalid header characters".to_string())?,
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
        ),
    );
    headers.insert("Accept", HeaderValue::from_static("application/json, text/plain, */*"));
    headers.insert(
        "Referer",
        HeaderValue::from_static("https://platform.xiaomimimo.com/"),
    );
    headers.insert(
        "Origin",
        HeaderValue::from_static("https://platform.xiaomimimo.com"),
    );
    headers.insert(
        "x-kl-ajax-request",
        HeaderValue::from_static("Ajax_Request"),
    );
    headers.insert(
        "x-timezone",
        HeaderValue::from_static("Asia/Shanghai"),
    );
    Ok(headers)
}

fn format_mimo_balance(balance_body: &Value, usage_body: &Value) -> Result<String, String> {
    let balance_code = balance_body
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or(-1);
    if balance_code != 0 {
        return Err(get_mimo_error(&balance_body, "balance"));
    }

    let data = balance_body
        .get("data")
        .ok_or_else(|| "MiMo balance response did not include data".to_string())?;
    let balance = data
        .get("balance")
        .and_then(Value::as_str)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("0");
    let currency = data
        .get("currency")
        .and_then(Value::as_str)
        .map(|s| s.trim())
        .unwrap_or("CNY");

    let mut parts = vec![format!("{} {}", currency, balance)];

    let usage_code = usage_body
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or(-1);
    if usage_code == 0 {
        if let Some(usage_items) = usage_body
            .get("data")
            .and_then(|d| d.get("usage"))
            .and_then(|u| u.get("items"))
            .and_then(Value::as_array)
        {
            if let Some(plan_item) = usage_items
                .iter()
                .find(|item| item.get("name").and_then(Value::as_str) == Some("plan_total_token"))
            {
                let used = plan_item.get("used").and_then(Value::as_i64).unwrap_or(0);
                let limit = plan_item.get("limit").and_then(Value::as_i64).unwrap_or(0);
                if limit > 0 {
                    let used_str = format_token_compact(used as u64);
                    let limit_str = format_token_compact(limit as u64);
                    parts.push(format!("套餐 {}/{}", used_str, limit_str));
                }
            }
        }
    }

    if parts.len() == 1 && parts[0] == "CNY 0" {
        return Err("MiMo balance response was empty".to_string());
    }

    Ok(parts.join(" | "))
}

fn get_mimo_error(body: &Value, api_name: &str) -> String {
    body.get("msg")
        .or_else(|| body.get("message"))
        .and_then(Value::as_str)
        .map(|s| format!("MiMo {} request failed: {}", api_name, s))
        .unwrap_or_else(|| format!("MiMo {} request returned non-zero code", api_name))
}

fn format_token_compact(n: u64) -> String {
    if n >= 100_000_000 {
        format!("{}亿", n / 100_000_000)
    } else if n >= 10_000 {
        format!("{}万", n / 10_000)
    } else {
        n.to_string()
    }
}

async fn query_mimo_balance(
    client: &reqwest::Client,
    service_token: &str,
    user_id: &str,
    slh: &str,
    ph: &str,
) -> Result<String, String> {
    let headers = build_mimo_headers(service_token, user_id, slh, ph)?;

    let balance_url = format!("{}/api/v1/balance", MIMO_API_BASE_URL);
    let balance_resp = client
        .get(&balance_url)
        .headers(headers.clone())
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "MiMo balance request timed out".to_string()
            } else {
                format!("MiMo balance request failed: {e}")
            }
        })?;

    let balance_status = balance_resp.status();
    if !balance_status.is_success() {
        return Err(match balance_status.as_u16() {
            401 | 403 => "MiMo authentication failed".to_string(),
            code => format!("MiMo balance request failed with HTTP {code}"),
        });
    }

    let balance_body = balance_resp
        .json::<Value>()
        .await
        .map_err(|_| "MiMo balance response was not valid JSON".to_string())?;

    let usage_url = format!("{}/api/v1/tokenPlan/usage", MIMO_API_BASE_URL);
    let usage_body = match client
        .get(&usage_url)
        .headers(headers)
        .send()
        .await
    {
        Ok(resp) => resp
            .json::<Value>()
            .await
            .unwrap_or_else(|_| serde_json::json!({"code": -1})),
        Err(_) => serde_json::json!({"code": -1}),
    };

    format_mimo_balance(&balance_body, &usage_body)
}

async fn refresh_balance_status_inner(
    source: Option<String>,
) -> Result<BalanceHelperSettings, String> {
    let requested = source.unwrap_or_else(|| "all".to_string());
    if !matches!(requested.as_str(), "all" | "deepseek" | "packy" | "mimo") {
        return Err(format!("Unknown balance source: {requested}"));
    }

    let settings = storage::settings::get_user_settings();
    let mut helper = settings.balance_helper.clone();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Balance HTTP client build failed: {e}"))?;

    if requested == "all" || requested == "deepseek" {
        let deepseek_key = settings
            .platform_credentials
            .iter()
            .find(|credential| credential.platform_id == "deepseek")
            .and_then(|credential| credential.api_key.as_deref())
            .unwrap_or("");
        let result = query_deepseek_balance(&client, deepseek_key, DEEPSEEK_BALANCE_BASE_URL).await;
        helper.cache.insert(
            "deepseek".to_string(),
            balance_cache_entry("deepseek", result),
        );
    }

    if requested == "all" || requested == "packy" {
        let result = query_packy_balance(
            &client,
            helper.packy_session.as_deref().unwrap_or(""),
            helper.packy_tdc_itoken.as_deref().unwrap_or(""),
            helper.packy_user_id.as_deref().unwrap_or(""),
        )
        .await;
        helper
            .cache
            .insert("packy".to_string(), balance_cache_entry("packy", result));
    }

    if requested == "all" || requested == "mimo" {
        let result = query_mimo_balance(
            &client,
            helper.mimo_service_token.as_deref().unwrap_or(""),
            helper.mimo_user_id.as_deref().unwrap_or(""),
            helper.mimo_slh.as_deref().unwrap_or(""),
            helper.mimo_ph.as_deref().unwrap_or(""),
        )
        .await;
        helper
            .cache
            .insert("mimo".to_string(), balance_cache_entry("mimo", result));
    }

    let updated = storage::settings::update_user_settings(serde_json::json!({
        "balance_helper": helper
    }))?;
    Ok(updated.balance_helper)
}

#[tauri::command]
pub async fn refresh_balance_status(
    source: Option<String>,
) -> Result<BalanceHelperSettings, String> {
    refresh_balance_status_inner(source).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_deepseek_balance_infos() {
        let body = serde_json::json!({
            "is_available": true,
            "balance_infos": [
                {"currency": "CNY", "total_balance": "110.00"},
                {"currency": "USD", "total_balance": "2.50"}
            ]
        });

        assert_eq!(
            format_deepseek_balance(&body).unwrap(),
            "CNY 110.00, USD 2.50"
        );
    }

    #[test]
    fn formats_packy_balance_from_quota() {
        let body = serde_json::json!({
            "success": true,
            "data": {
                "quota": 87_304_703
            }
        });

        assert_eq!(format_packy_balance(&body).unwrap(), "USD 174.61");
    }

    #[test]
    fn rejects_packy_headers_when_required_values_missing() {
        let err = build_packy_headers("", "595383047:1776349439", "98264").unwrap_err();
        assert_eq!(err, "Packy session is not configured");

        let err = build_packy_headers("session", "", "98264").unwrap_err();
        assert_eq!(err, "Packy TDC_itoken is not configured");

        let err = build_packy_headers("session", "595383047:1776349439", "").unwrap_err();
        assert_eq!(err, "Packy user id is not configured");
    }

    #[test]
    fn formats_mimo_balance_with_usage() {
        let balance_body = serde_json::json!({
            "code": 0,
            "data": { "balance": "4.80", "frozenBalance": "0.00", "currency": "CNY", "giftBalance": "4.80", "cashBalance": "0.00" }
        });
        let usage_body = serde_json::json!({
            "code": 0,
            "data": { "usage": { "items": [{ "name": "plan_total_token", "used": 10136576, "limit": 700000000, "percent": 0.01 }] } }
        });

        let result = format_mimo_balance(&balance_body, &usage_body).unwrap();
        assert!(result.contains("CNY 4.80"));
        assert!(result.contains("套餐 1013万/7亿"));
    }

    #[test]
    fn formats_mimo_balance_without_usage() {
        let balance_body = serde_json::json!({
            "code": 0,
            "data": { "balance": "10.00", "currency": "CNY" }
        });
        let usage_body = serde_json::json!({
            "code": 0,
            "data": { "usage": { "items": [] } }
        });

        let result = format_mimo_balance(&balance_body, &usage_body).unwrap();
        assert_eq!(result, "CNY 10.00");
    }

    #[test]
    fn rejects_mimo_balance_with_nonzero_code() {
        let balance_body = serde_json::json!({
            "code": 1,
            "msg": "authentication failed"
        });
        let usage_body = serde_json::json!({});

        let err = format_mimo_balance(&balance_body, &usage_body).unwrap_err();
        assert!(err.contains("authentication failed"));
    }

    #[test]
    fn rejects_mimo_headers_when_required_values_missing() {
        let err = build_mimo_headers("", "2413232036", "slh", "ph").unwrap_err();
        assert!(err.contains("service token"));

        let err = build_mimo_headers("jwt-token", "", "slh", "ph").unwrap_err();
        assert!(err.contains("user id"));
    }

    #[test]
    fn format_token_compact_works() {
        assert_eq!(format_token_compact(0), "0");
        assert_eq!(format_token_compact(500), "500");
        assert_eq!(format_token_compact(10_000), "1万");
        assert_eq!(format_token_compact(10_136_576), "1013万");
        assert_eq!(format_token_compact(700_000_000), "7亿");
        assert_eq!(format_token_compact(100_000_000), "1亿");
        assert_eq!(format_token_compact(150_000_000), "1亿");
    }

    #[test]
    fn redacts_sensitive_values_from_errors() {
        let err = redacted_operational_error("HTTP 401 sk-live cookie=session_token=abc");

        assert!(!err.contains("sk-live"));
        assert!(!err.contains("session_token=abc"));
        assert!(err.contains("[redacted]"));
    }
}
