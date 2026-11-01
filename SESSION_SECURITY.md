# Session and local data security

This fork encrypts saved sessions with AES-256-GCM. The random 256-bit key is kept in the operating system credential vault under service `net.Trikim7.ctdozlite`; the account is derived from the absolute session file path. It is never written beside the encrypted file. A new random 96-bit nonce is used on each save, with versioned authenticated context.

Linux builds explicitly use Secret Service with encrypted D-Bus transport; Windows uses Credential Manager; macOS uses Keychain. Unsupported platforms fail compilation instead of selecting a mock credential backend. There is no plaintext save fallback. When the vault cannot be accessed, saving fails and the GUI displays a session persistence warning. A previous encrypted file is preserved if writing its replacement fails.

The application stores its files in the current user's application-data directory. Unix directory/file permissions are restricted. Windows files inherit the user application-data directory ACL; the app does not install a custom Windows ACL. Do not run the app as an administrator/root. File and directory checks reduce accidental symlink writes; they do not isolate the app from an attacker already running as the same OS user.

The vault key does not authenticate the app author and does not stop malware with access to your unlocked OS session. Sensitive SQLite history fields and the complete thread cache are encrypted with independent vault-backed data keys. This is field encryption, not whole-database encryption: IDs, timestamps, group/outgoing flags, row counts and schema remain visible. Thumbnails, stickers and downloaded attachments remain unencrypted. See [history encryption](docs/HISTORY_ENCRYPTION.md). Full-disk encryption and an isolated OS account remain relevant.

Existing plaintext sessions are migrated in place before credentials are restored, provided the vault and atomic replacement both succeed. Legacy files elsewhere are not searched or imported automatically. Old plaintext backups, snapshots and SSD blocks are not securely erased. Copying an encrypted file to another location does not migrate its vault key.

Raw call/VoIP logs and payload logging are disabled by default, including developer mode. The explicit `CTDOZLITE_SENSITIVE_LOGS=1` diagnostic option can expose private message content, session and call data. Never upload diagnostics or local data publicly.

Logging out removes local files through existing application paths; this is not a guarantee that the server session was revoked. If credentials or local data may have been exposed, revoke the session using the official Zalo app on your phone. Merely deleting this client does not revoke a server-side session.
