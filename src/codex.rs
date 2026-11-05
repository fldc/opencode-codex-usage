use serde_json::{json, Value};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};

pub struct Client {
    child: Child,
    input: ChildStdin,
    output: Lines<BufReader<ChildStdout>>,
    next_id: u64,
}
impl Client {
    pub async fn start() -> Result<Self, &'static str> {
        let executable = std::env::var_os("CODEX_USAGE_CLI").unwrap_or_else(|| "codex".into());
        let mut child = Command::new(executable)
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    "NO CLI"
                } else {
                    "CLI ERROR"
                }
            })?;
        let mut c = Self {
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()).lines(),
            child,
            next_id: 0,
        };
        c.rpc("initialize", json!({"clientInfo":{"name":"opendeck_codex_usage","version":env!("CARGO_PKG_VERSION")}})).await?;
        c.send(json!({"method":"initialized"})).await?;
        Ok(c)
    }
    async fn send(&mut self, value: Value) -> Result<(), &'static str> {
        let mut data = serde_json::to_vec(&value).map_err(|_| "PROTOCOL")?;
        data.push(b'\n');
        self.input
            .write_all(&data)
            .await
            .map_err(|_| "DISCONNECTED")
    }
    pub async fn rpc(&mut self, method: &str, params: Value) -> Result<Value, &'static str> {
        self.next_id += 1;
        let id = self.next_id;
        timeout(Duration::from_secs(20), async {
            self.send(json!({"id":id,"method":method,"params":params})).await?;
            loop {
                let line = self.output.next_line().await.map_err(|_| "DISCONNECTED")?.ok_or("DISCONNECTED")?;
                if line.len() > 1_048_576 { return Err("PROTOCOL"); }
                let v: Value = serde_json::from_str(&line).map_err(|_| "PROTOCOL")?;
                if v.get("id")==Some(&json!(id)) {
                    if v.get("error").is_some() { return Err("QUOTA ERROR"); }
                    return v.get("result").cloned().ok_or("PROTOCOL");
                }
                // No turns are started. Reject unexpected server requests rather than granting approval.
                if v.get("method").is_some() && v.get("id").is_some() {
                    self.send(json!({"id":v["id"],"error":{"code":-32601,"message":"Unsupported request"}})).await?;
                }
                // Notifications are drained here; periodic reads remain the source of truth.
            }
        }).await.map_err(|_| "TIMEOUT")?
    }
    pub async fn close(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}
