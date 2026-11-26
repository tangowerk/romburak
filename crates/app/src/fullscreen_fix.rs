//! Workaround for a RetroArch Wayland bug: in fullscreen on an output with fractional
//! scaling (e.g. 1.25), the game picture starts oversized and off-centre, ignoring the
//! aspect ratio until the next video reinit (libretro/RetroArch#19936). Toggling
//! fullscreen off and on fixes it, so Romburak does that once shortly after the start.
//!
//! Detection needs the compositor: only Hyprland (`hyprctl monitors -j`) for now.

use romburak_core::retroarch::managed::{Managed, display::Display};
use std::time::Duration;

/// Wait before the toggle: the core must be running (the bug shows with its first frame).
const DELAY: Duration = Duration::from_millis(2500);
/// Gap between the two toggles so RetroArch applies the first one.
const GAP: Duration = Duration::from_millis(400);

/// Whether the game about to start needs the toggle.
pub fn needed(m: &Managed) -> bool {
    fullscreen(m) && fractional_hyprland()
}

/// Sends the two toggles in the background once the game runs.
pub fn schedule() {
    std::thread::spawn(|| {
        std::thread::sleep(DELAY);
        if matches!(crate::live::send("FULLSCREEN_TOGGLE"), Ok(true)) {
            std::thread::sleep(GAP);
            let _ = crate::live::send("FULLSCREEN_TOGGLE");
        }
    });
}

fn fullscreen(m: &Managed) -> bool {
    match m.display {
        Some(d) => d == Display::Fullscreen,
        // unset: RetroArch's own setting decides
        None => std::fs::read_to_string(m.cfg()).is_ok_and(|t| {
            romburak_core::retroarch::info::kv(&t)
                .any(|(k, v)| k == "video_fullscreen" && v.trim_matches('"') == "true")
        }),
    }
}

fn fractional_hyprland() -> bool {
    if !cfg!(target_os = "linux")
        || std::env::var_os("WAYLAND_DISPLAY").is_none()
        || std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none()
    {
        return false;
    }
    std::process::Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .is_some_and(|o| focused_scale_is_fractional(&o.stdout))
}

/// RetroArch opens on the focused monitor.
fn focused_scale_is_fractional(json: &[u8]) -> bool {
    let Ok(serde_json::Value::Array(monitors)) = serde_json::from_slice(json) else {
        return false;
    };
    monitors
        .iter()
        .find(|m| m["focused"].as_bool() == Some(true))
        .and_then(|m| m["scale"].as_f64())
        .is_some_and(|s| s.fract().abs() > 1e-3)
}

#[cfg(test)]
mod tests {
    use super::focused_scale_is_fractional as frac;

    #[test]
    fn focused_monitor_decides() {
        let j = br#"[{"name":"eDP-1","scale":1.25,"focused":false},
                     {"name":"DP-2","scale":1.0,"focused":true}]"#;
        assert!(!frac(j));
        let j = br#"[{"name":"eDP-1","scale":1.25,"focused":true}]"#;
        assert!(frac(j));
        let j = br#"[{"name":"eDP-1","scale":2.0,"focused":true}]"#;
        assert!(!frac(j));
        assert!(!frac(b"not json"));
    }
}
