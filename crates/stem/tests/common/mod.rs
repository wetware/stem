//! Common helpers for integration tests.

use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

/// True if `anvil`, `forge`, and `cast` are in PATH (Foundry toolchain available).
/// Use at the start of integration tests to skip when not in CI/local dev with Foundry.
pub fn foundry_available() -> bool {
    fn in_path(cmd: &str, args: &[&str]) -> bool {
        Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    in_path("anvil", &["--help"]) && in_path("forge", &["--help"]) && in_path("cast", &["--help"])
}

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

/// Call setHead(bytes) via cast send (Option A).
pub fn set_head(repo_root: &std::path::Path, rpc_url: &str, contract: &str, cid_hex: &str) -> Result<()> {
    let out = Command::new("cast")
        .current_dir(repo_root)
        .args([
            "send", contract,
            "setHead(bytes)",
            cid_hex,
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
