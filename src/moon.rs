//! Sustained Eclipse burns; damage remains delayed, outside direct hit rolls.
use serde::Deserialize;
#[derive(Deserialize)]
struct Rules {
    strong_burn_immune: Vec<i32>,
}
pub fn burn_immune(id: i32) -> bool {
    static DATA: std::sync::OnceLock<Rules> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/moon-rules.json")).expect("Moon rules")
    })
    .strong_burn_immune
    .contains(&crate::encounters::source_npc_id(id))
}
/// Bernoulli arrivals into a deterministic-lifetime loss system. At an attack
/// epoch, the previous (lifetime-1)/interval epochs can hold up to `cap` burns.
/// Each admissible occupancy mask with k<cap has weight (p/q)^k; a full mask
/// has weight q*(p/q)^cap. Summing masks by binomial coefficient avoids an
/// exponential state enumeration. Expiration precedes an arrival at that tick.
pub fn accepted_burns_per_attack(chance: f64, interval: i32, lifetime: i32, cap: usize) -> f64 {
    if chance <= 0.0 || interval <= 0 || lifetime <= 0 || cap == 0 {
        return 0.0;
    }
    let p = chance.clamp(0.0, 1.0);
    let previous = ((lifetime - 1) / interval) as usize;
    if previous < cap {
        return p;
    }
    if p == 1.0 {
        return cap as f64 / (previous + 1) as f64;
    }
    let q = 1.0 - p;
    let odds = p / q;
    let mut weight = 1.0;
    let mut total = 1.0;
    let mut full = 0.0;
    for k in 1..=cap {
        weight *= (previous - k + 1) as f64 / k as f64 * odds;
        if k == cap {
            full = weight * q;
            total += full;
        } else {
            total += weight;
        }
    }
    p * (1.0 - full / total)
}
pub fn burn_dps(accuracy: f64, interval: i32) -> f64 {
    accepted_burns_per_attack(accuracy * 0.2, interval, 40, 5) * 10.0 / (f64::from(interval) * 0.6)
}
