use ironbound::tooltip;

#[test]
fn artwork_can_arrive_after_items_render_without_losing_unknown_items() {
    assert!(tooltip::item_icon(2347).contains("data-item-icon=\"2347\""));
    assert!(tooltip::install_icons("unavailable").is_err());
    assert!(tooltip::item_icon(2347).contains("data-item-icon=\"2347\""));
    tooltip::install_icons(r#"{"2347":"iVBORw0KGgo="}"#).unwrap();
    let icon = tooltip::item_icon(2347);
    assert!(icon.contains("src=\"data:image/png;base64,iVBORw0KGgo=\""));
    assert!(!icon.contains("data-item-icon"));
    assert!(tooltip::item_icon(999_999).contains("unknown-item"));
}

#[test]
fn compressed_stages_merge_and_invalid_stage_preserves_loaded_artwork() {
    let first = ironbound::icon_assets::compress(br#"{"999991":"Zmlyc3Q="}"#).unwrap();
    let second = ironbound::icon_assets::compress(br#"{"999992":"c2Vjb25k"}"#).unwrap();
    tooltip::install_compressed_icons(&first).unwrap();
    assert!(tooltip::install_compressed_icons(b"invalid gzip").is_err());
    let malformed = ironbound::icon_assets::compress(b"not JSON").unwrap();
    assert!(tooltip::install_compressed_icons(&malformed).is_err());
    assert!(tooltip::item_icon(999991).contains("Zmlyc3Q="));
    tooltip::install_compressed_icons(&second).unwrap();
    assert!(tooltip::item_icon(999991).contains("Zmlyc3Q="));
    assert!(tooltip::item_icon(999992).contains("c2Vjb25k"));
}
