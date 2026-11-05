use base64::Engine;
use openaction::*;
use opendeck_codex_usage::{
    provider::{now, Provider, Snapshot},
    render,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::LazyLock, time::Duration};
use tokio::sync::{Mutex, Notify};

static PROVIDER: LazyLock<Mutex<Provider>> = LazyLock::new(|| Mutex::new(Provider::default()));
static SETTINGS: LazyLock<Mutex<HashMap<String, Settings>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static SNAPSHOT: LazyLock<Mutex<Snapshot>> = LazyLock::new(|| Mutex::new(Snapshot::default()));
static REFRESH: Notify = Notify::const_new();
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    bucket: String,
    window: String,
    show_percent: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            bucket: "codex".into(),
            window: "primary".into(),
            show_percent: true,
        }
    }
}
async fn paint(
    instance: &Instance,
    settings: &Settings,
    snapshot: &Snapshot,
) -> OpenActionResult<()> {
    let timestamp = now();
    let window =
        snapshot
            .buckets
            .get(&settings.bucket)
            .and_then(|b| match settings.window.as_str() {
                "primary" => b.primary.as_ref(),
                "secondary" => b.secondary.as_ref(),
                _ => None,
            });
    let stale = snapshot.error.is_some()
        || snapshot.fetched_at.is_some_and(|t| timestamp - t > 120)
        || window.is_some_and(|w| w.resets_at.is_some_and(|r| r <= timestamp));
    let svg = match window {
        Some(w) => render::usage(w, &settings.window, timestamp, settings.show_percent, stale),
        None => render::status(snapshot.error.unwrap_or(if snapshot.fetched_at.is_some() {
            "N/A"
        } else {
            "LOADING"
        })),
    };
    instance
        .set_image(
            Some(format!(
                "data:image/svg+xml;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(svg)
            )),
            None,
        )
        .await?;
    let status = snapshot.error.unwrap_or(if snapshot.fetched_at.is_none() {
        "Loading"
    } else if window.is_none() {
        "Window unavailable"
    } else if stale {
        "Waiting for fresh quota"
    } else {
        "Connected"
    });
    instance.send_to_property_inspector(json!({"status":status,"fetchedAt":snapshot.fetched_at,"stale":stale,"buckets":snapshot.buckets})).await
}
struct Usage;
#[async_trait]
impl Action for Usage {
    const UUID: ActionUuid = "se.fredrik.codexusage.usage";
    type Settings = Settings;
    async fn will_appear(&self, i: &Instance, s: &Settings) -> OpenActionResult<()> {
        SETTINGS
            .lock()
            .await
            .insert(i.instance_id.clone(), s.clone());
        paint(i, s, &{ SNAPSHOT.lock().await.clone() }).await?;
        REFRESH.notify_one();
        Ok(())
    }
    async fn will_disappear(&self, i: &Instance, _: &Settings) -> OpenActionResult<()> {
        SETTINGS.lock().await.remove(&i.instance_id);
        Ok(())
    }
    async fn did_receive_settings(&self, i: &Instance, s: &Settings) -> OpenActionResult<()> {
        SETTINGS
            .lock()
            .await
            .insert(i.instance_id.clone(), s.clone());
        paint(i, s, &{ SNAPSHOT.lock().await.clone() }).await
    }
    async fn key_up(&self, _: &Instance, _: &Settings) -> OpenActionResult<()> {
        REFRESH.notify_one();
        Ok(())
    }
    async fn property_inspector_did_appear(
        &self,
        i: &Instance,
        s: &Settings,
    ) -> OpenActionResult<()> {
        paint(i, s, &{ SNAPSHOT.lock().await.clone() }).await
    }
    async fn send_to_plugin(
        &self,
        i: &Instance,
        s: &Settings,
        _: &serde_json::Value,
    ) -> OpenActionResult<()> {
        paint(i, s, &{ SNAPSHOT.lock().await.clone() }).await
    }
}
#[tokio::main]
async fn main() -> OpenActionResult<()> {
    if std::env::args().any(|a| a == "--probe") {
        let mut p = Provider::default();
        let snapshot = p.refresh(true).await;
        p.close().await;
        if let Some(error) = snapshot.error {
            eprintln!("{error}");
            std::process::exit(1);
        }
        println!(
            "{}",
            serde_json::to_string_pretty(
                &json!({"fetchedAt":snapshot.fetched_at,"buckets":snapshot.buckets})
            )?
        );
        return Ok(());
    }
    let _ = simplelog::SimpleLogger::init(log::LevelFilter::Warn, simplelog::Config::default());
    register_action(Usage).await;
    let worker = tokio::spawn(async {
        let mut tick = tokio::time::interval(Duration::from_secs(5));
        loop {
            let force = tokio::select! {_=tick.tick()=>false,_=REFRESH.notified()=>true};
            let instances = visible_instances(Usage::UUID).await;
            if instances.is_empty() {
                continue;
            }
            let snapshot = PROVIDER.lock().await.refresh(force).await;
            *SNAPSHOT.lock().await = snapshot.clone();
            for instance in instances {
                let settings = SETTINGS.lock().await.get(&instance.instance_id).cloned();
                if let Some(settings) = settings {
                    if let Err(e) = paint(&instance, &settings, &snapshot).await {
                        log::warn!("Cannot update key: {e}");
                    }
                }
            }
        }
    });
    let result = run(std::env::args().collect()).await;
    worker.abort();
    let _ = worker.await;
    PROVIDER.lock().await.close().await;
    result
}
