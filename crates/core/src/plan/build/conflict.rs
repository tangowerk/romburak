//! Arcade conflicts decided by DAT completeness (`Rules::auto_conflicts`, SPEC §11a).

use super::{Builder, Item, Op, TRASH_DIR};
use crate::rules::{Rule, Why};
use std::path::{Path, PathBuf};

impl Builder<'_> {
    /// `it` clashes with `target` (different content). With both copies scored against the
    /// target core's DAT (`Options::dat_score`), the more complete one wins: a better inbox
    /// copy replaces the library file (which goes to `_trash`), otherwise `it` is trashed.
    /// `None` leaves the conflict as a decision (no DAT, or the target is claimed by an item
    /// this plan places and `it` would be better).
    pub(super) fn auto_conflict(&mut self, it: &Item, ops: &[Op], target: &Path) -> Option<bool> {
        if !self.opts.rules.auto_conflicts {
            return None;
        }
        let score = |p: &Path| self.opts.dat_score.get(p).copied();
        let mine = score(it.files.primary())?;
        let (other, replaceable): (PathBuf, bool) = match self.claimed.get(target) {
            Some(Some(src)) => (src.clone(), false),
            Some(None) => return None,
            None => (target.to_path_buf(), true),
        };
        let theirs = score(&other)?;
        let missing = |(have, need): (u32, u32)| need.saturating_sub(have);
        let detail = format!("{}/{} vs {}/{} roms", mine.0, mine.1, theirs.0, theirs.1);
        if missing(mine) < missing(theirs) {
            if !replaceable {
                return None;
            }
            let trash = self.library.join(TRASH_DIR);
            let mut new = vec![Op::Move {
                from: target.to_path_buf(),
                to: self.free_trash_target(&trash, target),
            }];
            new.extend(ops.iter().cloned());
            self.why = Why::new(
                Rule::AutoConflict,
                format!("auto: more complete ({detail}), library copy to _trash"),
            );
            return Some(self.commit(it, new));
        }
        let as_good = if missing(mine) == missing(theirs) {
            "as complete"
        } else {
            "more complete"
        };
        self.why = Why::new(
            Rule::AutoConflict,
            format!("auto: kept copy is {as_good} ({detail})"),
        );
        self.discard(it);
        Some(false)
    }
}
