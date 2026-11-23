use ironbound::{
    account, account_transfer as transfer,
    planner::{self, Options},
    setups::{Library, Saved},
    share,
};
use std::collections::BTreeMap;
fn saved() -> Saved {
    Saved {
        version: 1,
        snapshot: account::demo(),
        options: Options::default(),
        pins: BTreeMap::new(),
        last_setup_fragment: Some("#setup=old-cached-url".into()),
        budget_caps: [2, 7],
        setups: Library::default(),
        death_risk: Default::default(),
    }
}
#[test]
fn private_transfer_preserves_bank_levels_goals_and_all_configuration_without_cached_urls() {
    let mut s = saved();
    s.options.max_switches = Some(3);
    s.options.food = 9;
    s.options.combat.ammo_shots = 321;
    s.snapshot.members[0]
        .level_overrides
        .insert("Sailing".into(), 88);
    s.snapshot.members[0].sailing_owned.boats[2] = true;
    s.snapshot.members[0]
        .sailing_owned
        .loaded
        .insert(31914, 456);
    s.pins
        .insert("private-goals".into(), vec!["item:20997".into()]);
    s.setups.save("My raid", &s.options).unwrap();
    let link = transfer::fragment(&s).unwrap();
    assert!(link.starts_with(transfer::PREFIX));
    assert!(!link.contains(&s.snapshot.selected));
    assert!(!link.contains("old-cached-url"));
    assert!(link.len() < serde_json::to_string(&s).unwrap().len());
    let restored = transfer::parse(&link).unwrap();
    s.last_setup_fragment = None;
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(s).unwrap()
    );
    assert_eq!(
        transfer::fragment(&transfer::parse(&link).unwrap()).unwrap(),
        link
    );
}
#[test]
fn normal_setup_links_still_exclude_bank_and_private_account_configuration() {
    let s = saved();
    let setup = share::Setup::from_plan(&planner::plan(&s.snapshot, &s.options), &s.options);
    let value = serde_json::to_value(&setup).unwrap();
    assert!(value.get("snapshot").is_none());
    assert!(value.get("bank").is_none());
    assert!(value.get("pins").is_none());
    assert!(value.get("setups").is_none());
    assert!(transfer::parse(&setup.fragment().unwrap()).is_err());
    assert!(share::Setup::parse(&transfer::fragment(&s).unwrap()).is_err());
}
#[test]
fn malformed_truncated_trailing_and_unknown_version_transfer_links_are_rejected() {
    let link = transfer::fragment(&saved()).unwrap();
    for value in [
        "#account=v2.AAAA".to_string(),
        "#account=v1.!!!!".into(),
        "#account=v1.".into(),
        link[..link.len() / 2].to_string(),
        format!("{link}AAAA"),
        format!("#account=v1.{}", "A".repeat(transfer::MAX_FRAGMENT)),
    ] {
        assert!(transfer::parse(&value).is_err());
    }
}
fn encoded(raw: &[u8]) -> String {
    use base64::Engine as _;
    use std::io::Write;
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    gzip.write_all(raw).unwrap();
    format!(
        "{}{}",
        transfer::PREFIX,
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(gzip.finish().unwrap())
    )
}
#[test]
fn decompression_and_account_validation_are_bounded_before_import() {
    let bomb = encoded(&vec![b' '; transfer::MAX_JSON + 1]);
    assert!(bomb.len() < transfer::MAX_FRAGMENT);
    assert!(transfer::parse(&bomb).is_err());
    let mut value = serde_json::to_value(saved()).unwrap();
    value["version"] = 999.into();
    assert!(transfer::parse(&encoded(value.to_string().as_bytes())).is_err());
    value["version"] = 1.into();
    value["snapshot"]["members"][0]["sailing_owned"]["loaded"] = serde_json::json!({"999":1});
    assert!(transfer::parse(&encoded(value.to_string().as_bytes())).is_err());
}

#[test]
fn oversized_valid_accounts_use_backup_fallback_instead_of_a_truncated_url() {
    let mut s = saved();
    let mut seed = 0x9e3779b97f4a7c15u64;
    for id in 1..=5000 {
        let name: String = (0..120)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                (b'a' + (seed % 26) as u8) as char
            })
            .collect();
        s.snapshot.members[0].item_names.insert(id, name);
    }
    assert!(Saved::parse(&serde_json::to_string(&s).unwrap()).is_ok());
    assert!(transfer::fragment(&s).err().unwrap().contains("too large"));
}
