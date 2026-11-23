//! Fixed public artwork bundles. Never select URLs or pack contents from a bank.
use std::{collections::BTreeMap, io::Write};
pub type Icons = BTreeMap<i32, String>;

pub fn packs(icons: Icons) -> (Icons, Icons) {
    icons.into_iter().partition(|(id, _)| {
        if crate::planner::gear(*id).is_some() || (31800..=32400).contains(id) {
            return true;
        }
        crate::inventory::public_name(*id).is_some_and(|name| {
            crate::inventory::category(*id, &name) != crate::inventory::Category::Other
                || [" plank", " bar", " nails", " parts"]
                    .iter()
                    .any(|end| name.ends_with(end))
        })
    })
}

pub fn compress(raw: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    gzip.write_all(raw)?;
    gzip.finish()
}
