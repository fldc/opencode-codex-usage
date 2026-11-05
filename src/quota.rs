use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub used_percent: f64,
    pub window_duration_mins: Option<i64>,
    pub resets_at: Option<i64>,
}
impl Window {
    pub fn label(&self, fallback: &str) -> String {
        match self.window_duration_mins {
            Some(m) if m > 0 && m % 1440 == 0 => format!("{}D", m / 1440),
            Some(m) if m > 0 && m % 60 == 0 => format!("{}H", m / 60),
            Some(m) if m > 0 => format!("{m}M"),
            _ => fallback.to_uppercase(),
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub primary: Option<Window>,
    pub secondary: Option<Window>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    rate_limits: Option<Bucket>,
    rate_limits_by_limit_id: Option<BTreeMap<String, Bucket>>,
}
pub fn parse(value: serde_json::Value) -> Result<BTreeMap<String, Bucket>, &'static str> {
    let r: Response = serde_json::from_value(value).map_err(|_| "INVALID DATA")?;
    let mut buckets = r.rate_limits_by_limit_id.unwrap_or_default();
    if buckets.is_empty() {
        if let Some(b) = r.rate_limits {
            buckets.insert("codex".into(), b);
        }
    }
    if buckets.is_empty() {
        return Err("NO QUOTA");
    }
    for b in buckets.values() {
        for w in [&b.primary, &b.secondary].into_iter().flatten() {
            if !w.used_percent.is_finite() || w.used_percent < 0.0 {
                return Err("INVALID DATA");
            }
        }
    }
    Ok(buckets)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn nullable_windows_and_legacy_fallback() {
        let b = parse(json!({"rateLimits":{"primary":{"usedPercent":12,"windowDurationMins":300,"resetsAt":100},"secondary":null}})).unwrap();
        assert_eq!(b["codex"].primary.as_ref().unwrap().label("primary"), "5H");
        assert!(b["codex"].secondary.is_none());
        assert!(parse(json!({"rateLimits":null})).is_err());
    }
    #[test]
    fn multiple_buckets_preserved() {
        let b = parse(json!({"rateLimitsByLimitId":{"codex":{"primary":null},"other":{"secondary":{"usedPercent":26,"windowDurationMins":10080}}}})).unwrap();
        assert_eq!(b.len(), 2);
        assert_eq!(
            b["other"].secondary.as_ref().unwrap().label("secondary"),
            "7D"
        );
    }
    #[test]
    fn rejects_invalid_percentage() {
        assert!(parse(json!({"rateLimits":{"primary":{"usedPercent":-1}}})).is_err());
    }
}
