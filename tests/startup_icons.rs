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
