use crate::{
    codex::Client,
    quota::{self, Bucket},
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub buckets: BTreeMap<String, Bucket>,
    pub fetched_at: Option<i64>,
    pub error: Option<&'static str>,
}
#[derive(Default)]
pub struct Provider {
    client: Option<Client>,
    account: Option<Value>,
    pub snapshot: Snapshot,
    attempted: Option<Instant>,
    failures: u32,
}
impl Provider {
    pub async fn refresh(&mut self, force: bool) -> Snapshot {
        let retry = if self.failures == 0 {
            60
        } else {
            (5u64 * 2u64.pow(self.failures.min(4))).min(60)
        };
        let reset_due = self.snapshot.fetched_at.is_some_and(|fetched| {
            self.snapshot
                .buckets
                .values()
                .flat_map(|b| [&b.primary, &b.secondary])
                .flatten()
                .any(|w| w.resets_at.is_some_and(|r| fetched < r && r <= now()))
        });
        let interval = if force || reset_due { 2 } else { retry };
        if self
            .attempted
            .is_some_and(|t| t.elapsed() < Duration::from_secs(interval))
        {
            return self.snapshot.clone();
        }
        self.attempted = Some(Instant::now());
        match self.fetch().await {
            Ok(buckets) => {
                self.snapshot = Snapshot {
                    buckets,
                    fetched_at: Some(now()),
                    error: None,
                };
                self.failures = 0;
            }
            Err(error) => {
                self.failures = self.failures.saturating_add(1);
                if matches!(error, "SIGN IN" | "AUTH MODE") {
                    self.snapshot = Snapshot::default();
                }
                self.snapshot.error = Some(error);
                if let Some(mut client) = self.client.take() {
                    client.close().await;
                }
            }
        }
        self.snapshot.clone()
    }
    async fn fetch(&mut self) -> Result<BTreeMap<String, Bucket>, &'static str> {
        if self.client.is_none() {
            self.client = Some(Client::start().await?);
        }
        let c = self.client.as_mut().unwrap();
        let result = c.rpc("account/read", json!({"refreshToken":false})).await?;
        let account = result
            .get("account")
            .filter(|a| !a.is_null())
            .ok_or("SIGN IN")?;
        if self.account.as_ref() != Some(account) {
            self.snapshot = Snapshot::default();
            self.account = Some(account.clone());
        }
        if account["type"] == "apiKey" {
            return Err("AUTH MODE");
        }
        quota::parse(c.rpc("account/rateLimits/read", json!({})).await?)
    }
    pub async fn close(&mut self) {
        if let Some(mut c) = self.client.take() {
            c.close().await;
        }
    }
}
