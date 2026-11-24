//! History of the shader presets from libretro/slang-shaders (`data/shader-index.json`, built
//! by `scripts/shader-index.py`): when a preset was added, last changed, and who added it.
//! Keyed by the path below `shaders_slang`; presets newer than the snapshot have no entry.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShaderMeta {
    /// `YYYY-MM-DD`
    pub added: String,
    /// `YYYY-MM-DD` of the last commit touching the preset file.
    pub changed: String,
    /// Git author of the commit that added the preset (often the porter, not the shader author).
    pub author: String,
}

#[derive(Deserialize)]
struct Index {
    source: String,
    presets: HashMap<String, (String, String, String)>,
}

fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        serde_json::from_str(include_str!("../../../data/shader-index.json")).unwrap_or(Index {
            source: String::new(),
            presets: HashMap::new(),
        })
    })
}

/// `libretro/slang-shaders@<commit>` the index was built from.
pub fn source() -> &'static str {
    &index().source
}

/// Preview id of a preset (`data/shader-thumbs.json`, built by `scripts/shader-thumbs/render.sh`):
/// the repo's `shader-thumbs/<id>.webp` and `<id>-detail.webp`. `None` = no preview rendered.
pub fn thumb(preset: &str) -> Option<&'static str> {
    static THUMBS: OnceLock<HashMap<String, String>> = OnceLock::new();
    THUMBS
        .get_or_init(|| {
            serde_json::from_str(include_str!("../../../data/shader-thumbs.json"))
                .unwrap_or_default()
        })
        .get(preset)
        .map(String::as_str)
}

/// History of a preset (`crt/crt-royale.slangp`), if the snapshot knows it.
pub fn meta(preset: &str) -> Option<ShaderMeta> {
    index()
        .presets
        .get(preset)
        .map(|(added, changed, author)| ShaderMeta {
            added: added.clone(),
            changed: changed.clone(),
            author: author.clone(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_index_parses() {
        assert!(source().starts_with("libretro/slang-shaders@"));
        let m = meta("crt/crt-royale.slangp").unwrap();
        assert_eq!(m.added.len(), 10);
        assert!(m.changed >= m.added);
        assert!(!m.author.is_empty());
        assert!(meta("nope/missing.slangp").is_none());
        assert!(thumb("nope/missing.slangp").is_none());
    }

    #[test]
    fn bundled_thumbs_are_ids() {
        assert_eq!(thumb("crt/crt-geom.slangp").map(str::len), Some(12));
    }
}
