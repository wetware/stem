# Stem membrane: ocap API on top of the Stem finalizer/adopted-epoch stream.
#
# Epoch-scoping rule:
# - Membrane and EpochWatcher are stable across epochs (durable capabilities).
# - Session is epoch-scoped and MUST fail once the adopted epoch advances (staleEpoch).
# - Renewal: clients call Membrane.login(...) again after receiving staleEpoch
#   to obtain a new Session under the current adopted epoch.

@0x9bce094a026970c4;

struct Epoch {
  seq @0 :UInt64;
  # Opaque bytes from Stem; not claimed to be a CID.
  head @1 :Data;
  adoptedBlock @2 :UInt64;
}

# Abstract signer capability; no Ethereum-specific types in schema.
interface Signer {
  sign @0 (domain :Text, nonce :UInt64) -> (sig :Data);
}

# Stable across epochs: the single durable capability exported by the system.
# Methods currentEpoch and watchEpoch are read-only and safe. login returns
# an epoch-scoped Session minted under the current adopted epoch.
interface Membrane {
  # Returns the currently adopted epoch; read-only, safe, stable.
  currentEpoch @0 () -> (epoch :Epoch);
  # Returns a watcher capability for observing epoch changes.
  watchEpoch @1 () -> (watcher :EpochWatcher);
  # Auth dance (details abstracted). Returns an epoch-scoped Session minted
  # under the *current* epoch and the epoch value so callers know issuance context.
  login @2 (signer :Signer) -> (session :Session, epoch :Epoch);
}

# Stable, read-only: observe adopted epoch changes. Callers may loop on next().
interface EpochWatcher {
  # Blocks until the adopted epoch changes; returns the new epoch.
  next @0 () -> (epoch :Epoch);
}

# Epoch-scoped, privileged. Bound to a specific epoch; once the adopted epoch
# is no longer equal to this session's issuance epoch, all privileged methods
# MUST fail with a deterministic error whose message includes "staleEpoch".
# (Renewal: call Membrane.login again to obtain a new Session.)
interface Session {
  # Returns the issuance epoch for this session.
  epoch @0 () -> (epoch :Epoch);
  # Placeholder privileged method; MUST fail with staleEpoch when adopted epoch != issuance epoch.
  ping @1 () -> ();
  # Optional placeholder privileged method (minimal).
  getPolicyRoot @2 () -> (root :Data);
}
