//! Offline replay uses only the exported numerical scenario, never an account API.
use ironbound::simulation::{Model, simulate};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Usage: cargo run --bin simulate_fight -- exported-scenario.json")?;
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > 4_000_000 {
        return Err("Scenario exceeds 4 MB".into());
    }
    let value: serde_json::Value = serde_json::from_reader(file)?;
    if value.get("version").and_then(|v| v.as_u64()) != Some(1) {
        return Err("Unsupported scenario version".into());
    }
    let model: Model = serde_json::from_value(
        value
            .get("inputs")
            .ok_or("Missing numerical scenario inputs")?
            .clone(),
    )?;
    println!("{}", serde_json::to_string_pretty(&simulate(&model)?)?);
    Ok(())
}
