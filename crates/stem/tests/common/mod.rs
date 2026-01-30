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

/// Deploy Stem contract via forge script. Run from repo root (where src/Stem.sol lives).
/// Parses the deployed address from the broadcast artifact (works across Foundry versions).
pub fn deploy_stem(repo_root: &std::path::Path, rpc_url: &str) -> Result<String> {
    let out = Command::new("forge")
        .current_dir(repo_root)
        .args([
            "script",
            "script/Deploy.s.sol:Deploy",
            "--rpc-url", rpc_url,
            "--broadcast",
            "--private-key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        ])
        .output()
        .context("forge script")?;
    if !out.status.success() {
        anyhow::bail!(
            "forge script failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    // Anvil chain id is 31337
    let artifact_path = repo_root.join("broadcast/Deploy.s.sol/31337/run-latest.json");
    let bytes = std::fs::read(&artifact_path)
        .with_context(|| format!("read broadcast artifact: {}", artifact_path.display()))?;
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).context("parse broadcast JSON")?;
    let txs = json
        .get("transactions")
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow::anyhow!("broadcast: missing transactions array"))?;
    for tx in txs {
        if tx.get("transactionType").and_then(|t| t.as_str()) == Some("CREATE") {
            let addr = tx
                .get("contractAddress")
                .and_then(|a| a.as_str())
                .ok_or_else(|| anyhow::anyhow!("CREATE tx missing contractAddress"))?;
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!("no CREATE transaction in broadcast artifact");
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
