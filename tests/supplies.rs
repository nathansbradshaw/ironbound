use ironbound::{account, inventory::Category, supplies};
#[test]
fn healing_uses_remaining_bites_and_level_scaling() {
    assert_eq!(supplies::food_healing(385, Some(93)).unwrap().0, 20);
    assert_eq!(supplies::food_healing(3144, Some(93)).unwrap().0, 18);
    assert_eq!(
        supplies::food_healing(1891, Some(93)).unwrap(),
        (12, 4, 0, 3)
    );
    assert_eq!(
        supplies::food_healing(2301, Some(93)).unwrap(),
        (22, 11, 0, 2)
    );
    assert_eq!(supplies::food_healing(13441, Some(78)).unwrap().0, 15);
    assert_eq!(supplies::food_healing(13441, Some(93)).unwrap().0, 22);
    assert!(supplies::food_healing(13441, None).is_none());
}
#[test]
fn potion_rankings_compare_restoration_and_remaining_doses() {
    let mut snapshot = account::demo();
    let member = &mut snapshot.members[0];
    member.level_overrides.insert("Prayer".into(), 74);
    let restore = supplies::value(Category::Potions, 3024, "Super restore(4)", member);
    let prayer = supplies::value(Category::Potions, 2434, "Prayer potion(4)", member);
    let half = supplies::value(Category::Potions, 141, "Prayer potion(2)", member);
    assert_eq!(restore.score, Some(104));
    assert_eq!(prayer.score, Some(100));
    assert_eq!(half.score, Some(50));
    assert!(restore.compare(&prayer).is_lt());
    assert!(prayer.compare(&half).is_lt());
    let full = supplies::value(Category::Potions, 2440, "Super strength(4)", member);
    let one = supplies::value(Category::Potions, 161, "Super strength(1)", member);
    assert!(full.compare(&one).is_lt());
    assert_ne!(restore.role, full.role);
    let combined = supplies::value(Category::Potions, 12695, "Super combat potion(4)", member);
    let normal = supplies::value(Category::Potions, 9739, "Combat potion(4)", member);
    assert_eq!(combined.role, "Melee boosts");
    assert!(combined.compare(&normal).is_lt());
}
#[test]
fn every_named_public_item_has_a_bundled_icon() {
    let names: std::collections::BTreeMap<i32, String> =
        serde_json::from_str(include_str!("../assets/item-names.json")).unwrap();
    let icons: std::collections::BTreeMap<i32, String> =
        serde_json::from_str(include_str!("../assets/item-icons-pack.json")).unwrap();
    assert!(names.len() > 20_000);
    let missing: Vec<_> = names
        .keys()
        .filter(|id| **id > 0 && !icons.contains_key(id))
        .collect();
    assert!(missing.is_empty(), "Missing public icons: {missing:?}");
}
