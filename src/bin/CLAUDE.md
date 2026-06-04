# src/bin/

Thin `main()` wrappers; real logic lives in the library modules. Each binary builds with its own
feature (see `Cargo.toml`):

- `client.rs` (`with-client`) -> `client::run_client(CliClient::parse())`
- `client_ui.rs` (`with-gui`) -> `ui::run_ui()` (Android uses `ui::android.rs::android_main`)
- `server.rs` (`with-server`) -> `server::run_server`
- `commander.rs` (`with-server`) -> the privileged executor that owns the Unix socket

Keep these files minimal: parse/args + dispatch only, so the logic stays unit-testable.
