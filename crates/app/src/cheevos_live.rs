//! Settings → Achievements: live connection status (login token, Web API key, profile,
//! recent unlocks), what the last sync matched, and RetroArch's in-game achievement options.

use crate::cheevos::KEY;
use crate::commands::{CmdResult, err, open_store};
use romburak_store::{RaCounts, RaLogin, RaSummary};
use serde::Serialize;

/// Setting: unix seconds of the last call that proved login and key work.
const LAST_OK: &str = "ra.last_ok";

/// One connection light: `ok` with data, or the error; `None` = not set up.
#[derive(Serialize)]
pub struct Check<T> {
    ok: Option<T>,
    error: Option<String>,
}

impl<T> Check<T> {
    fn from(r: std::io::Result<T>) -> Self {
        match r {
            Ok(v) => Check {
                ok: Some(v),
                error: None,
            },
            Err(e) => Check {
                ok: None,
                error: Some(e.to_string()),
            },
        }
    }
}

fn fine<T>(c: &Option<Check<T>>) -> bool {
    c.as_ref().is_some_and(|c| c.ok.is_some())
}

#[derive(Serialize)]
pub struct Live {
    /// `None` = not logged in
    login: Option<Check<RaLogin>>,
    /// `None` = no Web API key (or no user name to ask for)
    key: Option<Check<RaSummary>>,
    /// unix seconds of the last fully successful check (also from before)
    last_ok: Option<i64>,
    counts: RaCounts,
}

/// Asks RetroAchievements now (two requests): is the login token valid, does the key work.
#[tauri::command]
pub async fn cheevos_live() -> CmdResult<Live> {
    tauri::async_runtime::spawn_blocking(move || {
        let (store, _) = open_store()?;
        let account = store.ra_account().map_err(err)?;
        let key = store.setting(KEY).map_err(err)?.filter(|k| !k.is_empty());
        let login = account
            .as_ref()
            .map(|(u, t)| Check::from(romburak_store::ra_check_login(u, t)));
        let user = account.as_ref().map(|(u, _)| u.clone());
        let summary = match (&key, &user) {
            (Some(k), Some(u)) => Some(Check::from(romburak_store::ra_summary(u, k, 5))),
            _ => None,
        };
        if fine(&login) && fine(&summary) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(err)?
                .as_secs();
            store.set_setting(LAST_OK, &now.to_string()).map_err(err)?;
        }
        Ok(Live {
            login,
            key: summary,
            last_ok: store
                .setting(LAST_OK)
                .map_err(err)?
                .and_then(|s| s.parse().ok()),
            counts: store.ra_counts().map_err(err)?,
        })
    })
    .await
    .map_err(err)?
}

#[derive(Serialize)]
pub struct CheevosOption {
    id: &'static str,
    on: bool,
    /// false = RetroArch's default, never written
    chosen: bool,
}

#[tauri::command]
pub fn cheevos_options() -> CmdResult<Vec<CheevosOption>> {
    let (store, _) = open_store()?;
    Ok(store
        .ra_options()
        .map_err(err)?
        .into_iter()
        .map(|(id, on, chosen)| CheevosOption { id, on, chosen })
        .collect())
}

/// Saves an option and rewrites the managed RetroArch config (applies at the next start).
#[tauri::command]
pub fn cheevos_set_option(id: String, on: bool) -> CmdResult<()> {
    let (store, _) = open_store()?;
    if !store.set_ra_option(&id, on).map_err(err)? {
        return Err(format!("unknown option {id}"));
    }
    crate::managed_ra::rewrite_config(&store)
}

/// Opens retroachievements.org's settings page (where the Web API key is) in the browser.
#[tauri::command]
pub fn cheevos_open_settings() -> CmdResult<()> {
    const URL: &str = "https://retroachievements.org/settings";
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", URL]);
        c
    } else {
        let mut c = std::process::Command::new(if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        });
        c.arg(URL);
        c
    };
    cmd.spawn().map(drop).map_err(err)
}
