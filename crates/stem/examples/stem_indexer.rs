//! Example: run StemIndexer and print each HeadUpdatedObserved.
//!
//! Usage: cargo run -p stem --example stem_indexer -- --http-url URL --ws-url WS_URL --contract 0x...

use stem::{IndexerConfig, StemIndexer};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();
    let mut http_url = String::new();
    let mut ws_url = String::new();
    let mut contract = String::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--http-url" => {
                i += 1;
                http_url = args.get(i).cloned().unwrap_or_default();
            }
            "--ws-url" => {
                i += 1;
                ws_url = args.get(i).cloned().unwrap_or_default();
            }
            "--contract" => {
                i += 1;
                contract = args.get(i).cloned().unwrap_or_default();
            }
            _ => {}
        }
        i += 1;
    }
    if http_url.is_empty() || ws_url.is_empty() || contract.is_empty() {
        eprintln!("Usage: stem_indexer --http-url URL --ws-url WS_URL --contract 0xADDR");
        std::process::exit(1);
    }
    let addr_hex = contract.strip_prefix("0x").unwrap_or(&contract);
    let addr_bytes = hex::decode(addr_hex)?;
    if addr_bytes.len() != 20 {
        eprintln!("contract must be 20 bytes");
        std::process::exit(1);
    }
    let mut contract_address = [0u8; 20];
    contract_address.copy_from_slice(&addr_bytes);

    let config = IndexerConfig {
        ws_url,
        http_url: http_url.clone(),
        contract_address,
        start_block: 0,
        getlogs_max_range: 1000,
        reconnection: Default::default(),
    };
    let indexer = Arc::new(StemIndexer::new(config));
    let mut recv = indexer.subscribe();
    let indexer_clone = Arc::clone(&indexer);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let _ = indexer_clone.run().await;
        });
    });
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        loop {
            tokio::select! {
                Ok(ev) = recv.recv() => {
                    println!(
                        "HeadUpdated seq={} block={} log_index={} writer=0x{} cid_len={}",
                        ev.seq,
                        ev.block_number,
                        ev.log_index,
                        hex::encode(ev.writer),
                        ev.cid.len()
                    );
                }
                _ = tokio::time::sleep(Duration::from_secs(3600)) => break,
            }
        }
    });
    Ok(())
}
