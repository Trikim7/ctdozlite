# Security hardening — 2026-11-01

Base: upstream commit `73849bde5e5ffff21537e92886891db510e9c857` (upstream tag v0.0.2).
Fork source version: **0.1.0**. The old upstream executable was reviewed separately and is not a build of this fork.

## Threats addressed

| Threat | Change | Evidence / boundary |
|---|---|---|
| Copying a plaintext session file permits session reuse | AES-256-GCM envelope and OS-vault key, fresh random nonce, authenticated context | Unit tests cover round trips, ciphertext tampering, wrong/missing keys, malformed nonces, unknown versions and vault failures |
| Failed writes truncate credentials / racing processes replace keys | Same-directory temporary file + sync + atomic replacement; process mutex and OS file lock | Atomic replacement / permission tests; lock held across key creation and replacement |
| Private files stored in arbitrary current directory | Per-user application data; Unix 0700 directory and 0600 session | Filesystem tests verify owner-only file mode and reject symlink targets |
| Login/key details leak through developer logs | Complete secret redaction; query/userinfo/fragment stripping; request bodies omitted by default; raw call and packet traces opt-in | Logging and URL tests; production UI warns when encrypted session persistence fails |
| A server-supplied/saved endpoint steals credentials | Exact domain boundary, HTTPS/WSS, port 443, no userinfo, authenticated HTTP redirects disabled, cookie insertion validates host | Tests include lookalike suffixes, userinfo tricks, plaintext transport, loopback and multipart rejection before network I/O |
| Existing plaintext session silently remains unprotected | In-place migration must successfully encrypt and replace before restoring credentials | Migration test also verifies original bytes are preserved when vault write fails |
| Downloaded release cannot be linked to source | Pinned CI actions, restricted permissions, version alignment and GitHub build attestations for future releases | This changes the workflow; it does not retroactively attest upstream binaries or claim a fork release already exists |

## Dependency review

Queried OSV for 513 registry package versions from Cargo.lock during implementation. Updated Tokio 1.54.0 to 1.54.1 for RUSTSEC-2026-0384. That number describes the lockfile query, including packages for other targets/features, rather than 513 libraries all executed at runtime.

Remaining reported dependencies include:

- `lru 0.12.5`, via `ratatui 0.29.0`: RUSTSEC-2026-0002 and RUSTSEC-2026-0253 concern memory unsoundness. Upgrading this transitive major version needs a separate TUI dependency migration. No complete exploitability assessment was performed; these findings are not waived as safe.
- `paste`, `audiopus_sys`, `ttf-parser`: unmaintained dependency notices.
- Lockfile-only `zvariant`, `zvariant_derive`, `zbus_names`: unmaintained notices. `cargo tree -i zvariant` shows no selected dependency on the tested Linux default feature set; other targets still need review.

Advisory sources: [Tokio](https://rustsec.org/advisories/RUSTSEC-2026-0384.html), [lru IterMut](https://rustsec.org/advisories/RUSTSEC-2026-0002.html), [lru panic safety](https://rustsec.org/advisories/RUSTSEC-2026-0253.html). This review is a point-in-time package check, not a promise that no other vulnerabilities exist.

## Validation and remaining limits

The local test suite and Clippy are run on Linux. Pre-existing Clippy warnings remain; this work does not claim a warning-free repository. The optional native vault test uses a temporary credential entry and cleans it up. Tests do not exercise Zalo authentication or access real chat content.

SQLite history and thread/media caches are still unencrypted. Windows ACL enforcement relies on the user's application-data directory; a Windows/macOS runtime test is still needed. Moving the encrypted session changes its vault lookup identity. No real login, messaging, call, camera/microphone, account-revocation or end-to-end installer tests have been performed. Treat this as a security improvement suitable for further testing, not a production security certification.
