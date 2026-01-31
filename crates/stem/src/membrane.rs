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
    fn current_epoch(
        &mut self,
        _: stem_capnp::membrane::CurrentEpochParams,
        mut results: stem_capnp::membrane::CurrentEpochResults,
    ) -> Promise<(), Error> {
        let epoch = self.get_current_epoch();
        let results_builder = results.get();
        let mut epoch_builder = results_builder.init_epoch();
        match fill_epoch_builder(&mut epoch_builder, &epoch) {
            Ok(()) => Promise::ok(()),
            Err(e) => Promise::err(e),
        }
    }

    fn watch_epoch(
        &mut self,
        _: stem_capnp::membrane::WatchEpochParams,
        mut results: stem_capnp::membrane::WatchEpochResults,
    ) -> Promise<(), Error> {
        let watcher = WatcherServer {
            receiver: self.receiver.clone(),
        };
        results.get().set_watcher(new_client(watcher));
        Promise::ok(())
    }

    fn graft(
        &mut self,
        _params: stem_capnp::membrane::GraftParams,
        mut results: stem_capnp::membrane::GraftResults,
    ) -> Promise<(), Error> {
        let epoch = self.get_current_epoch();
        let mut results_builder = results.get();
        let mut session_builder = results_builder.reborrow().init_session();
        if fill_epoch_builder(&mut session_builder.reborrow().init_issued_epoch(), &epoch).is_err() {
            return Promise::err(Error::failed("fill issued epoch".to_string()));
        }
        let poller = StatusPollerServer {
            issuance_epoch: epoch.clone(),
            receiver: self.receiver.clone(),
        };
        session_builder.set_status_poller(new_client(poller));
        let mut epoch_builder = results_builder.init_epoch();
        match fill_epoch_builder(&mut epoch_builder, &epoch) {
            Ok(()) => Promise::ok(()),
            Err(e) => Promise::err(e),
        }
    }
}

/// Watcher server: blocks on next() until the adopted epoch changes.
struct WatcherServer {
    receiver: watch::Receiver<Epoch>,
}

impl stem_capnp::watcher::Server for WatcherServer {
    fn next(
        &mut self,
        _: stem_capnp::watcher::NextParams,
        mut results: stem_capnp::watcher::NextResults,
    ) -> Promise<(), Error> {
        let mut receiver = self.receiver.clone();
        match tokio::runtime::Handle::current().block_on(receiver.changed()) {
            Ok(()) => {}
            Err(_) => return Promise::err(Error::failed("epoch watcher closed".to_string())),
        }
        let epoch = receiver.borrow().clone();
        let results_builder = results.get();
        let mut epoch_builder = results_builder.init_epoch();
        match fill_epoch_builder(&mut epoch_builder, &epoch) {
            Ok(()) => Promise::ok(()),
            Err(e) => Promise::err(e),
        }
    }
}

/// StatusPoller server: epoch-scoped; pollStatus returns StaleEpoch when seq differs.
struct StatusPollerServer {
    issuance_epoch: Epoch,
    receiver: watch::Receiver<Epoch>,
}

impl StatusPollerServer {
    fn check_epoch(&self) -> Result<(), Error> {
        let current = self.receiver.borrow();
        if current.seq != self.issuance_epoch.seq {
            return Err(Error::failed("staleEpoch: session epoch no longer current".to_string()));
        }
        Ok(())
    }
}

impl stem_capnp::status_poller::Server for StatusPollerServer {
    fn poll_status(
        &mut self,
        _: stem_capnp::status_poller::PollStatusParams,
        mut results: stem_capnp::status_poller::PollStatusResults,
    ) -> Promise<(), Error> {
        let status = match self.check_epoch() {
            Ok(()) => stem_capnp::Status::Ok,
            Err(_) => stem_capnp::Status::StaleEpoch,
        };
        results.get().set_status(status);
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
    async fn membrane_current_epoch_returns_watch_value() {
        let (_tx, rx) = watch::channel(epoch(1, b"head1", 100));
        let server = MembraneServer::new(rx);
        assert_eq!(server.get_current_epoch().seq, 1);
        assert_eq!(server.get_current_epoch().head, b"head1");
        assert_eq!(server.get_current_epoch().adopted_block, 100);
    }

    #[tokio::test]
    async fn status_poller_check_epoch_fails_when_seq_differs() {
        let (tx, rx) = watch::channel(epoch(1, b"head1", 100));
        let poller = StatusPollerServer {
            issuance_epoch: epoch(1, b"head1", 100),
            receiver: rx.clone(),
        };
        assert!(poller.check_epoch().is_ok());
        tx.send(epoch(2, b"head2", 101)).unwrap();
        let res = poller.check_epoch();
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("staleEpoch"));
    }
}
