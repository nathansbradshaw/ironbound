//! Run public, independent reference cases; never reads a player bank.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "tests/fixtures/wiki-reference.json".into());
    let suite: ironbound::validation::ReferenceSuite =
        serde_json::from_str(&std::fs::read_to_string(path)?)?;
    println!("Reference: {} @ {}", suite.source, suite.revision);
    if let Some(revision) = &suite.live_revision {
        println!("Live Wiki UI reference: {revision} (rounded DPS/accuracy)");
    }
    for revision in suite
        .cases
        .iter()
        .filter_map(|case| case.reference_revision.as_deref())
        .collect::<std::collections::BTreeSet<_>>()
    {
        println!("Additional per-case reference: {revision}");
    }
    let mut failed = 0;
    for case in &suite.cases {
        match case.check() {
            Ok(actual) => println!(
                "PASS {}: attack {}, max {}, DPS {:.6}",
                case.name, actual.attack_roll, actual.max_hit, actual.dps
            ),
            Err(reason) => {
                eprintln!("FAIL {reason}");
                failed += 1;
            }
        }
    }
    println!("{} passed; {failed} failed", suite.cases.len() - failed);
    if failed > 0 {
        return Err(
            "Reference mismatch; do not update expected values to match implementation".into(),
        );
    }
    Ok(())
}
