//! The automatics (SPEC §11a "Fewer import to-dos"): ties, 1G1R losers, arcade conflicts.

use super::tests::{file, item};
use super::*;
use crate::rules::Rule;
use std::path::Path;
use tempfile::TempDir;

const CPC: &str = "Amstrad - CPC";

fn game(system: &str, name: &str) -> Ident {
    Ident::Known(Game {
        system: system.into(),
        name: name.into(),
        crc: Some(1),
    })
}

fn opts() -> Options {
    Options {
        mode: Mode::Move,
        rules: Default::default(),
        verdicts: Default::default(),
        hashes: Default::default(),
        misnamed: Default::default(),
        dat_score: Default::default(),
        inbox: None,
        ignore: Vec::new(),
    }
}

/// (rule id, target relative to `lib`) per op.
fn ops(plan: &Plan, lib: &Path) -> Vec<(&'static str, String)> {
    plan.ops
        .iter()
        .zip(&plan.why)
        .map(|(op, why)| {
            let t = op.target().strip_prefix(lib).unwrap_or(op.target());
            (why.rule.id(), t.display().to_string())
        })
        .collect()
}

#[test]
fn tie_is_decided_and_the_loser_trashed() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(
            file(&inbox, "a.dsk", "1"),
            game(CPC, "180 [cr Section Jaguar]"),
            false,
        ),
        item(file(&inbox, "b.dsk", "2"), game(CPC, "180"), false),
    ];
    let plan = build(&items, &lib, &opts());
    assert!(plan.decisions.is_empty(), "{:?}", plan.decisions);
    assert_eq!(
        ops(&plan, &lib),
        [
            ("auto-tie", format!("{CPC}/180.dsk")),
            ("auto-trash", "_trash/a.dsk".into()),
        ]
    );
    assert_eq!(plan.why[0].detail, "auto: original before crack/trainer");
    assert_eq!(plan.why[1].detail, "auto: lost the tie");
}

#[test]
fn prefer_verdict_beats_the_automatic_tie() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(
            file(&inbox, "a.dsk", "1"),
            game(CPC, "180 [cr Section Jaguar]"),
            false,
        ),
        item(file(&inbox, "b.dsk", "2"), game(CPC, "180"), false),
    ];
    let mut o = opts();
    o.verdicts.insert(
        (CPC.into(), "180 [cr Section Jaguar]".into()),
        Verdict::Prefer,
    );
    let plan = build(&items, &lib, &o);
    assert_eq!(
        ops(&plan, &lib)[0],
        ("g1r-pick", format!("{CPC}/180 [cr Section Jaguar].dsk"))
    );
}

#[test]
fn losers_go_to_the_trash_unless_kept_or_nothing_is_picked() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let sys = "Nintendo - SNES";
    let items = [
        item(
            file(&inbox, "eu.sfc", "1"),
            game(sys, "Mario (Europe)"),
            false,
        ),
        item(file(&inbox, "us.sfc", "2"), game(sys, "Mario (USA)"), false),
        item(
            file(&inbox, "jp.sfc", "3"),
            game(sys, "Mario (Japan)"),
            false,
        ),
        item(
            file(&inbox, "dup.sfc", "1"),
            game(sys, "Mario (Europe)"),
            false,
        ),
        item(
            file(&inbox, "beta.sfc", "4"),
            game(sys, "Zelda (Europe) (Beta)"),
            false,
        ),
    ];
    let mut o = opts();
    o.verdicts
        .insert((sys.into(), "Mario (Japan)".into()), Verdict::Keep);
    let plan = build(&items, &lib, &o);
    let got = ops(&plan, &lib);
    assert!(got.contains(&("g1r-pick", format!("{sys}/Mario (Europe).sfc"))));
    assert!(got.contains(&("verdict", format!("{sys}/Mario (Japan).sfc"))));
    assert!(got.contains(&("auto-trash", "_trash/us.sfc".into())));
    assert!(got.contains(&("auto-trash", "_trash/dup.sfc".into())));
    // every Zelda release is excluded: trashing it would lose the game
    assert!(matches!(
        &plan.decisions[..],
        [Decision::Rejected { name, kept: None, .. }] if name == "Zelda (Europe) (Beta)"
    ));
    assert_eq!(plan.discarded, 2);
}

#[test]
fn switches_off_restore_the_decision_queue() {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let items = [
        item(
            file(&inbox, "a.dsk", "1"),
            game(CPC, "180 [cr Section Jaguar]"),
            false,
        ),
        item(file(&inbox, "b.dsk", "2"), game(CPC, "180"), false),
        item(
            file(&inbox, "c.dsk", "3"),
            game(CPC, "Xenon (France)"),
            false,
        ),
        item(file(&inbox, "d.dsk", "4"), game(CPC, "Xenon (USA)"), false),
    ];
    let mut o = opts();
    o.rules.auto_ties = false;
    o.rules.auto_trash = false;
    let plan = build(&items, &lib, &o);
    assert!(matches!(
        &plan.decisions[..],
        [Decision::Tie { .. }, Decision::Rejected { .. }]
    ));
}

/// An inbox BIOS `neogeo.zip` conflicting with the library copy in `_bios/fbneo`.
fn bios_conflict(
    inbox_score: (u32, u32),
    lib_score: Option<(u32, u32)>,
) -> (Plan, PathBuf, TempDir) {
    let tmp = TempDir::new().unwrap();
    let (inbox, lib) = (tmp.path().join("inbox"), tmp.path().join("lib"));
    let fb = "FBNeo - Arcade Games";
    let in_lib = lib.join(crate::arcade::bios_dir(fb)).join("neogeo.zip");
    file(
        &lib,
        &in_lib.strip_prefix(&lib).unwrap().display().to_string(),
        "old",
    );
    let new = file(&inbox, "neogeo.zip", "new");
    let items = [Item {
        files: Files::Set {
            archive: new.clone(),
            chds: vec![],
            alt: vec![],
            dat_note: String::new(),
        },
        ident: Ident::Bios(Game {
            system: fb.into(),
            name: "Neo Geo".into(),
            crc: None,
        }),
        in_library: false,
    }];
    let mut o = opts();
    o.dat_score.insert(new, inbox_score);
    if let Some(s) = lib_score {
        o.dat_score.insert(in_lib.clone(), s);
    }
    let plan = build(&items, &lib, &o);
    let rel = in_lib.strip_prefix(&lib).unwrap().to_path_buf();
    (plan, rel, tmp)
}

#[test]
fn more_complete_inbox_copy_replaces_the_library_copy() {
    let (plan, rel, _tmp) = bios_conflict((20, 20), Some((18, 20)));
    assert!(plan.decisions.is_empty(), "{:?}", plan.decisions);
    let Op::Move { from, to } = &plan.ops[0] else {
        panic!("{:?}", plan.ops)
    };
    assert!(from.ends_with(&rel) && to.ends_with("_trash/neogeo.zip"));
    assert!(plan.ops[1].target().ends_with(&rel));
    assert!(plan.why.iter().all(|w| w.rule == Rule::AutoConflict));
    assert!(
        plan.why[0].detail.contains("20/20 vs 18/20"),
        "{}",
        plan.why[0]
    );
}

#[test]
fn equally_complete_keeps_the_library_copy() {
    let (plan, _, _tmp) = bios_conflict((20, 20), Some((20, 20)));
    assert!(plan.decisions.is_empty());
    assert_eq!(plan.ops.len(), 1);
    assert!(plan.ops[0].target().ends_with("_trash/neogeo.zip"));
    assert_eq!(plan.discarded, 1);
    assert!(
        plan.why[0]
            .detail
            .starts_with("auto: kept copy is as complete")
    );
    let (plan, _, _tmp) = bios_conflict((18, 20), Some((20, 20)));
    assert!(
        plan.why[0]
            .detail
            .starts_with("auto: kept copy is more complete")
    );
}

#[test]
fn conflict_without_dat_score_stays_a_decision() {
    let (plan, _, _tmp) = bios_conflict((20, 20), None);
    assert!(matches!(plan.decisions[..], [Decision::Conflict { .. }]));
    assert!(plan.ops.is_empty());
}
