//! Planning with arcade conflicts decided by DAT completeness (`Rules::auto_conflicts`).

use crate::{Result, Store};
use romburak_core::arcade;
use romburak_core::plan::{self, Decision, Files, Ident, Item, Options, Plan};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

impl Store {
    /// [`plan::build`]; if conflicts remain and `auto_conflicts` is on, the zips competing
    /// for each target are scored together (DAT or superset) and the plan is built again,
    /// until no new competitor appears (a replaced library copy can turn a duplicate of it
    /// into a new conflict).
    pub fn plan(&self, items: &[Item], library: &Path, opts: &mut Options) -> Result<Plan> {
        if opts.content.is_empty() {
            opts.content = self.content_hashes(library)?;
            if let Some(inbox) = &opts.inbox {
                opts.content.extend(self.content_hashes(inbox)?);
            }
        }
        let mut p = plan::build(items, library, opts);
        if !opts.rules.auto_conflicts {
            return Ok(p);
        }
        // target → (system, competing inbox/library paths)
        let mut groups: HashMap<PathBuf, (String, BTreeSet<PathBuf>)> = HashMap::new();
        for _ in 0..3 {
            let mut grew = false;
            for d in &p.decisions {
                let Decision::Conflict { path, target } = d else {
                    continue;
                };
                let Some(system) = conflict_system(items, library, path, target) else {
                    continue;
                };
                let (_, paths) = groups
                    .entry(target.clone())
                    .or_insert_with(|| (system, BTreeSet::from([target.clone()])));
                grew |= paths.insert(path.clone());
            }
            if !grew {
                break;
            }
            opts.dat_score.clear();
            for (target, (system, paths)) in &groups {
                opts.dat_score
                    .extend(self.score_group(system, target, paths)?);
            }
            p = plan::build(items, library, opts);
        }
        Ok(p)
    }

    /// (roms present, roms needed) per zip competing for `target`, against the DAT set of
    /// `system`. Without a DAT set (e.g. Flycast's `_bios/dc`) the union of all copies
    /// counts, but only if one copy holds all of it; otherwise nothing is scored and the
    /// conflicts stay a decision.
    fn score_group(
        &self,
        system: &str,
        target: &Path,
        paths: &BTreeSet<PathBuf>,
    ) -> Result<Vec<(PathBuf, (u32, u32))>> {
        let mut copies = Vec::new();
        for p in paths {
            let Ok(members) = romburak_core::archive::members(p) else {
                return Ok(Vec::new());
            };
            let mut crcs: Vec<u32> = members.into_iter().map(|(_, crc)| crc).collect();
            crcs.sort_unstable();
            crcs.dedup();
            copies.push((p.clone(), crcs));
        }
        let Some(stem) = target.file_stem() else {
            return Ok(Vec::new());
        };
        let needed: Vec<u32> = match self.dat_set_rc(system, &stem.to_string_lossy())? {
            Some(set) => set
                .roms
                .iter()
                .filter(|r| !r.disk && !r.merge && r.crc != 0)
                .map(|r| r.crc)
                .collect(),
            None => {
                let mut union: Vec<u32> = copies.iter().flat_map(|(_, c)| c.clone()).collect();
                union.sort_unstable();
                union.dedup();
                if !copies.iter().any(|(_, c)| *c == union) {
                    return Ok(Vec::new());
                }
                union
            }
        };
        Ok(copies
            .into_iter()
            .map(|(p, have)| {
                let n = needed
                    .iter()
                    .filter(|c| have.binary_search(c).is_ok())
                    .count();
                (p, (n as u32, needed.len() as u32))
            })
            .collect())
    }
}

/// The core whose folder (or `_bios/…`) holds `target`, for the zip item at `path`; `None`
/// if this is no zip conflict of an identified item.
fn conflict_system(items: &[Item], library: &Path, path: &Path, target: &Path) -> Option<String> {
    let zip = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"));
    if !zip(path) || !zip(target) {
        return None;
    }
    let it = items.iter().find(|i| i.files.primary() == path)?;
    let g = match &it.ident {
        Ident::Known(g) | Ident::Bios(g) => g,
        // System.dat firmware (e.g. `_bios/dc/awbios.zip`)
        Ident::Firmware(fw) => fw.first()?,
        _ => return None,
    };
    let alt = match &it.files {
        Files::Set { alt, .. } => alt.as_slice(),
        _ => &[],
    };
    let dir = target.parent();
    let found = std::iter::once(g).chain(alt).find(|s| {
        dir == Some(&library.join(&s.system))
            || dir == Some(&library.join(arcade::bios_dir(&s.system)))
    });
    Some(found.unwrap_or(g).system.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use romburak_core::plan::{Game, Mode};
    use std::io::Write;

    fn zip(path: &Path, members: &[&str]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for m in members {
            w.start_file(*m, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(m.as_bytes()).unwrap();
        }
        w.finish().unwrap();
    }

    /// Plans an inbox `awbios.zip` (Flycast, no DAT) against the library copy.
    fn plan(inbox_members: &[&str], lib_members: &[&str]) -> Plan {
        let tmp = tempfile::TempDir::new().unwrap();
        let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
        let sys = "Sega - Naomi";
        let new = inbox.join("awbios.zip");
        zip(&new, inbox_members);
        zip(
            &lib.join(arcade::bios_dir(sys)).join("awbios.zip"),
            lib_members,
        );
        let items = [Item {
            files: Files::Set {
                archive: new,
                chds: vec![],
                alt: vec![],
                dat_note: String::new(),
            },
            ident: Ident::Bios(Game {
                system: sys.into(),
                name: "awbios".into(),
                crc: None,
            }),
            in_library: false,
        }];
        let mut opts = Options {
            mode: Mode::Move,
            rules: Default::default(),
            verdicts: Default::default(),
            hashes: Default::default(),
            misnamed: Default::default(),
            dat_score: Default::default(),
            content: Default::default(),
            inbox: None,
            ignore: Vec::new(),
        };
        Store::open_in_memory()
            .unwrap()
            .plan(&items, &lib, &mut opts)
            .unwrap()
    }

    #[test]
    fn without_dat_a_superset_wins() {
        // inbox holds everything of the library copy plus one file: it replaces it
        let p = plan(&["bios0", "bios1", "fpr"], &["bios0", "bios1"]);
        assert!(p.decisions.is_empty(), "{:?}", p.decisions);
        assert_eq!(p.ops.len(), 2);
        assert!(p.ops[0].target().ends_with("_trash/awbios.zip"));
        assert!(p.why[0].detail.contains("3/3 vs 2/3 roms"), "{}", p.why[0]);
        // library holds more: the inbox copy goes to the trash
        let p = plan(&["bios0"], &["bios0", "bios1"]);
        assert!(p.decisions.is_empty());
        assert_eq!(p.discarded, 1);
    }

    #[test]
    fn system_dat_firmware_superset_wins() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
        let new = inbox.join("dc/awbios.zip");
        zip(&new, &["bios0", "bios1", "fpr"]);
        zip(&lib.join("_bios/dc/awbios.zip"), &["bios0", "bios1"]);
        let items = [Item {
            files: Files::Single(new),
            ident: Ident::Firmware(vec![Game {
                system: "Sega - Dreamcast-based Arcade".into(),
                name: "dc/awbios.zip".into(),
                crc: None,
            }]),
            in_library: false,
        }];
        let mut opts = Options {
            mode: Mode::Move,
            rules: Default::default(),
            verdicts: Default::default(),
            hashes: Default::default(),
            misnamed: Default::default(),
            dat_score: Default::default(),
            content: Default::default(),
            inbox: None,
            ignore: Vec::new(),
        };
        let p = Store::open_in_memory()
            .unwrap()
            .plan(&items, &lib, &mut opts)
            .unwrap();
        assert!(p.decisions.is_empty(), "{:?}", p.decisions);
        assert!(p.ops[0].target().ends_with("_trash/awbios.zip"));
    }

    #[test]
    fn copy_of_the_replaced_library_file_loses_too() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
        let sys = "Sega - Naomi";
        let (better, old) = (inbox.join("dc/awbios.zip"), inbox.join("awbios.zip"));
        zip(&better, &["bios0", "bios1", "fpr"]);
        zip(&old, &["bios0", "bios1"]);
        let in_lib = lib.join(arcade::bios_dir(sys)).join("awbios.zip");
        std::fs::create_dir_all(in_lib.parent().unwrap()).unwrap();
        std::fs::copy(&old, &in_lib).unwrap();
        let bios = |p: PathBuf| Item {
            files: Files::Single(p),
            ident: Ident::Bios(Game {
                system: sys.into(),
                name: "awbios".into(),
                crc: None,
            }),
            in_library: false,
        };
        let items = [bios(better), bios(old)];
        let mut opts = Options {
            mode: Mode::Move,
            rules: Default::default(),
            verdicts: Default::default(),
            hashes: Default::default(),
            misnamed: Default::default(),
            dat_score: Default::default(),
            content: Default::default(),
            inbox: None,
            ignore: Vec::new(),
        };
        let p = Store::open_in_memory()
            .unwrap()
            .plan(&items, &lib, &mut opts)
            .unwrap();
        assert!(p.decisions.is_empty(), "{:?}", p.decisions);
        assert_eq!(p.ops.len(), 3, "{:?}", p.ops);
        assert_eq!(p.discarded, 1);
    }

    #[test]
    fn without_dat_truly_different_copies_stay_a_decision() {
        let p = plan(&["bios0", "new"], &["bios0", "old"]);
        assert!(matches!(p.decisions[..], [Decision::Conflict { .. }]));
    }
}
