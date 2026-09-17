# Historical source restoration

Base: `rustdesk-client` commit `4b066b1fbaa8d5d6f9b53cb1e5b25e484f229da1`,
directory `libs/hbb_common`, immediately before extraction in `c44803f5b`.
The historical repository license is in its root `LICENCE` (GNU AGPL v3).

Local changes are identified against the Git HEAD referenced by
`libs/hbb_common - kopia/.git`, not by copying the newer library wholesale.
The backup directory remains unchanged.

Ported local changes:
- Build-time `CNG_SERVER_ADDRESS` for rendezvous selection.
- Multiple encryption layers for authenticated transport and session keys.

The client application and Flutter interface now use the same historical
1.3.6 base. The library is tracked directly, without a Git submodule.
The historical source has no WebSocket/WSS transport: CNG uses UDP receiver
registration and authenticated encrypted TCP for technicians.

Both encryption tests pass (preserved transport sequencing when a session key
is installed, and tampered ciphertext rejection).

The Rust client check, release library build, and Flutter Windows release build
pass. See `MIGRATION-1.3.6.md` in the repository root for validation and rebuild
instructions. No post-extraction hbb_common upstream implementation was copied.
