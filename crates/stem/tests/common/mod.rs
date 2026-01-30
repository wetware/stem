//! Common helpers for integration tests.

use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

/// Spawn Anvil on a dynamic port and wait until ready.
pub async fn spawn_anvil() -> Result<(Child, String)> {
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .context("bind for port")?;
        listener.local_addr()?.port()
    };
    let rpc_url = format!("http://127.0.0.1:{}", port);
    let mut cmd = Command::new("anvil");
    cmd.arg("--port").arg(port.to_string()).arg("--host").arg("127.0.0.1");
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let process = cmd.spawn().context("spawn anvil")?;
    wait_for_rpc(&rpc_url).await?;
    Ok((process, rpc_url))
}

async fn wait_for_rpc(url: &str) -> Result<()> {
    let client = reqwest::Client::new();
    for _ in 0..30 {
        let ok = client
            .post(url)
            .json(&serde_json::json!({"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}))
            .send()
            .await
            .is_ok();
        if ok {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }
    anyhow::bail!("RPC not ready");
}

/// Deploy Stem contract. Run from repo root (where src/Stem.sol lives).
pub fn deploy_stem(repo_root: &std::path::Path, rpc_url: &str) -> Result<String> {
    let out = Command::new("forge")
        .current_dir(repo_root)
        .args([
            "create",
            "--rpc-url", rpc_url,
            "--private-key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
            "src/Stem.sol:Stem",
            "--constructor-args", "0", "0x697066732d696e697469616c",
        ])
        .output()
        .context("forge create")?;
    if !out.status.success() {
        anyhow::bail!("forge create failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        if line.contains("Deployed to:") {
            let addr = line.split_whitespace().last().unwrap_or("").trim();
            if addr.starts_with("0x") {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("could not parse deployed address from: {}", s);
}

/// Call setHead(hint, cid) via cast send.
pub fn set_head(repo_root: &std::path::Path, rpc_url: &str, contract: &str, hint: u8, cid_hex: &str) -> Result<()> {
    let out = Command::new("cast")
        .current_dir(repo_root)
        .args([
            "send", contract,
            "setHead(uint8,bytes)",
            &hint.to_string(), cid_hex,
            "--rpc-url", rpc_url,
            "--private-key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        ])
        .output()
        .context("cast send")?;
    if !out.status.success() {
        anyhow::bail!("cast send failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(())
}
