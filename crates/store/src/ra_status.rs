//! RetroAchievements connection status for the settings page: is the login token still
//! valid (`dorequest.php?r=login2`, like RetroArch at start), does the Web API key work
//! (`API_GetUserSummary`: rank, points, recent unlocks), what the last sync matched, and
//! the in-game options the user chose.

use crate::{Result, Store, http_get};
use romburak_core::retroarch::managed::cheevos::{OPTIONS, option_default};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;

const LOGIN: &str = "https://retroachievements.org/dorequest.php";
const SUMMARY: &str = "https://retroachievements.org/API/API_GetUserSummary.php";
const CONSOLES: &str = "https://retroachievements.org/API/API_GetConsoleIDs.php";
const MEDIA: &str = "https://media.retroachievements.org";
const OPTION: &str = "ra.opt.";

/// What the login token check returns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all(deserialize = "PascalCase"))]
pub struct RaLogin {
    pub user: String,
    #[serde(default)]
    pub avatar_url: String,
    /// hardcore points
    #[serde(default)]
    pub score: u64,
    #[serde(default)]
    pub softcore_score: u64,
}

/// Checks the stored token (no password involved); error text for the UI on failure.
pub fn ra_check_login(user: &str, token: &str) -> io::Result<RaLogin> {
    let reply = ureq::post(LOGIN)
        .header("User-Agent", "romburak")
        .send_form([("r", "login2"), ("u", user), ("t", token)]);
    let body = match reply {
        Ok(mut r) => r.body_mut().read_to_string().map_err(io::Error::other)?,
        Err(ureq::Error::StatusCode(401)) => {
            return Err(io::Error::other("login expired – log in again"));
        }
        Err(e) => return Err(io::Error::other(e)),
    };
    parse_login(&body)
}

fn parse_login(body: &str) -> io::Result<RaLogin> {
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Reply {
        success: bool,
        error: Option<String>,
        #[serde(flatten)]
        login: Option<RaLogin>,
    }
    let r: Reply = serde_json::from_str(body).map_err(io::Error::other)?;
    match (r.success, r.login) {
        (true, Some(l)) => Ok(l),
        _ => Err(io::Error::other(
            r.error.unwrap_or_else(|| "login check failed".into()),
        )),
    }
}

/// One recent unlock of the user.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RaUnlock {
    pub game: u64,
    pub game_title: String,
    pub title: String,
    pub points: u32,
    /// badge image URL
    pub badge: String,
    /// RA date text (UTC), `YYYY-MM-DD hh:mm:ss`
    pub date: String,
    pub hardcore: bool,
}

/// Profile from the Web API (proves the key works).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RaSummary {
    pub user: String,
    pub avatar: String,
    pub points: u64,
    pub softcore_points: u64,
    pub rank: Option<u64>,
    pub total_ranked: Option<u64>,
    pub member_since: String,
    /// what the user is doing in their last game, per RA
    pub rich_presence: String,
    /// newest first
    pub recent: Vec<RaUnlock>,
}

/// Error text for a key RA refuses (HTTP 401).
pub const KEY_REJECTED: &str = "Web API key rejected – check it on retroachievements.org";

fn key_error(e: io::Error) -> io::Error {
    if e.to_string().contains("401") {
        io::Error::other(KEY_REJECTED)
    } else {
        e
    }
}

/// Checks a Web API key on its own (no user name needed); returns the number of active consoles.
pub fn ra_check_key(api_key: &str) -> io::Result<usize> {
    let body = http_get(&format!("{CONSOLES}?y={api_key}&a=1")).map_err(key_error)?;
    let list: Vec<serde_json::Value> = serde_json::from_slice(&body).map_err(io::Error::other)?;
    Ok(list.len())
}

/// Profile and the newest unlocks (`n` at most) of `user`.
pub fn ra_summary(user: &str, api_key: &str, n: usize) -> io::Result<RaSummary> {
    let url = format!("{SUMMARY}?u={user}&y={api_key}&g=5&a={n}");
    let body = http_get(&url).map_err(key_error)?;
    parse_summary(&body, n)
}

fn parse_summary(body: &[u8], n: usize) -> io::Result<RaSummary> {
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Summary {
        user: String,
        #[serde(default)]
        user_pic: String,
        #[serde(default)]
        total_points: u64,
        #[serde(default)]
        total_softcore_points: u64,
        rank: Option<u64>,
        total_ranked: Option<u64>,
        #[serde(default)]
        member_since: String,
        #[serde(default)]
        rich_presence_msg: String,
        #[serde(default)]
        recent_achievements: Recent,
    }
    // RA sends `[]` instead of `{}` when there are none
    #[derive(Deserialize, Default)]
    #[serde(untagged)]
    enum Recent {
        Games(HashMap<String, HashMap<String, Unlock>>),
        #[default]
        #[allow(dead_code)]
        None,
        #[allow(dead_code)]
        Empty(Vec<serde_json::Value>),
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Unlock {
        #[serde(rename = "GameID")]
        game_id: u64,
        #[serde(default)]
        game_title: String,
        title: String,
        #[serde(default)]
        points: u32,
        #[serde(default)]
        badge_name: String,
        #[serde(default)]
        date_awarded: String,
        #[serde(default)]
        hardcore_achieved: serde_json::Value,
    }
    let s: Summary = serde_json::from_slice(body).map_err(io::Error::other)?;
    let mut recent: Vec<RaUnlock> = match s.recent_achievements {
        Recent::Games(games) => games
            .into_values()
            .flat_map(HashMap::into_values)
            .map(|u| RaUnlock {
                game: u.game_id,
                game_title: u.game_title,
                title: u.title,
                points: u.points,
                badge: format!("{MEDIA}/Badge/{}.png", u.badge_name),
                date: u.date_awarded,
                // 1 / "1" / true depending on the server
                hardcore: matches!(u.hardcore_achieved, serde_json::Value::Bool(true))
                    || u.hardcore_achieved.as_u64() == Some(1)
                    || u.hardcore_achieved.as_str() == Some("1"),
            })
            .collect(),
        _ => Vec::new(),
    };
    recent.sort_by(|a, b| b.date.cmp(&a.date));
    recent.truncate(n);
    let avatar = if s.user_pic.starts_with('/') {
        format!("{MEDIA}{}", s.user_pic)
    } else {
        s.user_pic
    };
    Ok(RaSummary {
        user: s.user,
        avatar,
        points: s.total_points,
        softcore_points: s.total_softcore_points,
        rank: s.rank,
        total_ranked: s.total_ranked,
        member_since: s.member_since,
        rich_presence: s.rich_presence_msg,
        recent,
    })
}

/// What the last sync matched.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct RaCounts {
    /// consoles with downloaded game lists
    pub consoles: u64,
    /// RA games in those lists
    pub games: u64,
    /// hashed library files
    pub files: u64,
    /// of those, files of a game with achievements
    pub files_with: u64,
}

impl Store {
    pub fn ra_counts(&self) -> Result<RaCounts> {
        Ok(self.conn.query_row(
            "SELECT (SELECT count(*) FROM ra_console), (SELECT count(*) FROM ra_game),
                    (SELECT count(*) FROM ra_file),
                    (SELECT count(*) FROM ra_file f JOIN ra_hash h ON h.hash = f.hash
                       JOIN ra_game g ON g.id = h.game WHERE g.achievements > 0)",
            [],
            |r| {
                Ok(RaCounts {
                    consoles: r.get(0)?,
                    games: r.get(1)?,
                    files: r.get(2)?,
                    files_with: r.get(3)?,
                })
            },
        )?)
    }

    /// In-game options with their current value (chosen, else RetroArch's default) and
    /// whether the user chose it.
    pub fn ra_options(&self) -> Result<Vec<(&'static str, bool, bool)>> {
        OPTIONS
            .iter()
            .map(|(id, _, default)| {
                let set = self.setting(&format!("{OPTION}{id}"))?;
                Ok((
                    *id,
                    set.as_deref().map_or(*default, |v| v == "true"),
                    set.is_some(),
                ))
            })
            .collect()
    }

    /// Options the user chose, for the config.
    pub(crate) fn ra_chosen_options(&self) -> Result<Vec<(String, bool)>> {
        Ok(self
            .ra_options()?
            .into_iter()
            .filter(|o| o.2)
            .map(|(id, on, _)| (id.to_owned(), on))
            .collect())
    }

    /// Sets an option; `false` for unknown ids.
    pub fn set_ra_option(&self, id: &str, on: bool) -> Result<bool> {
        if option_default(id).is_none() {
            return Ok(false);
        }
        self.set_setting(&format!("{OPTION}{id}"), &on.to_string())?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_check_reply() {
        let ok = r#"{"Success":true,"User":"kc","AvatarUrl":"https://x/kc.png","Score":5,"SoftcoreScore":10,"Messages":0}"#;
        let l = parse_login(ok).unwrap();
        assert_eq!((l.user.as_str(), l.score, l.softcore_score), ("kc", 5, 10));
        let bad = r#"{"Success":false,"Error":"Invalid user/token combination.","Code":"invalid_credentials"}"#;
        assert!(parse_login(bad).unwrap_err().to_string().contains("token"));
    }

    #[test]
    fn summary_with_and_without_unlocks() {
        let body = br#"{"User":"kc","UserPic":"/UserPic/kc.png","TotalPoints":8317,"TotalSoftcorePoints":3,
            "Rank":4616,"TotalRanked":45654,"MemberSince":"2021-12-20 03:13:20","RichPresenceMsg":"L=1",
            "RecentAchievements":{"15758":{"1":{"ID":1,"GameID":15758,"GameTitle":"Crazy Kong","Title":"Old",
              "Points":5,"BadgeName":"9","IsAwarded":"1","DateAwarded":"2023-03-09 08:20:34","HardcoreAchieved":0},
              "2":{"ID":2,"GameID":15758,"GameTitle":"Crazy Kong","Title":"New","Points":25,"BadgeName":"325551",
              "IsAwarded":"1","DateAwarded":"2023-03-10 08:20:34","HardcoreAchieved":1}}}}"#;
        let s = parse_summary(body, 5).unwrap();
        assert_eq!(
            s.avatar,
            "https://media.retroachievements.org/UserPic/kc.png"
        );
        assert_eq!((s.rank, s.total_ranked), (Some(4616), Some(45654)));
        assert_eq!(s.recent.len(), 2);
        assert_eq!(s.recent[0].title, "New");
        assert!(s.recent[0].hardcore && !s.recent[1].hardcore);
        assert!(s.recent[0].badge.ends_with("/Badge/325551.png"));
        let none = br#"{"User":"kc","TotalPoints":0,"Rank":null,"RecentAchievements":[]}"#;
        let s = parse_summary(none, 5).unwrap();
        assert!(s.recent.is_empty() && s.rank.is_none());
    }
}
