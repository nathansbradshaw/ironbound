//! One OSRS paper-doll layout, shared by editable loadouts and recommendation previews.
use crate::{planner, tooltip::item_icon};
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub const POSITIONS: [(usize, &str); 11] = [
    (0, "eq-head"),
    (1, "eq-cape"),
    (2, "eq-neck"),
    (13, "eq-ammo"),
    (3, "eq-weapon"),
    (4, "eq-body"),
    (5, "eq-shield"),
    (7, "eq-legs"),
    (9, "eq-hands"),
    (10, "eq-feet"),
    (12, "eq-ring"),
];
pub fn paper(title: &str, mut cell: impl FnMut(usize, &str) -> String) -> String {
    let mut h = format!(
        "<div class=\"equipment-paper\"><div class=\"paper-header\">{}</div><div class=\"equipment-grid\">",
        escape(title)
    );
    for (index, class) in POSITIONS {
        h.push_str(&cell(index, class));
    }
    h.push_str("</div></div>");
    h
}
pub fn preview(set: &[Option<i32>], title: &str) -> String {
    paper(title, |index, class| {
        let id = set.get(index).copied().flatten();
        let name = id.map(planner::item_name).unwrap_or_else(|| "Empty".into());
        let label = escape(&format!("{}: {name}", planner::SLOTS[index]));
        format!(
            "<span class=\"equipment-edit {class}\" tabindex=\"0\" role=\"img\" title=\"{label}\" aria-label=\"{label}\"><span class=\"item-slot {}\">{}</span></span>",
            if id.is_some() { "filled" } else { "empty" },
            id.map(item_icon).unwrap_or_default()
        )
    })
}
