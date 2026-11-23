use ironbound::{icon_assets, planner};
use std::{collections::BTreeMap, io::Read};
#[test]
fn public_packs_prioritize_every_equipment_icon_and_reconstruct_the_catalog() {
    let original: BTreeMap<i32, String> =
        serde_json::from_str(include_str!("../assets/item-icons-pack.json")).unwrap();
    let (core, extra) = icon_assets::packs(original.clone());
    for id in original.keys().filter(|id| planner::gear(**id).is_some()) {
        assert!(
            core.contains_key(id),
            "equipment {id} waits for extra artwork"
        );
    }
    for id in [3755, 11907, 554, 566, 385, 2434, 31964, 32181] {
        assert!(core.contains_key(&id), "common gear/supply {id}");
    }
    assert!(core.keys().all(|id| !extra.contains_key(id)));
    let mut reconstructed = BTreeMap::new();
    let mut sizes = vec![];
    for pack in [core, extra] {
        let raw = serde_json::to_vec(&pack).unwrap();
        let gzip = icon_assets::compress(&raw).unwrap();
        sizes.push(gzip.len());
        let mut decoded = String::new();
        flate2::read::GzDecoder::new(gzip.as_slice())
            .read_to_string(&mut decoded)
            .unwrap();
        let decoded: BTreeMap<i32, String> = serde_json::from_str(&decoded).unwrap();
        reconstructed.extend(decoded);
    }
    assert_eq!(reconstructed, original);
    assert!(
        sizes[0] < 5_000_000,
        "equipment stage unexpectedly large: {}",
        sizes[0]
    );
    assert!(sizes.iter().sum::<usize>() < 14_000_000);
}
