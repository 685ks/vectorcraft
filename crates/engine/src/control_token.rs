//! The control channel's shared secret (`docs/control-protocol.md` → Authentication).
//!
//! The desktop app's loopback control server (`vectorcraft --control PORT`) refuses every request on
//! a connection until its first frame is `{"method": "auth", "params": {"token": "<64 hex>"}}` with
//! this token. Any local process could otherwise drive the app and read and write files as the user.
//!
//! The token is 32 random bytes as 64 lowercase hexadecimal characters. The app takes it from
//! `$VECTORCRAFT_CONTROL_TOKEN`, or reads it from a token file (`--control-token-file PATH`,
//! `$VECTORCRAFT_CONTROL_TOKEN_FILE`, or [`default_token_file`] in the settings folder), creating that
//! file (owner-only, 0600 on Unix) with a new token when it does not exist. Clients
//! (`vectorcraft-cli run|mcp --connect`) look in the same places, so one user's app and tools agree
//! without configuration. Ported from PhotoCraft's control-channel security
//! (crates/automation/src/security.rs, MIT OR Apache-2.0).

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The first request on every connection must use this method.
pub const AUTH_METHOD: &str = "auth";
/// The reply to a connection whose first frame is not a valid `auth`.
pub const AUTH_REQUIRED: &str = "authentication required";
/// Environment variable holding the token itself.
pub const TOKEN_ENV: &str = "VECTORCRAFT_CONTROL_TOKEN";
/// Environment variable naming the token file.
pub const TOKEN_FILE_ENV: &str = "VECTORCRAFT_CONTROL_TOKEN_FILE";
/// The token file's name in the settings folder.
pub const TOKEN_FILE_NAME: &str = "control-token";

const TOKEN_BYTES: usize = 32;
const TOKEN_HEX_LEN: usize = TOKEN_BYTES * 2;

/// `<settings folder>/control-token`, if there is a settings folder: `$VECTORCRAFT_CONFIG_DIR`, else
/// the platform's VectorCraft settings folder (`~/Library/Application Support/VectorCraft`,
/// `%APPDATA%\\VectorCraft`, `$XDG_CONFIG_HOME/vectorcraft` or `~/.config/vectorcraft`). A portable
/// copy keeps its settings beside the app, but its token stays here, where the command-line tools
/// look for it.
pub fn default_token_file() -> Option<PathBuf> {
    config_dir(std::env::consts::OS, |key| std::env::var_os(key)).map(|d| d.join(TOKEN_FILE_NAME))
}

fn config_dir(os: &str, env: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    if let Some(root) = env("VECTORCRAFT_CONFIG_DIR").filter(|r| !r.is_empty()) {
        return Some(PathBuf::from(root));
    }
    match os {
        "macos" => env("HOME").map(|p| PathBuf::from(p).join("Library/Application Support/VectorCraft")),
        "windows" => env("APPDATA").map(|p| PathBuf::from(p).join("VectorCraft")),
        _ => {
            env("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| env("HOME").map(|p| PathBuf::from(p).join(".config"))).map(|p| p.join("vectorcraft"))
        }
    }
}

/// A new random token.
pub fn generate_token() -> Result<String, String> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| format!("cannot generate a control token: {e}"))?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut token = String::with_capacity(TOKEN_HEX_LEN);
    for byte in bytes {
        token.push(char::from(HEX[usize::from(byte >> 4)]));
        token.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(token)
}

/// Accept only the fixed-width hexadecimal form [`generate_token`] makes.
pub fn validate_token(token: &str) -> Result<(), String> {
    if token.len() != TOKEN_HEX_LEN || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("the control token must be exactly 64 hexadecimal characters".into());
    }
    Ok(())
}

/// Compare two tokens without stopping at the first different byte. Hex letters match in either
/// case; an empty or wrong-length token never matches.
pub fn token_matches(expected: &str, supplied: &str) -> bool {
    if expected.len() != TOKEN_HEX_LEN || supplied.len() != TOKEN_HEX_LEN {
        return false;
    }
    expected.bytes().zip(supplied.bytes()).fold(0u8, |different, (a, b)| different | (a.to_ascii_lowercase() ^ b.to_ascii_lowercase())) == 0
}

/// The authentication frame a client sends first.
pub fn auth_request(token: &str) -> Value {
    json!({"id": "auth", "method": AUTH_METHOD, "params": {"token": token}})
}

/// Whether a connection's first request (`method`, `params`) authenticates with `expected`.
pub fn authenticates(expected: &str, method: &str, params: &Value) -> bool {
    let supplied = params.get("token").and_then(Value::as_str).unwrap_or("");
    method == AUTH_METHOD && token_matches(expected, supplied)
}

fn read_token_file(path: &Path) -> Result<String, String> {
    let shown = path.display();
    let mut file = std::fs::File::open(path).map_err(|e| format!("{shown}: {e}"))?;
    let meta = file.metadata().map_err(|e| format!("{shown}: {e}"))?;
    if !meta.is_file() {
        return Err(format!("{shown}: the control token file is not a regular file"));
    }
    // A token another local user can read is no secret: refuse it rather than trust it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.mode() & 0o077 != 0 {
            return Err(format!(
                "{shown}: the control token file is readable by group or others (mode {:o}); restrict it to its owner (chmod 600)",
                meta.mode() & 0o777
            ));
        }
    }
    let text = std::io::read_to_string(&mut file).map_err(|e| format!("{shown}: {e}"))?;
    let token = text.trim().to_ascii_lowercase();
    validate_token(&token).map_err(|e| format!("{shown}: {e}"))?;
    Ok(token)
}

fn create_token_file(path: &Path, token: &str) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    writeln!(file, "{token}").map_err(|e| format!("{}: {e}", path.display()))
}

/// The token the server uses: `supplied` (validated) when given, else the one in `token_file`
/// (created with a new token when missing), else a new one that only this process knows.
pub fn server_token(supplied: Option<&str>, token_file: Option<&Path>) -> Result<String, String> {
    if let Some(token) = supplied {
        validate_token(token)?;
        return Ok(token.to_ascii_lowercase());
    }
    let Some(path) = token_file else {
        return generate_token();
    };
    if path.exists() {
        return read_token_file(path);
    }
    let token = generate_token()?;
    match create_token_file(path, &token) {
        Ok(()) => Ok(token),
        // Another process created it first: use theirs.
        Err(_) if path.exists() => read_token_file(path),
        Err(e) => Err(e),
    }
}

/// The token a client sends: `supplied` when given, else the one in `token_file`. Clients never
/// make up a token; `None` when there is none to read.
pub fn client_token(supplied: Option<&str>, token_file: Option<&Path>) -> Result<Option<String>, String> {
    if let Some(token) = supplied {
        validate_token(token)?;
        return Ok(Some(token.to_ascii_lowercase()));
    }
    match token_file {
        Some(path) if path.exists() => read_token_file(path).map(Some),
        _ => Ok(None),
    }
}

/// The token file named by the environment, or the default one.
pub fn token_file_from_env() -> Option<PathBuf> {
    std::env::var_os(TOKEN_FILE_ENV).filter(|v| !v.is_empty()).map(PathBuf::from).or_else(default_token_file)
}

/// The token given in the environment, if any.
pub fn token_from_env() -> Option<String> {
    std::env::var(TOKEN_ENV).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vectorcraft-token-{name}-{}-{}", std::process::id(), generate_token().unwrap()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_token_file_lives_in_the_settings_folder() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| std::ffi::OsString::from(v))
        };
        assert_eq!(config_dir("linux", env(&[("VECTORCRAFT_CONFIG_DIR", "/iso"), ("HOME", "/h")])), Some(PathBuf::from("/iso")));
        assert_eq!(config_dir("linux", env(&[("HOME", "/h")])), Some(PathBuf::from("/h/.config/vectorcraft")));
        assert_eq!(config_dir("linux", env(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")])), Some(PathBuf::from("/x/vectorcraft")));
        assert_eq!(config_dir("macos", env(&[("HOME", "/h")])), Some(PathBuf::from("/h/Library/Application Support/VectorCraft")));
        assert_eq!(config_dir("windows", env(&[("APPDATA", "C:/a")])), Some(PathBuf::from("C:/a").join("VectorCraft")));
        assert_eq!(config_dir("linux", env(&[])), None);
    }

    #[test]
    fn generated_tokens_are_64_hex_and_differ() {
        let a = generate_token().unwrap();
        let b = generate_token().unwrap();
        validate_token(&a).unwrap();
        assert_ne!(a, b);
        assert!(validate_token("abc").is_err());
        assert!(validate_token(&"g".repeat(64)).is_err());
    }

    #[test]
    fn token_comparison_rejects_every_single_byte_difference() {
        let token = generate_token().unwrap();
        assert!(token_matches(&token, &token));
        assert!(token_matches(&token, &token.to_ascii_uppercase()), "hex letters match in either case");
        for i in 0..token.len() {
            let mut bytes = token.clone().into_bytes();
            bytes[i] = if bytes[i] == b'0' { b'1' } else { b'0' };
            let other = String::from_utf8(bytes).unwrap();
            assert!(!token_matches(&token, &other), "difference at {i} accepted");
        }
        assert!(!token_matches(&token, &token[..63]));
        assert!(!token_matches(&token, &format!("{token}0")));
        assert!(!token_matches(&token, ""));
        assert!(!token_matches("", ""), "an empty expected token never matches");
    }

    #[test]
    fn only_an_auth_frame_with_the_token_authenticates() {
        let token = generate_token().unwrap();
        assert!(authenticates(&token, AUTH_METHOD, &json!({"token": token})));
        assert!(!authenticates(&token, AUTH_METHOD, &json!({"token": "f".repeat(64)})));
        assert!(!authenticates(&token, AUTH_METHOD, &json!({})));
        assert!(!authenticates(&token, "ui.inspect", &json!({"token": token})), "the token on another method is not auth");
        let req = auth_request(&token);
        assert_eq!(req["method"], AUTH_METHOD);
        assert!(authenticates(&token, req["method"].as_str().unwrap(), &req["params"]));
    }

    #[test]
    fn a_missing_token_file_is_created_owner_only_and_read_back() {
        let dir = scratch("create");
        let path = dir.join("nested").join(TOKEN_FILE_NAME);
        let token = server_token(None, Some(&path)).unwrap();
        validate_token(&token).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "created token file mode {mode:o}");
        }
        assert_eq!(server_token(None, Some(&path)).unwrap(), token, "the app keeps its token across restarts");
        assert_eq!(client_token(None, Some(&path)).unwrap(), Some(token), "a client reads the same token");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn a_token_file_others_can_read_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("mode");
        let path = dir.join(TOKEN_FILE_NAME);
        std::fs::write(&path, format!("{}\n", "a".repeat(64))).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(server_token(None, Some(&path)).unwrap_err().contains("readable by group or others"));
        assert!(client_token(None, Some(&path)).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_supplied_token_wins_and_a_client_never_makes_one_up() {
        let token = "AB".repeat(32);
        assert_eq!(server_token(Some(&token), None).unwrap(), token.to_ascii_lowercase());
        assert!(server_token(Some("short"), None).is_err());
        let dir = scratch("client");
        assert_eq!(client_token(None, Some(&dir.join("absent"))).unwrap(), None);
        assert_eq!(client_token(None, None).unwrap(), None);
        // a server without a file still gets a token, known only to it
        validate_token(&server_token(None, None).unwrap()).unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }
}
