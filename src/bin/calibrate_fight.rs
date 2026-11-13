//! Local numerical calibration. No network, account API, or Python dependency.
use ironbound::{calibration, simulation::Model};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let mode = args
        .get(1)
        .ok_or("Usage: calibrate_fight template SCENARIO.json | compare WORKSHEET.json")?;
    let path = args.get(2).ok_or("Provide a JSON file")?;
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > calibration::MAX_BYTES as u64 {
        return Err("Input exceeds 4 MB".into());
    }
    let value: serde_json::Value = serde_json::from_reader(file)?;
    match mode.as_str() {
        "template" => {
            if value["version"].as_u64() != Some(1) {
                return Err("Unsupported scenario version".into());
            }
            let model: Model = serde_json::from_value(
                value
                    .get("inputs")
                    .ok_or("Missing numerical inputs")?
                    .clone(),
            )?;
            // Validate the model instead of trusting an arbitrary export.
            ironbound::simulation::simulate(&model)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&calibration::worksheet(&model))?
            );
        }
        "compare" => {
            let data: calibration::Dataset = serde_json::from_value(value)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&calibration::compare(&data)?)?
            );
        }
        _ => return Err("Expected template or compare".into()),
    }
    Ok(())
}
