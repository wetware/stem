//! Pure-Rust Membrane server: epoch validity via seq equality (Approach A),
//! backed by `watch::Receiver<Epoch>`, exposed over capnp-rpc.

use crate::stem_capnp;
use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::new_client;
use tokio::sync::watch;

/// Epoch value used by the membrane (matches capnp struct Epoch).
#[derive(Clone, Debug)]
pub struct Epoch {
    pub seq: u64,
    pub head: Vec<u8>,
    pub adopted_block: u64,
}

fn fill_epoch_builder(
    builder: &mut stem_capnp::epoch::Builder<'_>,
    epoch: &Epoch,
) -> Result<(), Error> {
    builder.set_seq(epoch.seq);
    builder.set_adopted_block(epoch.adopted_block);
    let head_builder = builder.reborrow().init_head(epoch.head.len() as u32);
    head_builder.copy_from_slice(epoch.head.as_slice());
    Ok(())
}

/// Membrane server: stable across epochs, backed by a watch receiver for the adopted epoch.
pub struct MembraneServer {
    receiver: watch::Receiver<Epoch>,
}

impl MembraneServer {
    pub fn new(receiver: watch::Receiver<Epoch>) -> Self {
        Self { receiver }
    }

    fn get_current_epoch(&self) -> Epoch {
        self.receiver.borrow().clone()
    }
}

impl stem_capnp::membrane::Server for MembraneServer {
    fn graft(
        &mut self,
        _params: stem_capnp::membrane::GraftParams,
        mut results: stem_capnp::membrane::GraftResults,
    ) -> Promise<(), Error> {
        let epoch = self.get_current_epoch();
        let mut session_builder = results.get().init_session();
        if fill_epoch_builder(&mut session_builder.reborrow().init_issued_epoch(), &epoch).is_err() {
            return Promise::err(Error::failed("fill issued epoch".to_string()));
        }
        let guard = EpochGuard {
            issued_seq: epoch.seq,
            receiver: self.receiver.clone(),
        };
        let poller = StatusPollerServer { guard };
        session_builder.set_status_poller(new_client(poller));
        Promise::ok(())
    }
}

/// Guard that checks whether the epoch under which a capability was issued is
/// still current. Shared by all session-scoped capability servers so that
/// every RPC hard-fails once the epoch advances.
#[derive(Clone)]
struct EpochGuard {
    issued_seq: u64,
    receiver: watch::Receiver<Epoch>,
}

impl EpochGuard {
    fn check(&self) -> Result<(), Error> {
        let current = self.receiver.borrow();
        if current.seq != self.issued_seq {
            return Err(Error::failed("staleEpoch: session epoch no longer current".to_string()));
        }
        Ok(())
    }
}

/// StatusPoller server: epoch-scoped; pollStatus returns an RPC error when the
/// epoch has advanced past the one under which this capability was issued.
struct StatusPollerServer {
    guard: EpochGuard,
}

impl stem_capnp::status_poller::Server for StatusPollerServer {
    fn poll_status(
        &mut self,
        _: stem_capnp::status_poller::PollStatusParams,
        mut results: stem_capnp::status_poller::PollStatusResults,
    ) -> Promise<(), Error> {
        if let Err(e) = self.guard.check() {
            return Promise::err(e);
        }
        results.get().set_status(stem_capnp::Status::Ok);
        Promise::ok(())
    }
}

/// Builds a Membrane capability client from a watch receiver (for use over capnp-rpc).
pub fn membrane_client(receiver: watch::Receiver<Epoch>) -> stem_capnp::membrane::Client {
    new_client(MembraneServer::new(receiver))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch(seq: u64, head: &[u8], adopted_block: u64) -> Epoch {
        Epoch {
            seq,
            head: head.to_vec(),
            adopted_block,
        }
    }

    #[tokio::test]
    async fn status_poller_check_epoch_fails_when_seq_differs() {
        let (tx, rx) = watch::channel(epoch(1, b"head1", 100));
        let guard = EpochGuard {
            issued_seq: 1,
            receiver: rx.clone(),
        };
        assert!(guard.check().is_ok());
        tx.send(epoch(2, b"head2", 101)).unwrap();
        let res = guard.check();
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("staleEpoch"));
    }
}
