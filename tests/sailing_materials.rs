use ironbound::{
    account,
    planner::Options,
    sailing::{Boat, Settings},
    sailing_materials as materials,
    sailing_owned::Fleet,
};
use std::collections::BTreeMap;
fn boat() -> Settings {
    Settings {
        boat: Boat::Sloop,
        keel: 4,
        ..Default::default()
    }
}
fn empty() -> BTreeMap<i32, u32> {
    BTreeMap::new()
}
fn line(b: &materials::Bill, id: i32) -> &materials::Line {
    b.raw.iter().find(|l| l.id == id).unwrap()
}
#[test]
fn pinned_catalog_covers_every_fitting_and_expands_without_cycles() {
    let c = materials::catalog();
    assert_eq!(c.recipes.len(), 52);
    assert_eq!(c.items.len(), 48);
    assert_eq!(c.sources.len(), 80);
    assert!(c.sources.values().all(|r| *r > 0));
    for recipe in c.recipes.values() {
        assert!(recipe.source.contains("?oldid="));
        let b = materials::bill(&[recipe], &[], &empty());
        assert!(!b.raw.is_empty());
    }
    for id in c.items.keys() {
        let b = materials::bill(&[], &[(*id, 100)], &empty());
        assert!(!b.raw.is_empty());
    }
}
#[test]
fn adamant_keel_cost_is_fifty_bars_for_skiff_and_four_hundred_for_sloop() {
    let mut b = boat();
    let cost = materials::upgrades(&b, None, &empty());
    assert_eq!(line(&cost, 2361).need, 400);
    assert_eq!(line(&cost, 32889).need, 5);
    b.boat = Boat::Skiff;
    let cost = materials::upgrades(&b, None, &empty());
    assert_eq!(line(&cost, 2361).need, 50);
}
#[test]
fn existing_large_parts_regular_parts_and_bars_reduce_the_remaining_cost_once() {
    let stock = BTreeMap::from([(32032, 2), (32011, 3), (2361, 100), (32889, 5)]);
    let cost = materials::bill(&[materials::recipe(&boat(), "keel").unwrap()], &[], &stock);
    assert_eq!(
        (
            line(&cost, 2361).need,
            line(&cost, 2361).have,
            line(&cost, 2361).missing
        ),
        (335, 100, 235)
    );
    assert_eq!(line(&cost, 32889).missing, 0);
    assert_eq!(stock[&32032], 2);
    assert_eq!(stock[&2361], 100);
}
#[test]
fn combined_cannon_recipes_share_nails_bars_and_batch_rounding() {
    let mut b = boat();
    b.cannon = Some(4);
    b.cannon_count = 2;
    b.second_cannon = Some(4);
    let r = materials::recipe(&b, "cannon").unwrap();
    let stock = BTreeMap::from([(2361, 18), (4823, 1), (31432, 8)]);
    let cost = materials::bill(&[r, r], &[], &stock);
    assert_eq!((line(&cost, 2361).need, line(&cost, 2361).missing), (19, 1));
    assert_eq!(cost.crafts[&4823], 45);
    assert_eq!(line(&cost, 31432).missing, 0);
}
#[test]
fn owned_prepared_parts_avoid_smithing_requirements_and_noted_bars_are_credited() {
    let cost = materials::bill(
        &[materials::recipe(&boat(), "keel").unwrap()],
        &[],
        &BTreeMap::from([(32032, 16), (32889, 5)]),
    );
    assert!(!cost.skills.contains_key("Smithing"));
    assert!(cost.raw.iter().all(|l| l.missing == 0));
    let mut s = account::demo();
    s.members[0].bank = Some(vec![2361, 3, 2362, 7]);
    assert_eq!(materials::stock(&s, &Options::default())[&2361], 10);
}
#[test]
fn plans_never_grant_fittings_and_marking_them_fitted_clears_the_completed_plan() {
    let mut f = Fleet::default();
    let fitted = boat();
    f.save_boat(None, "My sloop", &fitted).unwrap();
    let mut target = fitted.clone();
    target.keel = 5;
    f.save_upgrade(0, &target).unwrap();
    assert_eq!(f.profiles[0].settings, fitted);
    assert!(!f.permits(&target));
    assert!(f.valid());
    let raw = serde_json::to_string(&f).unwrap();
    assert_eq!(serde_json::from_str::<Fleet>(&raw).unwrap(), f);
    f.save_boat(Some(0), "My sloop", &target).unwrap();
    assert!(f.profiles[0].upgrade.is_none());
    assert!(f.permits(&target));
    assert!(materials::changed(&target, Some(&target)).is_empty());
}
#[test]
fn multiple_saved_boat_plans_share_one_bank_in_the_combined_bill() {
    let mut s = account::demo();
    s.members[0].bank = Some(vec![2361, 50]);
    let mut b = boat();
    b.boat = Boat::Skiff;
    b.keel = 0;
    let f = &mut s.members[0].sailing_owned;
    for n in 0..2 {
        f.save_boat(None, &format!("Skiff {n}"), &b).unwrap();
        let mut target = b.clone();
        target.keel = 4;
        f.save_upgrade(n, &target).unwrap();
    }
    let html = materials::fleet_cost(&s, &Options::default());
    assert!(html.contains("Adamantite bar"));
    assert!(html.contains("<td>100</td><td>50</td><td class=\"material-missing\">50</td>"));
}
#[test]
fn supply_batches_credit_owned_kits_and_dragon_cannonballs_are_gathered_not_smithed() {
    let b = materials::bill(&[], &[(31982, 10)], &BTreeMap::from([(31982, 4)]));
    assert_eq!(line(&b, 31438).need, 2);
    assert_eq!(line(&b, 31996).need, 1);
    assert_eq!(line(&b, 1941).need, 10);
    let b = materials::bill(&[], &[(31916, 100)], &BTreeMap::from([(31996, 1000)]));
    assert_eq!(line(&b, 31916).missing, 100);
    assert!(!b.raw.iter().any(|l| l.id == 31996));
}

#[test]
fn real_bank_kit_ids_and_legacy_manual_counts_are_credited_without_double_counting() {
    let mut s = account::demo();
    s.members[0].bank = Some(vec![31982, 5, 31983, 2]);
    s.members[0].sailing_owned.loaded.insert(32306, 3);
    let o = Options::default();
    assert!(s.members[0].sailing_owned.valid());
    assert_eq!(ironbound::sailing_owned::stock(&s, &o)[&31982], 10);
    assert_eq!(materials::stock(&s, &o)[&31982], 10);
    s.members[0].sailing_owned.loaded.insert(31982, 4);
    assert_eq!(ironbound::sailing_owned::stock(&s, &o)[&31982], 11);
    assert_eq!(materials::stock(&s, &o)[&31982], 11);
}
