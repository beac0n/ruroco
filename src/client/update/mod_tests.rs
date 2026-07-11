use crate::client::update::{GithubApiAsset, Updater, GH_RELEASES_URL};
use openssl::pkey::{PKey, Private};
use openssl::sign::Signer;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::{env, fs};

fn create_updater(dir: &Path) -> Updater {
    Updater::create(false, None, Some(dir.to_path_buf()), false).unwrap()
}

/// Builds an Updater that verifies against `public_key_pem` (so tests can sign payloads
/// with a matching private key) instead of the embedded release key.
fn updater_with_key(dir: &Path, public_key_pem: Vec<u8>) -> Updater {
    Updater {
        force: false,
        version: None,
        bin_path: dir.to_path_buf(),
        server: false,
        public_key_pem,
        releases_url: GH_RELEASES_URL.to_string(),
    }
}

/// Test-only composition of the two update steps `Updater::update` now runs separately (verify
/// every target, then swap every target), matching the pre-split `download_and_save_bin` shape
/// so the existing tests below don't need to be rewritten one by one.
fn download_and_save_bin(
    updater: &Updater,
    bin_url: String,
    sig_url: String,
    bin_name: &str,
    permissions_mode: u32,
    user_and_group: Option<&str>,
) -> anyhow::Result<()> {
    let bin_bytes = updater.download_and_verify_bin(bin_url, sig_url, bin_name)?;
    updater.save_bin(&bin_bytes, bin_name, permissions_mode, user_and_group)
}

fn test_keypair() -> (Vec<u8>, PKey<Private>) {
    let key = PKey::generate_ed25519().unwrap();
    (key.public_key_to_pem().unwrap(), key)
}

fn sign_bytes(key: &PKey<Private>, message: &[u8]) -> Vec<u8> {
    Signer::new_without_digest(key).unwrap().sign_oneshot_to_vec(message).unwrap()
}

fn create_readonly_dir(parent: &Path) -> PathBuf {
    let dir = parent.join("readonly");
    fs::create_dir_all(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o444)).unwrap();
    dir
}

/// Spawns a local HTTP server that serves `payload` once, returns (port, join handle).
fn serve_payload(payload: Vec<u8>) -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1024];
        let _ = stream.read(&mut buf);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            payload.len()
        );
        stream.write_all(resp.as_bytes()).unwrap();
        stream.write_all(&payload).unwrap();
    });
    (port, handle)
}

fn make_asset(name: &str, url: &str) -> GithubApiAsset {
    GithubApiAsset {
        name: name.to_string(),
        browser_download_url: url.to_string(),
    }
}

#[test]
fn test_create_with_nonexistent_bin_path() {
    let result =
        Updater::create(false, None, Some(PathBuf::from("/tmp/no_such_dir_ruroco")), false);
    assert!(result.unwrap_err().to_string().contains("does not exist or is not a directory"));
}

#[test]
fn test_create_with_file_as_bin_path() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("not_a_dir");
    fs::write(&file_path, "test").unwrap();
    let result = Updater::create(false, None, Some(file_path), false);
    assert!(result.unwrap_err().to_string().contains("does not exist or is not a directory"));
}

#[test]
fn test_create_with_valid_bin_path() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Updater::create(false, None, Some(dir.path().to_path_buf()), false).is_ok());
}

#[test]
fn test_create_server_mode() {
    let dir = tempfile::tempdir().unwrap();
    let updater =
        Updater::create(true, Some("v1.0.0".to_string()), Some(dir.path().to_path_buf()), true)
            .unwrap();
    assert!(updater.server);
    assert!(updater.force);
}

#[test]
fn test_check_if_writable() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Updater::check_if_writable(dir.path()).unwrap());
}

#[test]
fn test_check_if_writable_readonly() {
    let dir = tempfile::tempdir().unwrap();
    let readonly_dir = create_readonly_dir(dir.path());
    assert!(!Updater::check_if_writable(&readonly_dir).unwrap());
    let _ = fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755));
}

#[test]
fn test_validate_dir_path_creates_dir() {
    let dir = tempfile::tempdir().unwrap();
    let new_dir = dir.path().join("new_sub_dir");
    assert!(!new_dir.exists());
    let result = Updater::validate_dir_path(new_dir.clone()).unwrap();
    assert!(new_dir.exists());
    assert_eq!(result, new_dir);
}

#[test]
fn test_validate_dir_path_existing_writable_dir() {
    let dir = tempfile::tempdir().unwrap();
    let result = Updater::validate_dir_path(dir.path().to_path_buf()).unwrap();
    assert_eq!(result, dir.path());
}

#[test]
fn test_validate_dir_path_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("a_file");
    fs::write(&file_path, "test").unwrap();
    assert!(Updater::validate_dir_path(file_path)
        .unwrap_err()
        .to_string()
        .contains("exists but is not a directory"));
}

#[test]
fn test_get_download_url_found() {
    let assets = vec![
        make_asset("client-v1.0.0-x86_64-linux", "https://example.com/client"),
        make_asset("server-v1.0.0-x86_64-linux", "https://example.com/server"),
    ];
    let result = Updater::get_download_url(&assets, "client-v1.0.0-x86_64-linux").unwrap();
    assert_eq!(result, "https://example.com/client");
}

#[test]
fn test_get_download_url_not_found() {
    let assets = vec![make_asset("other-binary", "https://example.com/other")];
    assert!(Updater::get_download_url(&assets, "nonexistent")
        .unwrap_err()
        .to_string()
        .contains("Could not find nonexistent"));
}

#[test_with::env(TEST_ONLINE)]
#[test]
fn test_update_already_current_version() {
    let dir = tempfile::tempdir().unwrap();
    let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let updater =
        Updater::create(false, Some(current_version), Some(dir.path().to_path_buf()), false)
            .unwrap();
    assert!(updater.update().is_ok());
}

#[test_with::env(TEST_ONLINE)]
#[test]
fn test_get_github_api_data_latest() {
    let data = Updater::get_github_api_data_from(GH_RELEASES_URL, None).unwrap();
    assert!(!data.tag_name.is_empty());
    assert!(!data.assets.is_empty());
}

#[test_with::env(TEST_ONLINE)]
#[test]
fn test_get_github_api_data_specific_version() {
    assert_eq!(
        Updater::get_github_api_data_from(GH_RELEASES_URL, Some(&"v0.10.0".to_string()))
            .unwrap()
            .tag_name,
        "v0.10.0"
    );
}

#[test_with::env(TEST_ONLINE)]
#[test]
fn test_get_github_api_data_nonexistent_version() {
    assert!(
        Updater::get_github_api_data_from(GH_RELEASES_URL, Some(&"v99.99.99".to_string())).is_err()
    );
}

#[test]
fn test_get_github_api_data_skips_prerelease_when_no_version_given() {
    let api_json = r#"[
        {"tag_name":"v2.0.0-rc1","assets":[],"prerelease":true},
        {"tag_name":"v1.0.0","assets":[],"prerelease":false}
    ]"#;
    let (port, _handle) = serve_payload(api_json.as_bytes().to_vec());
    let releases_url = format!("http://127.0.0.1:{port}");

    let data = Updater::get_github_api_data_from(&releases_url, None).unwrap();

    assert_eq!(data.tag_name, "v1.0.0");
}

#[test]
fn test_get_github_api_data_explicit_version_allows_prerelease() {
    let api_json = r#"[
        {"tag_name":"v2.0.0-rc1","assets":[],"prerelease":true},
        {"tag_name":"v1.0.0","assets":[],"prerelease":false}
    ]"#;
    let (port, _handle) = serve_payload(api_json.as_bytes().to_vec());
    let releases_url = format!("http://127.0.0.1:{port}");

    let data =
        Updater::get_github_api_data_from(&releases_url, Some(&"v2.0.0-rc1".to_string())).unwrap();

    assert_eq!(data.tag_name, "v2.0.0-rc1");
}

#[test]
fn test_create_with_readonly_bin_path() {
    let dir = tempfile::tempdir().unwrap();
    let readonly_dir = create_readonly_dir(dir.path());
    let result = Updater::create(false, None, Some(readonly_dir.clone()), false);
    assert!(result.unwrap_err().to_string().contains("can't write to"));
    let _ = fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755));
}

#[test]
fn test_validate_dir_path_readonly() {
    let dir = tempfile::tempdir().unwrap();
    let readonly_dir = create_readonly_dir(dir.path());
    let result = Updater::validate_dir_path(readonly_dir.clone());
    assert!(result.unwrap_err().to_string().contains("can't write to"));
    let _ = fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755));
}

#[test]
fn test_create_server_mode_auto_bin_path() {
    // server mode with no bin_path falls back to SERVER_BIN_DIR; whether /usr/local/bin is
    // writable decides ok vs err, so we only assert it doesn't panic.
    let _ = Updater::create(false, None, None, true);
}

#[test]
fn test_create_no_home_env_returns_error() {
    env::remove_var("RUROCO_CONF_DIR");
    env::remove_var("HOME");
    let result = Updater::create(false, None, None, false);
    assert!(result.unwrap_err().to_string().contains("Could not get home env"));
}

#[test]
fn test_download_and_save_bin_creates_file() {
    let (pub_pem, key) = test_keypair();
    let content = b"fake-binary-content".to_vec();
    let sig = sign_bytes(&key, &content);
    let (bin_port, bin_handle) = serve_payload(content.clone());
    let (sig_port, sig_handle) = serve_payload(sig);
    let dir = tempfile::tempdir().unwrap();
    let updater = updater_with_key(dir.path(), pub_pem);
    let result = download_and_save_bin(
        &updater,
        format!("http://127.0.0.1:{bin_port}/bin"),
        format!("http://127.0.0.1:{sig_port}/sig"),
        "tb",
        0o755,
        None,
    );
    bin_handle.join().unwrap();
    sig_handle.join().unwrap();
    assert!(result.is_ok(), "download_and_save_bin failed: {result:?}");

    let target = dir.path().join("tb");
    assert_eq!(fs::read(&target).unwrap(), content);
    assert_eq!(fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o755);
}

#[test]
fn test_download_and_save_bin_renames_existing_to_old() {
    let (pub_pem, key) = test_keypair();
    let content = b"new-binary".to_vec();
    let sig = sign_bytes(&key, &content);
    let (bin_port, bin_handle) = serve_payload(content.clone());
    let (sig_port, sig_handle) = serve_payload(sig);
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tb"), b"old-binary").unwrap();

    let updater = updater_with_key(dir.path(), pub_pem);
    let result = download_and_save_bin(
        &updater,
        format!("http://127.0.0.1:{bin_port}/bin"),
        format!("http://127.0.0.1:{sig_port}/sig"),
        "tb",
        0o755,
        None,
    );
    bin_handle.join().unwrap();
    sig_handle.join().unwrap();
    assert!(result.is_ok(), "download_and_save_bin failed: {result:?}");

    assert_eq!(fs::read(dir.path().join("tb")).unwrap(), b"new-binary");
    assert_eq!(fs::read(dir.path().join("tb.old")).unwrap(), b"old-binary");
}

#[test]
fn test_download_and_save_bin_replaced_binary_is_executable() {
    let (pub_pem, key) = test_keypair();
    let content = b"new-executable".to_vec();
    let sig = sign_bytes(&key, &content);
    let (bin_port, bin_handle) = serve_payload(content.clone());
    let (sig_port, sig_handle) = serve_payload(sig);
    let dir = tempfile::tempdir().unwrap();
    // start with a non-executable existing binary so we know the mode was set on replace
    fs::write(dir.path().join("tb"), b"old-binary").unwrap();
    fs::set_permissions(dir.path().join("tb"), fs::Permissions::from_mode(0o600)).unwrap();

    let updater = updater_with_key(dir.path(), pub_pem);
    let result = download_and_save_bin(
        &updater,
        format!("http://127.0.0.1:{bin_port}/bin"),
        format!("http://127.0.0.1:{sig_port}/sig"),
        "tb",
        0o755,
        None,
    );
    bin_handle.join().unwrap();
    sig_handle.join().unwrap();
    assert!(result.is_ok(), "download_and_save_bin failed: {result:?}");

    let target = dir.path().join("tb");
    assert_eq!(fs::read(&target).unwrap(), content);
    let mode = fs::metadata(&target).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
    // owner execute bit must be set on the replaced binary
    assert_ne!(mode & 0o100, 0, "replaced binary is not executable");
}

#[test]
fn test_download_and_save_bin_target_never_missing_and_no_temp_leftover() {
    // The atomic temp-file + rename flow must never leave the target absent: after a
    // successful replace both the new target and its `.old` backup exist, and no partial
    // temp file survives. There is no code path that fs::write()s directly onto the target.
    let (pub_pem, key) = test_keypair();
    let content = b"brand-new-binary".to_vec();
    let sig = sign_bytes(&key, &content);
    let (bin_port, bin_handle) = serve_payload(content.clone());
    let (sig_port, sig_handle) = serve_payload(sig);
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tb"), b"old-binary").unwrap();

    let updater = updater_with_key(dir.path(), pub_pem);
    let result = download_and_save_bin(
        &updater,
        format!("http://127.0.0.1:{bin_port}/bin"),
        format!("http://127.0.0.1:{sig_port}/sig"),
        "tb",
        0o755,
        None,
    );
    bin_handle.join().unwrap();
    sig_handle.join().unwrap();
    assert!(result.is_ok(), "download_and_save_bin failed: {result:?}");

    // target present with new contents, backup present with old contents
    assert_eq!(fs::read(dir.path().join("tb")).unwrap(), content);
    assert_eq!(fs::read(dir.path().join("tb.old")).unwrap(), b"old-binary");
    // no stray temp files from the atomic write remain in the directory
    let leftover: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
        .collect();
    assert!(leftover.is_empty(), "temp files left behind: {leftover:?}");
}

#[test]
fn test_download_and_save_bin_download_failure() {
    let dir = tempfile::tempdir().unwrap();
    let result = download_and_save_bin(
        &create_updater(dir.path()),
        "http://127.0.0.1:1/nonexistent".to_string(),
        "http://127.0.0.1:1/nonexistent.sig".to_string(),
        "tb",
        0o755,
        None,
    );
    assert!(result.unwrap_err().to_string().contains("Could not get binary"));
}

#[test]
fn test_download_and_save_bin_invalid_signature_aborts() {
    let (pub_pem, key) = test_keypair();
    let content = b"genuine-binary".to_vec();
    // Sign different bytes so the signature does not match the served binary.
    let sig = sign_bytes(&key, b"some-other-bytes");
    let (bin_port, bin_handle) = serve_payload(content);
    let (sig_port, sig_handle) = serve_payload(sig);
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("tb"), b"old-binary").unwrap();

    let updater = updater_with_key(dir.path(), pub_pem);
    let result = download_and_save_bin(
        &updater,
        format!("http://127.0.0.1:{bin_port}/bin"),
        format!("http://127.0.0.1:{sig_port}/sig"),
        "tb",
        0o755,
        None,
    );
    bin_handle.join().unwrap();
    sig_handle.join().unwrap();

    assert!(result.unwrap_err().to_string().contains("Signature verification failed"));
    // The live binary must be untouched and no .old left behind.
    assert_eq!(fs::read(dir.path().join("tb")).unwrap(), b"old-binary");
    assert!(!dir.path().join("tb.old").exists());
}

#[test]
fn test_download_and_save_bin_with_empty_user_group() {
    let (pub_pem, key) = test_keypair();
    let content = b"binary-with-ownership".to_vec();
    let sig = sign_bytes(&key, &content);
    let (bin_port, bin_handle) = serve_payload(content.clone());
    let (sig_port, sig_handle) = serve_payload(sig);
    let dir = tempfile::tempdir().unwrap();
    let updater = updater_with_key(dir.path(), pub_pem);
    let result = download_and_save_bin(
        &updater,
        format!("http://127.0.0.1:{bin_port}/bin"),
        format!("http://127.0.0.1:{sig_port}/sig"),
        "tb",
        0o755,
        Some(""),
    );
    bin_handle.join().unwrap();
    sig_handle.join().unwrap();
    assert!(result.is_ok(), "download_and_save_bin with ownership failed: {result:?}");
    assert_eq!(fs::read(dir.path().join("tb")).unwrap(), content);
}

#[test]
fn test_swap_all_rolls_back_previously_applied_targets_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a"), b"old-a").unwrap();
    fs::write(dir.path().join("b"), b"old-b").unwrap();
    let updater = create_updater(dir.path());

    let result = updater.swap_all(vec![
        (b"new-a".to_vec(), "a", 0o755, None),
        (b"new-b".to_vec(), "b", 0o755, Some("ruroco-nonexistent-test-user")),
    ]);

    assert!(result.unwrap_err().to_string().contains("rolled back"));
    assert_eq!(fs::read(dir.path().join("a")).unwrap(), b"old-a", "target a must be rolled back");
    assert_eq!(fs::read(dir.path().join("b")).unwrap(), b"old-b", "target b must be rolled back");
    assert!(!dir.path().join("a.old").exists());
    assert!(!dir.path().join("b.old").exists());
}

#[test]
fn test_swap_all_removes_fresh_targets_with_no_prior_binary_on_failure() {
    let dir = tempfile::tempdir().unwrap();
    let updater = create_updater(dir.path());

    let result = updater.swap_all(vec![
        (b"new-a".to_vec(), "a", 0o755, None),
        (b"new-b".to_vec(), "b", 0o755, Some("ruroco-nonexistent-test-user")),
    ]);

    assert!(result.is_err());
    assert!(!dir.path().join("a").exists(), "target a had no prior binary, so it's removed");
    assert!(!dir.path().join("b").exists(), "target b had no prior binary, so it's removed");
}

#[test]
fn test_create_no_bin_path_client() {
    let dir = tempfile::tempdir().unwrap();
    let bin_dir = dir.path().join(".local").join("bin");
    env::set_var("HOME", dir.path());
    let updater = Updater::create(false, None, None, false).unwrap();
    assert_eq!(updater.bin_path, bin_dir);
    assert!(bin_dir.exists());
}

#[test]
fn test_update_no_force_version_matches_skips_network() {
    let dir = tempfile::tempdir().unwrap();
    let current_version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let updater =
        Updater::create(false, Some(current_version), Some(dir.path().to_path_buf()), false)
            .unwrap();
    assert!(updater.update().is_ok());
}

#[test]
fn test_update_verifies_all_targets_before_swapping_any() {
    use crate::client::update::{CLIENT_BIN_NAME, CLIENT_UI_BIN_NAME};
    use std::env::consts::{ARCH, OS};

    let (pub_pem, key) = test_keypair();
    let tag = "v9.9.9";

    // client: valid content + matching signature.
    let client_content = b"new-client".to_vec();
    let client_sig = sign_bytes(&key, &client_content);
    // client-ui: signature does not match its content, so verification fails.
    let ui_content = b"new-client-ui".to_vec();
    let ui_sig = sign_bytes(&key, b"some-other-bytes");

    let (client_bin_port, client_bin_handle) = serve_payload(client_content);
    let (client_sig_port, client_sig_handle) = serve_payload(client_sig);
    let (ui_bin_port, ui_bin_handle) = serve_payload(ui_content);
    let (ui_sig_port, ui_sig_handle) = serve_payload(ui_sig);

    let client_bin_name = format!("client-{tag}-{ARCH}-{OS}");
    let ui_bin_name = format!("client-ui-{tag}-{ARCH}-{OS}");
    let api_json = format!(
        r#"[{{"tag_name":"{tag}","assets":[
            {{"name":"{client_bin_name}","browser_download_url":"http://127.0.0.1:{client_bin_port}/bin"}},
            {{"name":"{client_bin_name}.sig","browser_download_url":"http://127.0.0.1:{client_sig_port}/sig"}},
            {{"name":"{ui_bin_name}","browser_download_url":"http://127.0.0.1:{ui_bin_port}/bin"}},
            {{"name":"{ui_bin_name}.sig","browser_download_url":"http://127.0.0.1:{ui_sig_port}/sig"}}
        ]}}]"#
    );
    let (api_port, api_handle) = serve_payload(api_json.into_bytes());

    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(CLIENT_BIN_NAME), b"old-client").unwrap();
    fs::write(dir.path().join(CLIENT_UI_BIN_NAME), b"old-client-ui").unwrap();

    let updater = Updater {
        force: true,
        version: None,
        bin_path: dir.path().to_path_buf(),
        server: false,
        public_key_pem: pub_pem,
        releases_url: format!("http://127.0.0.1:{api_port}/releases"),
    };

    let result = updater.update();

    api_handle.join().unwrap();
    client_bin_handle.join().unwrap();
    client_sig_handle.join().unwrap();
    ui_bin_handle.join().unwrap();
    ui_sig_handle.join().unwrap();

    assert!(result.unwrap_err().to_string().contains("Signature verification failed"));
    // client's signature verified fine, but client-ui's did not: neither must be swapped in, since
    // every target is verified before any of them is written to disk.
    assert_eq!(fs::read(dir.path().join(CLIENT_BIN_NAME)).unwrap(), b"old-client");
    assert_eq!(fs::read(dir.path().join(CLIENT_UI_BIN_NAME)).unwrap(), b"old-client-ui");
    assert!(!dir.path().join(format!("{CLIENT_BIN_NAME}.old")).exists());
    assert!(!dir.path().join(format!("{CLIENT_UI_BIN_NAME}.old")).exists());
}

#[test]
fn test_is_downgrade_older_target() {
    assert_eq!(Updater::is_downgrade("v1.2.3", "v1.2.2"), Some(true));
    assert_eq!(Updater::is_downgrade("v1.2.3", "v1.1.9"), Some(true));
    assert_eq!(Updater::is_downgrade("v1.2.3", "v0.9.9"), Some(true));
}

#[test]
fn test_is_downgrade_equal_or_newer_target() {
    assert_eq!(Updater::is_downgrade("v1.2.3", "v1.2.3"), Some(false));
    assert_eq!(Updater::is_downgrade("v1.2.3", "v1.2.4"), Some(false));
    assert_eq!(Updater::is_downgrade("v1.2.3", "v1.3.0"), Some(false));
    assert_eq!(Updater::is_downgrade("v1.2.3", "v2.0.0"), Some(false));
}

#[test]
fn test_is_downgrade_malformed_tag_returns_none() {
    assert_eq!(Updater::is_downgrade("v1.2.3", "not-a-version"), None);
    assert_eq!(Updater::is_downgrade("not-a-version", "v1.2.3"), None);
    assert_eq!(Updater::is_downgrade("v1.2", "v1.2.3"), None);
}

#[test]
fn test_binary_targets_client_mode() {
    let dir = tempfile::tempdir().unwrap();
    let updater = create_updater(dir.path());
    let targets = updater.binary_targets();
    assert_eq!(targets.len(), 2);
    assert!(targets.iter().any(|(prefix, _, _, _)| *prefix == "client"));
    assert!(targets.iter().any(|(prefix, _, _, _)| *prefix == "client-ui"));
}

#[test]
fn test_binary_targets_server_mode() {
    let dir = tempfile::tempdir().unwrap();
    let updater = Updater::create(false, None, Some(dir.path().to_path_buf()), true).unwrap();
    let targets = updater.binary_targets();
    assert_eq!(targets.len(), 2);
    assert!(targets.iter().any(|(prefix, _, _, owner)| *prefix == "commander" && owner.is_none()));
    assert!(targets
        .iter()
        .any(|(prefix, _, _, owner)| *prefix == "server" && *owner == Some("ruroco")));
}
