//! Planning with arcade conflicts decided by DAT completeness (`Rules::auto_conflicts`).

use crate::{Result, Store};
use romburak_core::arcade;
use romburak_core::plan::{self, Decision, Files, Ident, Item, Options, Plan};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

impl Store {
    /// [`plan::build`]; if conflicts remain and `auto_conflicts` is on, the arcade zips
    /// involved are scored against the target core's DAT and the plan is built again.
    pub fn plan(&self, items: &[Item], library: &Path, opts: &mut Options) -> Result<Plan> {
        let p = plan::build(items, library, opts);
        if !opts.rules.auto_conflicts {
            return Ok(p);
        }
        let scores = self.conflict_scores(items, library, &p.decisions)?;
        if scores.is_empty() {
            return Ok(p);
        }
        opts.dat_score.extend(scores);
        Ok(plan::build(items, library, opts))
    }

    /// (roms present, roms needed) for both zips of each arcade conflict, against the DAT
    /// set of the core whose folder holds the target. Conflicts without a DAT are left out.
    fn conflict_scores(
        &self,
        items: &[Item],
        library: &Path,
        decisions: &[Decision],
    ) -> Result<HashMap<PathBuf, (u32, u32)>> {
        let mut out = HashMap::new();
        for d in decisions {
            let Decision::Conflict { path, target } = d else {
                continue;
            };
            let zip = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"));
            let (Some(stem), true) = (target.file_stem(), zip(path) && zip(target)) else {
                continue;
            };
            let Some(it) = items.iter().find(|i| i.files.primary() == path) else {
                continue;
            };
            let (Ident::Known(g) | Ident::Bios(g)) = &it.ident else {
                continue;
            };
            let alt = match &it.files {
                Files::Set { alt, .. } => alt.as_slice(),
                _ => &[],
            };
            let dir = target.parent();
            let system = std::iter::once(g)
                .chain(alt)
                .find(|s| {
                    dir == Some(&library.join(&s.system))
                        || dir == Some(&library.join(arcade::bios_dir(&s.system)))
                })
                .unwrap_or(g)
                .system
                .clone();
            let Some(set) = self.dat_set_rc(&system, &stem.to_string_lossy())? else {
                continue;
            };
            let needed: Vec<u32> = set
                .roms
                .iter()
                .filter(|r| !r.disk && !r.merge && r.crc != 0)
                .map(|r| r.crc)
                .collect();
            let score = |p: &Path| -> Option<(u32, u32)> {
                let members = romburak_core::archive::members(p).ok()?;
                let have = needed
                    .iter()
                    .filter(|c| members.iter().any(|(_, m)| m == *c))
                    .count();
                Some((have as u32, needed.len() as u32))
            };
            if let (Some(a), Some(b)) = (score(path), score(target)) {
                out.insert(path.clone(), a);
                out.insert(target.clone(), b);
            }
        }
        Ok(out)
    }
}
