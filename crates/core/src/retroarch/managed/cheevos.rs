//! RetroAchievements in the managed RetroArch: login (user + token, never the password)
//! and hardcore mode. The token comes from `dorequest.php?r=login2`. A few in-game options
//! ([`OPTIONS`]) are written only once the user chose them; unset keeps RetroArch's default.

/// In-game achievement options: (id, `retroarch.cfg` keys, RetroArch default).
pub const OPTIONS: &[(&str, &[&str], bool)] = &[
    (
        "unlock_popup",
        &["cheevos_visibility_unlock", "cheevos_visibility_mastery"],
        true,
    ),
    ("unlock_sound", &["cheevos_unlock_sound_enable"], false),
    ("unlock_screenshot", &["cheevos_auto_screenshot"], false),
    (
        "leaderboards",
        &[
            "cheevos_visibility_lboard_start",
            "cheevos_visibility_lboard_submit",
            "cheevos_visibility_lboard_cancel",
            "cheevos_visibility_lboard_trackers",
        ],
        true,
    ),
    (
        "challenge_indicators",
        &["cheevos_challenge_indicators"],
        true,
    ),
    ("menu_badges", &["cheevos_badges_enable"], false),
    ("encore", &["cheevos_start_active"], false),
];

/// RetroArch's default of option `id`, `None` if unknown.
pub fn option_default(id: &str) -> Option<bool> {
    OPTIONS.iter().find(|o| o.0 == id).map(|o| o.2)
}

/// Achievement settings romburak writes into the config.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cheevos {
    /// (user name, login token); `None` = logged out.
    pub login: Option<(String, String)>,
    /// Hardcore: no savestates, rewind or cheats; unlocks count double on the site.
    pub hardcore: bool,
    /// Options the user set (id from [`OPTIONS`], on); others stay as RetroArch has them.
    pub options: Vec<(String, bool)>,
}

impl Cheevos {
    /// `retroarch.cfg` keys; logged out disables achievements and clears the credentials.
    pub fn keys(&self) -> Vec<(&'static str, String)> {
        let (enable, user, token) = match &self.login {
            Some((u, t)) => (true, u.as_str(), t.as_str()),
            None => (false, "", ""),
        };
        let mut keys = vec![
            ("cheevos_enable", enable.to_string()),
            ("cheevos_username", user.into()),
            ("cheevos_token", token.into()),
            ("cheevos_password", String::new()),
            ("cheevos_hardcore_mode_enable", self.hardcore.to_string()),
        ];
        for (id, on) in &self.options {
            if let Some((_, cfg, _)) = OPTIONS.iter().find(|o| o.0 == id) {
                keys.extend(cfg.iter().map(|k| (*k, on.to_string())));
            }
        }
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_writes_token_never_password() {
        let c = Cheevos {
            login: Some(("kc".into(), "tok".into())),
            hardcore: true,
            options: Vec::new(),
        };
        let keys = c.keys();
        assert!(keys.contains(&("cheevos_enable", "true".into())));
        assert!(keys.contains(&("cheevos_token", "tok".into())));
        assert!(keys.contains(&("cheevos_password", String::new())));
        assert!(keys.contains(&("cheevos_hardcore_mode_enable", "true".into())));
        let off = Cheevos::default().keys();
        assert!(off.contains(&("cheevos_enable", "false".into())));
        assert!(off.contains(&("cheevos_token", String::new())));
    }

    #[test]
    fn only_chosen_options_are_written() {
        let c = Cheevos {
            options: vec![("leaderboards".into(), false), ("unknown".into(), true)],
            ..Cheevos::default()
        };
        let keys = c.keys();
        assert!(keys.contains(&("cheevos_visibility_lboard_start", "false".into())));
        assert!(keys.contains(&("cheevos_visibility_lboard_trackers", "false".into())));
        assert!(
            !keys
                .iter()
                .any(|(k, _)| *k == "cheevos_unlock_sound_enable")
        );
        assert_eq!(keys.len(), 5 + 4);
        assert_eq!(option_default("unlock_sound"), Some(false));
    }
}
