@0x9bce094a026970c4;

struct Epoch {
  seq @0 :UInt64;        # Monotonic epoch sequence number (from Stem.seq).
  head @1 :Data;         # Opaque head bytes from the Stem contract.
  adoptedBlock @2 :UInt64;# Block number at which this epoch was adopted.
}

enum Status {
  ok @0;             # Operation succeeded under the current epoch.
  staleEpoch @1;     # Session was minted under a different epoch.
  unauthorized @2;   # Caller not authorized under current policy.
  internalError @3;  # Unexpected internal failure.
}

interface Signer {
  sign @0 (domain :Text, nonce :UInt64) -> (sig :Data);
  # Sign an arbitrary nonce under a given domain string.
}

interface Watcher {
  next @0 () -> (epoch :Epoch);
  # Blocks until the adopted epoch changes.
}

interface StatusPoller {
  pollStatus @0 () -> (status :Status);
}

struct Session {
  issuedEpoch @0 :Epoch;
  # Epoch under which this session was minted.

  statusPoller @1 :StatusPoller;
  # Capability for polling session status. Can be withheld (client receives null capability).
}

interface Membrane {
  currentEpoch @0 () -> (epoch :Epoch);
  # Returns the currently adopted epoch (read-only, safe).

  watchEpoch @1 () -> (watcher :Watcher);
  # Returns a watcher capability for observing epoch changes.

  graft @2 (signer :Signer) -> (session :Session, epoch :Epoch);
  # Graft a signer to the membrane, establishing an epoch-scoped session.
}
