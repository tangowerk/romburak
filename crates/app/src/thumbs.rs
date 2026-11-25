//! Thumbnails from thumbnails.libretro.com (GitHub mirror as fallback), cached on disk
//! (misses as `.miss` markers, retried after a week).
use crate::commands::{CmdResult, err, open_store};
use romburak_core::thumbnail::{
    BASE_URL, Kind, best_match, cache_path, github_url, parse_listing, url,
};
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use tauri::ipc::Response;

const MAX_BYTES: u64 = 8 << 20;
/// A miss is retried after this long (the server or the mirror may have gained the image).
const MISS_TTL_SECS: u64 = 7 * 86400;
/// Setting: `"0"` disables downloads (cached images are still shown).
const ONLINE_KEY: &str = "thumbs_online";

fn online() -> CmdResult<bool> {
    let (store, _) = open_store()?;
    Ok(store.setting(ONLINE_KEY).map_err(err)?.as_deref() != Some("0"))
}

#[tauri::command]
pub async fn thumbs_online_get() -> CmdResult<bool> {
    online()
}

#[tauri::command]
pub async fn thumbs_online_set(on: bool) -> CmdResult<()> {
    let (store, _) = open_store()?;
    store
        .set_setting(ONLINE_KEY, if on { "1" } else { "0" })
        .map_err(err)
}

fn cache_root() -> CmdResult<PathBuf> {
    let base = romburak_core::paths::cache().ok_or("no cache directory")?;
    Ok(base.join("thumbnails"))
}

fn fetch(system: &str, name: &str, kind: Kind) -> CmdResult<Option<Vec<u8>>> {
    let path = cache_path(&cache_root()?, system, name, kind);
    if let Ok(b) = std::fs::read(&path) {
        return Ok(Some(b));
    }
    let miss = path.with_extension("miss");
    if fresh(&miss, MISS_TTL_SECS) || !online()? {
        return Ok(None);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(err)?;
    }
    // the libretro server first (with its listing for near names); if it has nothing or
    // is down, the GitHub mirror under the exact name (an error there is no miss)
    let server = || -> CmdResult<Option<Vec<u8>>> {
        if let Some(b) = download(&url(system, name, kind))? {
            return Ok(Some(b));
        }
        match fuzzy_name(system, name, kind, &path)? {
            Some(other) => download(&url(system, &other, kind)),
            None => Ok(None),
        }
    };
    let bytes = match server().ok().flatten() {
        Some(b) => b,
        None => download(&github_url(system, name, kind))?.unwrap_or_default(),
    };
    if bytes.is_empty() {
        std::fs::write(&miss, b"").map_err(err)?;
        return Ok(None);
    }
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &bytes).map_err(err)?;
    std::fs::rename(&tmp, &path).map_err(err)?;
    Ok(Some(bytes))
}

/// Whether `path` exists and was written less than `secs` ago.
fn fresh(path: &Path, secs: u64) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .is_ok_and(|t| t.elapsed().is_ok_and(|d| d.as_secs() < secs))
}

/// Body of `url`, or `None` on 404.
fn download(url: &str) -> CmdResult<Option<Vec<u8>>> {
    let body = match ureq::get(url).call() {
        Ok(r) => r.into_body(),
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(e) => return Err(err(e)),
    };
    let mut bytes = Vec::new();
    body.into_reader()
        .take(MAX_BYTES)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    Ok(Some(bytes))
}

/// Server name with the same title (region/tags ignored); directory listing cached 30 days.
fn fuzzy_name(system: &str, name: &str, kind: Kind, path: &Path) -> CmdResult<Option<String>> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // One listing download per directory, even with many tiles loading at once.
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let index = path.with_file_name(".index");
    let names: Vec<String> = if fresh(&index, 30 * 86400) {
        std::fs::read_to_string(&index)
            .map_err(err)?
            .lines()
            .map(str::to_owned)
            .collect()
    } else {
        let dir_url = url(system, "x", kind);
        let dir_url = &dir_url[..dir_url.rfind('/').unwrap_or(BASE_URL.len()) + 1];
        let html = download(dir_url)?.unwrap_or_default();
        let names = parse_listing(&String::from_utf8_lossy(&html));
        // an empty listing means the server is down or moved, not that it has no images
        if !names.is_empty() {
            std::fs::write(&index, names.join("\n")).map_err(err)?;
        }
        names
    };
    Ok(best_match(name, &names).map(str::to_owned))
}

/// PNG bytes of a thumbnail (`kind`: boxart/snap/title); empty if the server has none.
#[tauri::command]
pub async fn thumbnail(system: String, name: String, kind: String) -> CmdResult<Response> {
    let kind = Kind::parse(&kind).ok_or("unknown thumbnail kind")?;
    tauri::async_runtime::spawn_blocking(move || {
        Ok(Response::new(
            fetch(&system, &name, kind)?.unwrap_or_default(),
        ))
    })
    .await
    .map_err(err)?
}

/// Shader preview `<id>.webp` / `<id>-detail.webp` from the repo's `shader-thumbs/` (content-addressed,
/// so a cached file never goes stale); `None` if the id is malformed, missing or offline.
fn shader_thumb(file: &str) -> CmdResult<Option<Vec<u8>>> {
    let id = file.strip_suffix(".webp").unwrap_or(file);
    let hash = id.strip_suffix("-detail").unwrap_or(id);
    if hash.len() != 12 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) || file == id {
        return Ok(None);
    }
    let base = romburak_core::paths::cache().ok_or("no cache directory")?;
    let path = base.join("shader-thumbs").join(file);
    if let Ok(b) = std::fs::read(&path) {
        return Ok(Some(b));
    }
    if !online()? {
        return Ok(None);
    }
    let Some(bytes) = download(&format!("{SHADER_THUMBS_URL}/{file}"))? else {
        return Ok(None);
    };
    std::fs::create_dir_all(base.join("shader-thumbs")).map_err(err)?;
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &bytes).map_err(err)?;
    std::fs::rename(&tmp, &path).map_err(err)?;
    Ok(Some(bytes))
}

const SHADER_THUMBS_URL: &str =
    "https://raw.githubusercontent.com/tangowerk/romburak/main/shader-thumbs";

/// `thumb://localhost/<kind>/<system>/<name>`, `shader/<id>[-detail].webp` or `state/<screenshot path>` (one percent-encoded segment, as built by
/// `convertFileSrc`): PNG, or 404 so the UI shows its placeholder. Lets the webview lazy-load
/// and cache grid tiles itself.
pub fn protocol(request: tauri::http::Request<Vec<u8>>, responder: tauri::UriSchemeResponder) {
    let path = request.uri().path().trim_start_matches('/').to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let decoded = percent_encoding::percent_decode_str(&path).decode_utf8_lossy();
        let mut parts = decoded.splitn(3, '/');
        let mut mime = "image/png";
        let found = if let Some(file) = decoded.strip_prefix("state/") {
            crate::states::screenshot(file)
        } else if let Some(file) = decoded.strip_prefix("shader/") {
            mime = "image/webp";
            shader_thumb(file).ok().flatten()
        } else {
            match (
                parts.next().and_then(Kind::parse),
                parts.next(),
                parts.next(),
            ) {
                (Some(kind), Some(system), Some(name)) => fetch(system, name, kind).ok().flatten(),
                _ => None,
            }
        };
        let res = tauri::http::Response::builder()
            .header("Access-Control-Allow-Origin", "*")
            .header("Cache-Control", "max-age=3600");
        let res = match found {
            Some(img) => res.header("Content-Type", mime).body(img),
            None => res.status(404).body(Vec::new()),
        };
        responder.respond(res.unwrap_or_default());
    });
}
