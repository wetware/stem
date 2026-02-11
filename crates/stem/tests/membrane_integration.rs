//! Integration test: Anvil → indexer → Epoch → Membrane server → client → graft → Session → pollStatus.
//! All local: membrane server and client are in-process (capnp-rpc local dispatch).

mod common;

use capnp_rpc::new_client;
use common::{deploy_stem, set_head, spawn_anvil};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use stem::stem_capnp;
use stem::{membrane_client, Epoch, IndexerConfig, StemIndexer};
use tokio::sync::watch;
use tokio::time::timeout;
use tracing_subscriber::EnvFilter;

/// Stub Signer for graft: returns empty signature (local test only).
struct StubSigner;

impl stem_capnp::signer::Server for StubSigner {
    fn sign(
        &mut self,
        _: stem_capnp::signer::SignParams,
        mut results: stem_capnp::signer::SignResults,
    ) -> capnp::capability::Promise<(), capnp::Error> {
        results.get().init_sig(0);
        capnp::capability::Promise::ok(())
    }
}

fn observed_to_epoch(ev: &stem::HeadUpdatedObserved) -> Epoch {
    Epoch {
        seq: ev.seq,
        head: ev.cid.clone(),
        adopted_block: ev.block_number,
    }
}

#[tokio::test]
async fn test_membrane_graft_poll_status_against_anvil() {
    if !common::foundry_available() {
        eprintln!("skipping test_membrane_graft_poll_status_against_anvil: anvil/forge/cast not in PATH");
        return;
    }
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("stem=debug".parse().unwrap()))
        .with_test_writer()
        .try_init();

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap();
    let (mut anvil_process, rpc_url) = spawn_anvil().await.expect("spawn anvil");
    let contract_addr = deploy_stem(repo_root, &rpc_url).expect("deploy Stem");
    let addr_bytes = hex::decode(contract_addr.strip_prefix("0x").unwrap_or(&contract_addr)).expect("hex");
    let mut contract_address = [0u8; 20];
    contract_address.copy_from_slice(&addr_bytes);

    set_head(repo_root, &rpc_url, &contract_addr, "setHead(bytes)", "0x697066732f2f6669727374", None).expect("setHead 1");

    let ws_url = rpc_url.replace("http://", "ws://").replace("https://", "wss://");
    let config = IndexerConfig {
        ws_url: ws_url.clone(),
        http_url: rpc_url.clone(),
        contract_address,
        start_block: 0,
        getlogs_max_range: 1000,
        reconnection: Default::default(),
    };
    let indexer = Arc::new(StemIndexer::new(config));
    let mut recv = indexer.subscribe();
    let indexer_clone = Arc::clone(&indexer);
    let indexer_task = tokio::spawn(async move {
        let _ = indexer_clone.run().await;
    });

    let first_ev = timeout(
        Duration::from_secs(15),
        async {
            loop {
                if let Ok(ev) = recv.recv().await {
                    return ev;
                }
            }
        },
    )
    .await
    .expect("timeout waiting for first event");

    indexer_task.abort();
    let _ = anvil_process.kill();

    let epoch1 = observed_to_epoch(&first_ev);
    let epoch2 = Epoch {
        seq: first_ev.seq + 1,
        head: b"next_head".to_vec(),
        adopted_block: first_ev.block_number + 1,
    };

    let (tx, rx) = watch::channel(epoch1.clone());
    let membrane = membrane_client(rx);
    let signer_client: stem_capnp::signer::Client = new_client(StubSigner);

    let mut graft_req = membrane.graft_request();
    graft_req.get().set_signer(signer_client);

    let graft_rpc_response = graft_req.send().promise.await.expect("graft RPC");
    let graft_response = graft_rpc_response.get().expect("graft results");
    let session = graft_response.get_session().expect("session");
    let issued = session.get_issued_epoch().expect("issued_epoch");
    assert_eq!(issued.get_seq(), epoch1.seq);
    assert_eq!(issued.get_adopted_block(), epoch1.adopted_block);

    let poller = session.get_status_poller().expect("status_poller");
    let poll_req = poller.poll_status_request();
    let status1 = {
        let r = poll_req.send().promise.await.expect("poll_status RPC");
        r.get().expect("poll_status results").get_status().expect("status")
    };
    assert_eq!(status1, stem_capnp::Status::Ok, "session should be ok under current epoch");

    tx.send(epoch2).unwrap();
    let poll_req2 = poller.poll_status_request();
    let status2 = {
        let r = poll_req2.send().promise.await.expect("poll_status again RPC");
        r.get().expect("poll_status again results").get_status().expect("status")
    };
    assert_eq!(status2, stem_capnp::Status::StaleEpoch, "session should be stale after epoch advance");
}

/// No-chain regression test: same poller returns StaleEpoch after epoch advance, then re-graft returns Ok.
#[tokio::test]
async fn test_membrane_stale_epoch_then_recovery_no_chain() {
    let epoch1 = Epoch {
        seq: 1,
        head: b"head1".to_vec(),
        adopted_block: 100,
    };
    let epoch2 = Epoch {
        seq: 2,
        head: b"head2".to_vec(),
        adopted_block: 101,
    };

    let (tx, rx) = watch::channel(epoch1.clone());
    let membrane = membrane_client(rx);
    let signer_client: stem_capnp::signer::Client = new_client(StubSigner);

    let mut graft_req = membrane.graft_request();
    graft_req.get().set_signer(signer_client.clone());
    let graft_rpc_response = graft_req.send().promise.await.expect("graft RPC");
    let graft_response = graft_rpc_response.get().expect("graft results");
    let session = graft_response.get_session().expect("session");
    let poller = session.get_status_poller().expect("status_poller");

    let poll_req = poller.poll_status_request();
    let status1 = {
        let r = poll_req.send().promise.await.expect("poll_status RPC");
        r.get().expect("poll_status results").get_status().expect("status")
    };
    assert_eq!(status1, stem_capnp::Status::Ok, "session should be ok under current epoch");

    tx.send(epoch2).unwrap();
    let poll_req2 = poller.poll_status_request();
    let status2 = {
        let r = poll_req2.send().promise.await.expect("poll_status again RPC");
        r.get().expect("poll_status again results").get_status().expect("status")
    };
    assert_eq!(
        status2,
        stem_capnp::Status::StaleEpoch,
        "same poller must return StaleEpoch after epoch advance"
    );

    let mut graft_req2 = membrane.graft_request();
    graft_req2.get().set_signer(signer_client.clone());
    let graft_rpc_response2 = graft_req2.send().promise.await.expect("re-graft RPC");
    let graft_response2 = graft_rpc_response2.get().expect("re-graft results");
    let session2 = graft_response2.get_session().expect("session");
    let poller2 = session2.get_status_poller().expect("status_poller");
    let poll_req3 = poller2.poll_status_request();
    let status3 = {
        let r = poll_req3.send().promise.await.expect("poll_status after re-graft RPC");
        r.get().expect("poll_status results").get_status().expect("status")
    };
    assert_eq!(status3, stem_capnp::Status::Ok, "re-graft session should be ok");
}
