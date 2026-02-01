//! Example: set up a membrane, obtain the bootstrap capability, graft, and call pollStatus.
//!
//! Demonstrates the membrane API: create a watch channel with an initial Epoch, build the
//! Membrane client (bootstrap capability), graft with a stub signer to get a Session, then
//! call pollStatus on the session's statusPoller. No RPC or Anvil required.
//!
//! Usage:
//!
//!   cargo run -p stem --example membrane_poll
//!
//! Options:
//!   --advance   Send a second epoch and poll again to show StaleEpoch.

use capnp_rpc::new_client;
use stem::stem_capnp;
use stem::{membrane_client, Epoch};
use tokio::sync::watch;

/// Stub Signer for graft: returns empty signature (example only).
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();
    let advance = args.iter().any(|a| a == "--advance");
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!(
            "Usage: membrane_poll [--advance]\n\
             Sets up a membrane, grabs the bootstrap capability, grafts, and calls pollStatus.\n\
             --advance  Advance epoch and poll again to show StaleEpoch."
        );
        std::process::exit(0);
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let epoch1 = Epoch {
            seq: 1,
            head: b"example".to_vec(),
            adopted_block: 100,
        };
        let (tx, rx) = watch::channel(epoch1.clone());
        let bootstrap = membrane_client(rx);
        let signer_client: stem_capnp::signer::Client = new_client(StubSigner);

        let mut graft_req = bootstrap.graft_request();
        graft_req.get().set_signer(signer_client);

        let graft_rpc_response = graft_req.send().promise.await?;
        let graft_response = graft_rpc_response.get()?;
        let session = graft_response.get_session()?;
        let poller = session.get_status_poller()?;

        let poll_req = poller.poll_status_request();
        let r = poll_req.send().promise.await?;
        let status = r.get()?.get_status()?;
        match status {
            stem_capnp::Status::Ok => println!("pollStatus: Ok"),
            stem_capnp::Status::StaleEpoch => println!("pollStatus: StaleEpoch"),
            stem_capnp::Status::Unauthorized => println!("pollStatus: Unauthorized"),
            stem_capnp::Status::InternalError => println!("pollStatus: InternalError"),
        }

        if advance {
            let epoch2 = Epoch {
                seq: 2,
                head: b"advanced".to_vec(),
                adopted_block: 101,
            };
            tx.send(epoch2).ok();
            let poll_req2 = poller.poll_status_request();
            let r2 = poll_req2.send().promise.await?;
            let status2 = r2.get()?.get_status()?;
            match status2 {
                stem_capnp::Status::Ok => println!("after advance: Ok"),
                stem_capnp::Status::StaleEpoch => println!("after advance: StaleEpoch"),
                stem_capnp::Status::Unauthorized => println!("after advance: Unauthorized"),
                stem_capnp::Status::InternalError => println!("after advance: InternalError"),
            }
        }

        Ok::<(), capnp::Error>(())
    })?;
    Ok(())
}
