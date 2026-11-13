use crate::{
    account::{self, SKILLS, Snapshot},
    combat, export,
    planner::{self, Boss, Options, Plan},
    reference,
    setups::{SaveStatus, Saved, default_budget_caps},
    share,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Element, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

const STORAGE: &str = "ironbound.account.v1";
fn comparison_caps_query() -> Option<[usize; 2]> {
    let search = window().location().search().ok()?;
    let value = search
        .trim_start_matches('?')
        .split('&')
        .find_map(|part| part.strip_prefix("compare="))?
        .replace("%2C", ",")
        .replace("%2c", ",");
    let (left, right) = value.split_once(',')?;
    let caps = [left.parse::<usize>().ok()?, right.parse::<usize>().ok()?];
    caps.iter().all(|cap| *cap <= 11).then_some(caps)
}
struct State {
    saved: Saved,
    save_status: Cell<SaveStatus>,
    unreadable_save: Option<String>,
    setups_open: bool,
    demo_return: Option<Saved>,
    tab: String,
    view: String,
    phase: Option<usize>,
    message: String,
    error: bool,
    busy: bool,
    export_text: Option<String>,
    shared_setup: Option<share::Setup>,
    edit_inventory: Option<usize>,
    inventory_query: String,
    inventory_filter: String,
    edit_style: Option<String>,
    compare_target: Option<i32>,
    edit_equipment: Option<usize>,
    gear_search: String,
    incoming_attack: Option<String>,
    encounter_query: String,
    encounter_filter: String,
    gear_filter: crate::bestiary::GearFilter,
    requirements_filter: crate::bestiary::RequirementsFilter,
    building_group: bool,
    raid_rooms_open: bool,
    bestiary_open: bool,
    risk_open: bool,
    wilderness_location: String,
    scaredy_cat: bool,
    filler_cap: u32,
    switch_preferences_open: bool,
    packing_limits_open: bool,
    compact_open: bool,
    encounter_filters_open: bool,
    wilderness_options_open: bool,
    combat_options_open: bool,
    attack_budget_open: bool,
    simulation_open: bool,
    simulation_result: Option<(String, Result<crate::simulation::Report, String>)>,
    calibration_result: Option<(String, Result<crate::calibration::Report, String>)>,
    calibration_open: bool,
    budget_comparison_open: bool,
    budget_phases_open: bool,
    budget_window: Option<String>,
    budget_incoming: Option<String>,
    ranking_draft: Option<crate::ranking::Preferences>,
}
impl State {
    fn initial() -> Self {
        let read = window()
            .local_storage()
            .and_then(|storage| storage.ok_or(JsValue::NULL))
            .and_then(|storage| storage.get_item(STORAGE));
        let raw = read.as_ref().ok().and_then(|raw| raw.clone());
        let restored = raw.as_deref().and_then(|raw| Saved::parse(raw).ok());
        let unreadable_save = raw.filter(|_| restored.is_none());
        let save_status = if unreadable_save.is_some() {
            SaveStatus::Unreadable
        } else if read.is_err() {
            SaveStatus::Unavailable
        } else if restored.is_some() {
            SaveStatus::Saved
        } else {
            SaveStatus::Pending
        };
        let mut saved = restored.unwrap_or(Saved {
            death_risk: Default::default(),
            version: 1,
            snapshot: account::demo(),
            options: Options::default(),
            pins: BTreeMap::new(),
            last_setup_fragment: None,
            budget_caps: default_budget_caps(),
            setups: Default::default(),
        });
        if saved.snapshot.source == "Demo" {
            saved.snapshot = account::demo();
        }
        let key = goal_key(&saved.snapshot.selected, &saved.options);
        if saved.options.boss == Boss::Custom {
            if let Some(legacy) = saved
                .pins
                .remove(&format!("{}:custom", saved.snapshot.selected))
            {
                saved.pins.entry(key).or_insert(legacy);
            }
        }
        saved.options.food = saved.options.food.min(20);
        saved.options.prayer_potions = saved.options.prayer_potions.min(12);
        saved.options.raid_level = saved.options.raid_level.min(150);
        saved.options.max_switches = saved.options.max_switches.map(|n| n.min(11));
        saved.options.switch_slots = saved.options.switch_slots.min(20);
        saved.budget_caps = saved.budget_caps.map(|n| n.min(11));
        let comparison_query = comparison_caps_query();
        if let Some(caps) = comparison_query {
            saved.budget_caps = caps;
        }
        if saved.options.fewer_switches {
            saved.options.max_switches = Some(4);
            saved.options.fewer_switches = false;
        }
        let hash = window().location().hash().unwrap_or_default();
        let mut shared_setup = if hash.starts_with("#setup=") {
            share::Setup::parse(&hash).ok()
        } else {
            None
        };
        if shared_setup.is_some()
            && (saved.last_setup_fragment.as_deref() == Some(hash.as_str())
                || share::Setup::from_plan(
                    &planner::plan(&saved.snapshot, &saved.options),
                    &saved.options,
                )
                .fragment()
                .ok()
                .as_deref()
                    == Some(hash.as_str()))
        {
            shared_setup = None;
        }
        if let Some(setup) = &shared_setup {
            saved.options = setup.options();
        }
        Self {
            save_status: Cell::new(save_status),
            unreadable_save,
            setups_open: false,
            demo_return: None,
            shared_setup,
            edit_inventory: None,
            inventory_query: String::new(),
            inventory_filter: "All".into(),
            edit_style: None,
            compare_target: None,
            edit_equipment: None,
            gear_search: String::new(),
            incoming_attack: None,
            encounter_query: String::new(),
            encounter_filter: "For you".into(),
            gear_filter: Default::default(),
            requirements_filter: Default::default(),
            building_group: false,
            raid_rooms_open: false,
            bestiary_open: false,
            risk_open: false,
            wilderness_location: "all".into(),
            scaredy_cat: saved.options.wilderness.is_some(),
            filler_cap: saved
                .options
                .wilderness
                .as_ref()
                .map(|k| k.filler_cap)
                .unwrap_or(crate::wilderness::DEFAULT_CAP),
            switch_preferences_open: false,
            packing_limits_open: false,
            compact_open: false,
            encounter_filters_open: false,
            wilderness_options_open: false,
            combat_options_open: false,
            attack_budget_open: false,
            simulation_open: false,
            simulation_result: None,
            calibration_result: None,
            calibration_open: false,
            budget_comparison_open: comparison_query.is_some(),
            budget_phases_open: false,
            budget_window: None,
            budget_incoming: None,
            ranking_draft: None,
            saved,
            tab: "bossing".into(),
            view: "loadout".into(),
            phase: None,
            message: String::new(),
            error: false,
            busy: false,
            export_text: None,
        }
    }
    fn plan(&self) -> Plan {
        self.shared_setup
            .as_ref()
            .map(share::Setup::plan)
            .unwrap_or_else(|| planner::plan(&self.saved.snapshot, &self.saved.options))
    }
    fn persist(&self) {
        // An example is a temporary preview. Never replace the saved account or its URL.
        if self.demo_return.is_some() || self.unreadable_save.is_some() {
            return;
        }
        let mut stored = self.saved.clone();
        if self.shared_setup.is_none() {
            let base = format!(
                "{}{}{}",
                window().location().origin().unwrap_or_default(),
                window().location().pathname().unwrap_or_default(),
                if self.budget_comparison_open {
                    format!(
                        "?compare={},{}",
                        self.saved.budget_caps[0], self.saved.budget_caps[1]
                    )
                } else {
                    String::new()
                }
            );
            let hash = share::Setup::from_plan(&self.plan(), &self.saved.options)
                .fragment()
                .unwrap_or_default();
            stored.last_setup_fragment = Some(hash.clone());
            let _ = window().history().and_then(|h| {
                h.replace_state_with_url(&JsValue::NULL, "", Some(&format!("{base}{hash}")))
            });
        }
        let result = serde_json::to_string(&stored)
            .map_err(|_| JsValue::NULL)
            .and_then(|raw| {
                window()
                    .local_storage()
                    .and_then(|storage| storage.ok_or(JsValue::NULL))
                    .and_then(|storage| storage.set_item(STORAGE, &raw))
            });
        self.save_status.set(if result.is_ok() {
            SaveStatus::Saved
        } else {
            SaveStatus::Unavailable
        });
    }
    fn notice(&mut self, msg: impl Into<String>, error: bool) {
        self.message = msg.into();
        self.error = error;
    }
    fn pins(&self) -> Vec<String> {
        self.saved
            .pins
            .get(&goal_key(
                &self.saved.snapshot.selected,
                &self.saved.options,
            ))
            .cloned()
            .unwrap_or_default()
    }
}
fn goal_key(account: &str, options: &Options) -> String {
    let encounter = if options.boss != Boss::Custom {
        options.boss.key().to_string()
    } else if let Some(raid) = crate::raids::for_targets(&options.targets) {
        format!("custom:raid:{}", raid.id)
    } else if let Some(entry) = selected_encounter(options) {
        format!("custom:encounter:{}", entry.name)
    } else {
        let mut targets: Vec<_> = options
            .targets
            .iter()
            .map(|t| format!("{}:{}", t.id, t.style))
            .collect();
        targets.sort();
        format!("custom:group:{}", targets.join(","))
    };
    format!("{account}:{encounter}")
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::initial());}
thread_local! {static LIVE_EDIT:Cell<u64>=const {Cell::new(0)};}
struct EfficientCache {
    key: String,
    result: Option<Vec<crate::efficient::Candidate>>,
}
thread_local! {static EFFICIENT:RefCell<Option<EfficientCache>>=const {RefCell::new(None)};}
fn efficient_key(snapshot: &Snapshot, options: &Options) -> String {
    crate::efficient::cache_key(snapshot, options)
}
fn efficient_result(s: &State) -> Option<crate::efficient::Suggestion> {
    let key = efficient_key(&s.saved.snapshot, &s.saved.options);
    EFFICIENT
        .with(|cache| {
            cache
                .borrow()
                .as_ref()
                .filter(|c| c.key == key)
                .and_then(|c| c.result.clone())
        })
        .map(|rows| {
            crate::efficient::choose(
                rows,
                crate::efficient::Tolerances::from_options(&s.saved.options),
                s.saved.options.max_switches.unwrap_or(11),
            )
        })
}
fn ensure_efficient(s: &State) {
    let key = efficient_key(&s.saved.snapshot, &s.saved.options);
    if EFFICIENT.with(|cache| cache.borrow().as_ref().is_some_and(|c| c.key == key)) {
        return;
    }
    EFFICIENT.with(|cache| {
        *cache.borrow_mut() = Some(EfficientCache {
            key: key.clone(),
            result: None,
        })
    });
    let snapshot = s.saved.snapshot.clone();
    let options = s.saved.options.clone();
    spawn_local(async move {
        let mut rows = Vec::new();
        for cap in 0..=11 {
            // Yield between bounded searches so fields and navigation stay usable.
            let promise = js_sys::Promise::new(&mut |resolve, _| {
                let _ = window().set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0);
            });
            let _ = JsFuture::from(promise).await;
            let current = STATE.with(|state| {
                let s = state.borrow();
                s.tab == "bossing"
                    && s.compact_open
                    && s.shared_setup.is_none()
                    && efficient_key(&s.saved.snapshot, &s.saved.options) == key
            }) && EFFICIENT
                .with(|cache| cache.borrow().as_ref().is_some_and(|c| c.key == key));
            if !current {
                EFFICIENT.with(|cache| {
                    let matches = cache.borrow().as_ref().is_some_and(|c| c.key == key);
                    if matches {
                        *cache.borrow_mut() = None;
                    }
                });
                return;
            }
            rows.push(crate::efficient::Candidate::new(crate::tradeoffs::budget(
                &snapshot, &options, cap,
            )));
        }
        EFFICIENT.with(|cache| {
            *cache.borrow_mut() = Some(EfficientCache {
                key,
                result: Some(rows),
            })
        });
        let edit = document().active_element().filter(|e| {
            ["INPUT", "TEXTAREA", "SELECT"].contains(&e.tag_name().as_str()) && !e.id().is_empty()
        });
        render_with_edit(edit.as_ref().map(Element::id).as_deref());
    });
}
thread_local! {
    static COMPARISON_TRIGGER: RefCell<Option<Element>> = const { RefCell::new(None) };
    static COMPARISON_DISMISS: Cell<u64> = const { Cell::new(0) };
}

fn live_number(id: &str) -> bool {
    if id.starts_with("risk-") {
        return true;
    }
    if id.starts_with("sim-") {
        return true;
    }
    [
        "max-switches",
        "switch-tolerance",
        "damage-tolerance",
        "prayer-tolerance",
        "switch-slots",
        "food",
        "potions",
        "raid-level",
        "budget-cap-left",
        "budget-cap-right",
        "spell-casts",
        "ammo-shots",
        "target-hp",
        "current-hitpoints",
    ]
    .contains(&id)
        || id.starts_with("priority-")
        || id.starts_with("skill-")
        || (id.starts_with("reduction-")
            && !["reduction-accursed", "reduction-vulnerability"].contains(&id))
}
fn apply_number(s: &mut State, id: &str, value: &str) -> bool {
    if s.shared_setup.is_some() && !id.starts_with("skill-") {
        return false;
    }
    if let Some(key) = id.strip_prefix("risk-") {
        if let Some((field, item)) = key
            .rsplit_once('-')
            .and_then(|(field, id)| Some((field, id.parse::<i32>().ok()?)))
        {
            let plan = s.plan();
            s.saved.death_risk.bind(&plan);
            return s.saved.death_risk.set(field, item, value);
        }
        return false;
    }
    if let Some(key) = id.strip_prefix("sim-") {
        return value
            .parse::<u32>()
            .ok()
            .is_some_and(|n| s.saved.options.simulation.set(key, n));
    }
    let target_id = comparison_monster(s).map(|m| m.id);
    let target_min = comparison_monster(s)
        .and_then(|m| combat::target_hp_bounds(m, s.saved.options.raid_level).ok())
        .map(|(min, _)| min)
        .unwrap_or(1);
    let target_max = comparison_monster(s)
        .and_then(|m| {
            crate::encounters::window(m.id)
                .and_then(|w| w.start_hp)
                .or_else(|| {
                    combat::target_hitpoints(m, s.saved.options.raid_level).map(|hp| hp as u32)
                })
        })
        .unwrap_or(100_000);
    if ["budget-cap-left", "budget-cap-right"].contains(&id) {
        let Ok(value) = value.parse::<usize>() else {
            return false;
        };
        s.saved.budget_caps[usize::from(id == "budget-cap-right")] = value.min(11);
        return true;
    }
    let o = &mut s.saved.options;
    if ["switch-tolerance", "damage-tolerance", "prayer-tolerance"].contains(&id) {
        let Ok(number) = value.parse::<i64>() else {
            return false;
        };
        let bounded = number.clamp(0, crate::efficient::MAX_TOLERANCE_PERCENT as i64) as u32;
        match id {
            "switch-tolerance" => o.switch_tolerance = bounded,
            "damage-tolerance" => o.damage_tolerance = bounded,
            _ => o.prayer_tolerance = bounded,
        }
        if number != bounded as i64 {
            if let Some(input) = document()
                .get_element_by_id(id)
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
            {
                input.set_value(&bounded.to_string());
            }
        }
        s.export_text = None;
        return true;
    }
    if id == "target-hp" && value.trim().is_empty() {
        if let Some(target) = target_id {
            o.combat.target_hp.remove(&target);
        }
        s.export_text = None;
        return true;
    }
    if id == "current-hitpoints" && value.trim().is_empty() {
        o.combat.current_hitpoints = None;
        s.export_text = None;
        return true;
    }
    if id == "max-switches" && value.trim().is_empty() {
        o.max_switches = None;
        o.fewer_switches = false;
        s.export_text = None;
        return true;
    }
    let Ok(n) = value.parse::<u32>() else {
        return false;
    };
    match id {
        "max-switches" => {
            o.max_switches = Some((n as usize).min(11));
            o.fewer_switches = false;
        }
        "switch-slots" => o.switch_slots = (n as usize).min(20),
        "food" => {
            o.food = (n as usize).min(20);
            o.inventory_overrides.clear();
            o.inventory_quantities.clear();
        }
        "potions" => {
            o.prayer_potions = (n as usize).min(12);
            o.inventory_overrides.clear();
            o.inventory_quantities.clear();
        }
        "raid-level" => o.raid_level = n.min(150),
        "spell-casts" => o.combat.spell_casts = n.clamp(1, 10_000),
        "target-hp" => {
            if let Some(target) = target_id {
                o.combat
                    .target_hp
                    .insert(target, n.clamp(target_min, target_max));
            }
        }
        "current-hitpoints" => o.combat.current_hitpoints = Some(n.clamp(1, 255)),
        "ammo-shots" => {
            let shots = n.clamp(1, 10_000);
            if o.combat.ammo_shots != shots {
                o.combat.loaded_ammo_confirmed = false;
            }
            o.combat.ammo_shots = shots;
        }
        _ if id.starts_with("reduction-") => {
            let Some(target) = target_id else {
                return false;
            };
            let r = o.combat.reductions.entry(target).or_default();
            match &id[10..] {
                "dwh" => r.dwh = n.min(20),
                "elder_maul" => r.elder_maul = n.min(20),
                "arclight" => r.arclight = n.min(20),
                "emberlight" => r.emberlight = n.min(20),
                "tonalztic" => r.tonalztic = n.min(20),
                "bgs" => r.bgs = n.min(10_000),
                "seercull" => r.seercull = n.min(10_000),
                "ayak" => r.ayak = n.min(10_000),
                _ => return false,
            }
            if r.is_empty() {
                o.combat.reductions.remove(&target);
            }
        }
        _ if id.starts_with("priority-") => {
            let Ok(target) = id[9..].parse::<i32>() else {
                return false;
            };
            if !encounter_targets(s).contains(&target) {
                return false;
            }
            s.saved.options.phase_priorities.insert(target, n.min(10));
        }
        _ if id.starts_with("skill-") => {
            let Ok(index) = id[6..].parse::<usize>() else {
                return false;
            };
            if index >= 24 || !(1..=99).contains(&n) {
                return false;
            }
            let selected = s.saved.snapshot.selected.clone();
            if let Some(m) = s
                .saved
                .snapshot
                .members
                .iter_mut()
                .find(|m| m.name == selected)
            {
                m.level_overrides.insert(SKILLS[index].into(), n);
            }
        }
        _ => return false,
    }
    s.export_text = None;
    true
}
/// Recalculate after a short typing pause; never require blur or steal focus.
fn schedule_number_render() {
    let revision = LIVE_EDIT.with(|value| {
        let next = value.get().wrapping_add(1);
        value.set(next);
        next
    });
    let callback = Closure::once_into_js(move || {
        if LIVE_EDIT.with(|value| value.get() != revision) {
            return;
        }
        let edit = document().active_element().filter(|e| {
            ["INPUT", "TEXTAREA", "SELECT"].contains(&e.tag_name().as_str()) && !e.id().is_empty()
        });
        STATE.with(|state| state.borrow().persist());
        // Refresh even after Tab moves to another field. Preserve that field,
        // rather than discarding this update or taking focus back.
        render_with_edit(edit.as_ref().map(Element::id).as_deref());
    });
    let _ = window()
        .set_timeout_with_callback_and_timeout_and_arguments_0(callback.unchecked_ref(), 250);
}
fn window() -> web_sys::Window {
    web_sys::window().unwrap()
}
fn document() -> web_sys::Document {
    window().document().unwrap()
}
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn input(id: &str) -> String {
    document()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        .map(|e| e.value())
        .unwrap_or_default()
}
fn text_input(id: &str) -> String {
    document()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|e| e.value())
        .unwrap_or_default()
}
fn image(id: i32) -> String {
    crate::tooltip::item_icon(id)
}

fn load_item_icons() {
    // Fetch the same complete public pack for everyone, never bank-derived URLs.
    // It must not block Wasm initialization or recalculate the user's plan.
    spawn_local(async {
        let load = async {
            let url = document()
                .get_element_by_id("item-icon-pack")
                .and_then(|el| el.get_attribute("content"))
                .ok_or(JsValue::NULL)?;
            let response = JsFuture::from(window().fetch_with_str(&url))
                .await?
                .dyn_into::<web_sys::Response>()?;
            if !response.ok() {
                return Err(JsValue::NULL);
            }
            let raw = JsFuture::from(response.text()?)
                .await?
                .as_string()
                .ok_or(JsValue::NULL)?;
            crate::tooltip::install_icons(&raw).map_err(|_| JsValue::NULL)?;
            let pending = document().query_selector_all("[data-item-icon]")?;
            for index in 0..pending.length() {
                if let Some(el) = pending
                    .item(index)
                    .and_then(|node| node.dyn_into::<Element>().ok())
                {
                    if let Some(id) = el
                        .get_attribute("data-item-icon")
                        .and_then(|id| id.parse().ok())
                    {
                        el.set_outer_html(&image(id));
                    }
                }
            }
            Ok::<_, JsValue>(())
        }
        .await;
        if load.is_err() {
            web_sys::console::warn_1(&JsValue::from_str(
                "Item artwork could not load; the planner is still available. Reload to retry.",
            ));
        }
    });
}

fn button(action: &str, label: &str, class: &str) -> String {
    format!(
        "<button type=\"button\" data-action=\"{}\" class=\"{}\">{}</button>",
        esc(action),
        class,
        label
    )
}
fn badge(label: &str, class: &str) -> String {
    format!("<span class=\"badge {class}\">{}</span>", esc(label))
}

fn hide_item_tooltip() {
    keep_item_tooltip();
    COMPARISON_TRIGGER.with(|trigger| {
        if let Some(item) = trigger.borrow_mut().take() {
            let _ = item.set_attribute("aria-expanded", "false");
        }
    });
    if let Some(tooltip) = document().get_element_by_id("item-explanation") {
        if tooltip.has_attribute("hidden") {
            return;
        }
        let _ = tooltip.set_attribute("hidden", "");
        tooltip.set_text_content(None);
    }
}

fn keep_item_tooltip() {
    COMPARISON_DISMISS.with(|version| version.set(version.get().wrapping_add(1)));
}

fn dismiss_item_tooltip_later() {
    let version = COMPARISON_DISMISS.with(|v| {
        v.set(v.get().wrapping_add(1));
        v.get()
    });
    let callback = Closure::once_into_js(move || {
        if COMPARISON_DISMISS.with(|v| v.get() == version) {
            hide_item_tooltip();
        }
    });
    let _ = window()
        .set_timeout_with_callback_and_timeout_and_arguments_0(callback.unchecked_ref(), 200);
}

fn comparison_contains(element: &Element) -> bool {
    element
        .closest("#item-explanation, [data-explanation]")
        .ok()
        .flatten()
        .is_some()
}

fn comparison_key(event: &web_sys::Event, key: &str) -> bool {
    let Some(panel) = document()
        .get_element_by_id("item-explanation")
        .filter(|p| !p.has_attribute("hidden"))
    else {
        return false;
    };
    let Some(active) = document().active_element() else {
        return false;
    };
    let inside = panel.contains(Some(&active));
    let trigger = COMPARISON_TRIGGER.with(|t| t.borrow().clone());
    if key == "Escape" && inside {
        event.prevent_default();
        if let Some(trigger) = trigger.and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
            let _ = trigger.focus();
        }
        hide_item_tooltip();
        return true;
    }
    if key != "Tab" {
        return false;
    }
    let backwards = js_sys::Reflect::get(event.as_ref(), &JsValue::from_str("shiftKey"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let Ok(buttons) = panel.query_selector_all("button[data-action]") else {
        return false;
    };
    if buttons.length() == 0 {
        return false;
    }
    if trigger.as_ref() == Some(&active) && !backwards {
        event.prevent_default();
        if let Some(first) = buttons
            .item(0)
            .and_then(|n| n.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = first.focus();
        }
        return true;
    }
    let edge = buttons
        .item(if backwards { 0 } else { buttons.length() - 1 })
        .and_then(|n| n.dyn_into::<Element>().ok());
    if inside && edge.as_ref() == Some(&active) {
        if let Some(trigger) = trigger.and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
            let _ = trigger.focus();
            if backwards {
                event.prevent_default();
            }
        }
        hide_item_tooltip();
        return true;
    }
    false
}

fn show_item_tooltip(item: &Element) {
    let Some(base) = item.get_attribute("data-explanation") else {
        return;
    };
    keep_item_tooltip();
    COMPARISON_TRIGGER.with(|trigger| {
        if let Some(prior) = trigger
            .borrow_mut()
            .replace(item.clone())
            .filter(|prior| prior != item)
        {
            let _ = prior.set_attribute("aria-expanded", "false");
        }
    });
    let _ = item.set_attribute("aria-expanded", "true");
    let text = item
        .get_attribute("data-complete-explanation")
        .unwrap_or_else(|| {
            let (choices, cap) = STATE.with(|state| {
                let s = state.borrow();
                if s.shared_setup.is_some() {
                    return (Vec::new(), None);
                }
                let p = s.plan();
                let action = item.get_attribute("data-action").unwrap_or_default();
                let choices = if let Some(slot) = action
                    .strip_prefix("edit-equipment:")
                    .and_then(|v| v.parse::<usize>().ok())
                {
                    crate::alternatives::equipment(
                        &s.saved.snapshot,
                        &s.saved.options,
                        &p,
                        s.phase,
                        slot,
                        2,
                    )
                } else if let Some(index) = action
                    .strip_prefix("edit-inventory:")
                    .and_then(|v| v.parse::<usize>().ok())
                {
                    p.preview_inventory(s.phase)
                        .get(index)
                        .map(|item| {
                            crate::alternatives::packed(
                                &s.saved.snapshot,
                                &s.saved.options,
                                &p,
                                item,
                                2,
                            )
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                (choices, s.saved.options.max_switches)
            });
            let current = item
                .get_attribute("data-item-id")
                .and_then(|id| id.parse().ok());
            let text = crate::tooltip::render(&base, current, &choices, cap);
            let _ = item.set_attribute("data-complete-explanation", &text);
            text
        });
    let Some(tooltip) = document().get_element_by_id("item-explanation") else {
        return;
    };
    let anchor = item.get_bounding_client_rect();
    let width = window()
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(320.0);
    let height = window()
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(480.0);
    let layout = format!("{}:{}:{width}:{height}:{text}", anchor.left(), anchor.top());
    if !tooltip.has_attribute("hidden")
        && tooltip.get_attribute("data-layout").as_deref() == Some(&layout)
    {
        return;
    }
    let _ = tooltip.set_attribute("data-layout", &layout);
    // Only our escaped renderer produces this cached markup.
    tooltip.set_inner_html(&text);
    let _ = tooltip.remove_attribute("hidden");
    let _ = tooltip.set_attribute("style", "left:0;top:0");
    let bounds = tooltip.get_bounding_client_rect();
    let position = crate::tooltip::position(
        (anchor.left(), anchor.top(), anchor.width(), anchor.height()),
        (width, height),
        (bounds.width(), bounds.height()),
    );
    let _ = tooltip.set_attribute(
        "style",
        &format!(
            "left:{}px;top:{}px;max-height:{}px",
            position.left, position.top, position.max_height
        ),
    );
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    render();
    load_item_icons();
    let explain = Closure::<dyn FnMut(web_sys::Event)>::new(|event: web_sys::Event| {
        if event
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .is_some_and(|e| e.closest("#item-explanation").ok().flatten().is_some())
        {
            keep_item_tooltip();
            return;
        }
        let item = event
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|e| e.closest("[data-explanation]").ok().flatten());
        if let Some(item) = item {
            show_item_tooltip(&item);
        } else if event.type_() == "mousemove" {
            if document()
                .active_element()
                .is_some_and(|e| comparison_contains(&e))
            {
                keep_item_tooltip();
            } else {
                dismiss_item_tooltip_later();
            }
        } else {
            hide_item_tooltip();
        }
    });
    // Pointer movement owns hover, while scroll-generated mouseover events
    // must not overwrite the explanation for a keyboard-focused item.
    for event in ["mousemove", "focusin"] {
        document().add_event_listener_with_callback(event, explain.as_ref().unchecked_ref())?;
    }
    explain.forget();
    let dismiss = Closure::<dyn FnMut(web_sys::Event)>::new(|event: web_sys::Event| {
        if let Some(mouse) = event.dyn_ref::<web_sys::MouseEvent>() {
            let item = event
                .target()
                .and_then(|t| t.dyn_into::<Element>().ok())
                .and_then(|e| {
                    e.closest("[data-explanation], #item-explanation")
                        .ok()
                        .flatten()
                });
            let Some(_) = item else { return };
            let related = mouse
                .related_target()
                .and_then(|t| t.dyn_into::<Element>().ok());
            if related.is_some_and(|related| comparison_contains(&related))
                || document()
                    .active_element()
                    .is_some_and(|active| comparison_contains(&active))
            {
                return;
            }
        }
        // Focus may move from the source item into an alternative button.
        dismiss_item_tooltip_later();
    });
    for event in ["mouseout", "focusout"] {
        document().add_event_listener_with_callback(event, dismiss.as_ref().unchecked_ref())?;
    }
    dismiss.forget();
    let click = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if !e
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .is_some_and(|el| comparison_contains(&el))
        {
            hide_item_tooltip();
        }
        let target = e
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|e| e.closest("[data-action]").ok().flatten());
        if let Some(el) = target {
            if let Some(action) = el.get_attribute("data-action") {
                let comparison_slot = el
                    .closest("#item-explanation")
                    .ok()
                    .flatten()
                    .and_then(|_| el.get_attribute("data-slot"))
                    .and_then(|slot| slot.parse::<usize>().ok());
                let return_slot = STATE.with(|s| s.borrow().edit_equipment);
                let return_inventory = STATE.with(|s| s.borrow().edit_inventory);
                act(&action);
                if let Some(slot) = comparison_slot {
                    focus_equipment(slot);
                    hide_item_tooltip();
                }
                if STATE.with(|s| s.borrow().edit_equipment.is_none()) {
                    if let Some(slot) = return_slot {
                        focus_equipment(slot);
                    }
                }
                if STATE.with(|s| s.borrow().edit_inventory.is_none()) {
                    if let Some(slot) = return_inventory {
                        focus_inventory(slot);
                    }
                }
            }
        }
    });
    document().add_event_listener_with_callback("click", click.as_ref().unchecked_ref())?;
    click.forget();
    let search = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if let Some(el) = e
            .target()
            .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
        {
            if el.id() == "encounter-search" {
                STATE.with(|state| {
                    let mut s = state.borrow_mut();
                    s.encounter_query = el.value();
                    refresh_encounter_browser(&s);
                });
            } else if el.id().starts_with("rank-") || el.id().starts_with("plan-rank-") {
                update_gear_ranking(&el.id(), &el.value());
            } else if el.id() == "inventory-search" {
                STATE.with(|state| {
                    let mut s = state.borrow_mut();
                    s.inventory_query = el.value();
                    if let Some(results) = document().get_element_by_id("inventory-pick-results") {
                        results.set_inner_html(&inventory_choices(&s));
                    }
                });
            } else if el.id() == "gear-search" {
                STATE.with(|state| {
                    let mut s = state.borrow_mut();
                    s.gear_search = el.value();
                    if let Some(results) = document().get_element_by_id("gear-pick-results") {
                        results.set_inner_html(&gear_choices(&s));
                    }
                });
            } else if live_number(&el.id()) {
                let changed = STATE
                    .with(|state| apply_number(&mut state.borrow_mut(), &el.id(), &el.value()));
                if changed {
                    schedule_number_render();
                }
            }
        }
    });
    document().add_event_listener_with_callback("input", search.as_ref().unchecked_ref())?;
    search.forget();
    let disclosure = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if let Some(el) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            if el.id() == "death-risk-details" {
                STATE.with(|state| state.borrow_mut().risk_open = el.has_attribute("open"));
            } else if el.id() == "boss-bestiary" {
                let open = el.has_attribute("open");
                let changed = STATE.with(|state| {
                    let mut s = state.borrow_mut();
                    let changed = s.bestiary_open != open;
                    s.bestiary_open = open;
                    changed
                });
                if changed && open {
                    render();
                }
            } else if el.id() == "raid-room-picker" {
                STATE.with(|state| state.borrow_mut().raid_rooms_open = el.has_attribute("open"));
            } else if el.id() == "switch-preferences" {
                STATE.with(|state| {
                    state.borrow_mut().switch_preferences_open = el.has_attribute("open")
                });
            } else if el.id() == "compact-setup" {
                let open = el.has_attribute("open");
                let changed = STATE.with(|state| {
                    let mut s = state.borrow_mut();
                    let changed = s.compact_open != open;
                    s.compact_open = open;
                    changed
                });
                if changed && open {
                    render();
                }
            } else if el.id() == "encounter-extra-filters" {
                STATE.with(|state| {
                    state.borrow_mut().encounter_filters_open = el.has_attribute("open")
                });
            } else if el.id() == "wilderness-options" {
                STATE.with(|state| {
                    state.borrow_mut().wilderness_options_open = el.has_attribute("open")
                });
            } else if el.id() == "packing-limits" {
                STATE.with(|state| {
                    state.borrow_mut().packing_limits_open = el.has_attribute("open")
                });
            } else if el.id() == "combat-assumptions" {
                STATE.with(|state| {
                    state.borrow_mut().combat_options_open = el.has_attribute("open")
                });
            } else if el.id() == "attack-budget" {
                STATE
                    .with(|state| state.borrow_mut().attack_budget_open = el.has_attribute("open"));
            } else if el.id() == "fight-simulation" {
                STATE.with(|state| state.borrow_mut().simulation_open = el.has_attribute("open"));
            } else if el.id() == "gameplay-calibration" {
                STATE.with(|state| state.borrow_mut().calibration_open = el.has_attribute("open"));
            } else if el.id() == "budget-phases" {
                STATE
                    .with(|state| state.borrow_mut().budget_phases_open = el.has_attribute("open"));
            } else if el.id() == "budget-comparison" {
                let changed = STATE.with(|state| {
                    let mut s = state.borrow_mut();
                    let open = el.has_attribute("open");
                    let changed = s.budget_comparison_open != open;
                    s.budget_comparison_open = open;
                    changed
                });
                if changed {
                    STATE.with(|state| state.borrow().persist());
                    render();
                }
            }
        }
    });
    document().add_event_listener_with_callback_and_bool(
        "toggle",
        disclosure.as_ref().unchecked_ref(),
        true,
    )?;
    disclosure.forget();
    let escape = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        let key = js_sys::Reflect::get(e.as_ref(), &JsValue::from_str("key"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default();
        if comparison_key(&e, &key) {
            return;
        }
        if key == "Escape" {
            hide_item_tooltip();
        }
        if key == "Tab"
            && STATE.with(|s| {
                let s = s.borrow();
                s.edit_equipment.is_some() || s.edit_inventory.is_some()
            })
        {
            if let Ok(Some(dialog)) = document().query_selector(".gear-picker") {
                if let Ok(nodes) = dialog
                    .query_selector_all("button:not([disabled]), input, select, summary, a[href]")
                {
                    let visible: Vec<Element> = (0..nodes.length())
                        .filter_map(|i| nodes.item(i)?.dyn_into::<Element>().ok())
                        .filter(|el| el.client_width() > 0 || el.client_height() > 0)
                        .filter(|el| {
                            let mut ancestor = el.parent_element();
                            while let Some(parent) = ancestor {
                                if parent.tag_name() == "DETAILS"
                                    && !parent.has_attribute("open")
                                    && !(el.tag_name() == "SUMMARY"
                                        && el.parent_element().as_ref() == Some(&parent))
                                {
                                    return false;
                                }
                                ancestor = parent.parent_element();
                            }
                            true
                        })
                        .collect();
                    if !visible.is_empty() {
                        let backwards =
                            js_sys::Reflect::get(e.as_ref(), &JsValue::from_str("shiftKey"))
                                .ok()
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                        let active = document().active_element();
                        let edge = visible.get(if backwards { 0 } else { visible.len() - 1 });
                        if active.is_none() || active.as_ref() == edge {
                            e.prevent_default();
                            if let Some(next) = visible
                                .get(if backwards { visible.len() - 1 } else { 0 })
                                .and_then(|n| n.dyn_ref::<web_sys::HtmlElement>())
                            {
                                let _ = next.focus();
                            }
                        }
                    }
                }
            }
        } else if key == "Escape"
            && STATE.with(|s| {
                let s = s.borrow();
                s.edit_equipment.is_some() || s.edit_inventory.is_some()
            })
        {
            let slot = STATE.with(|s| s.borrow().edit_equipment);
            let inventory = STATE.with(|s| s.borrow().edit_inventory);
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.edit_equipment = None;
                s.edit_inventory = None;
            });
            render();
            if let Some(slot) = slot {
                focus_equipment(slot);
            }
            if let Some(slot) = inventory {
                focus_inventory(slot);
            }
        }
    });
    document().add_event_listener_with_callback("keydown", escape.as_ref().unchecked_ref())?;
    escape.forget();
    let change = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        let Some(el) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let id = el.id();
        if id.starts_with("rank-") || id.starts_with("plan-rank-") {
            let value = el
                .dyn_ref::<HtmlInputElement>()
                .map(|e| e.value())
                .or_else(|| el.dyn_ref::<HtmlSelectElement>().map(|e| e.value()))
                .unwrap_or_default();
            update_gear_ranking(&id, &value);
            return;
        }
        if id == "recommendation-mode" {
            if let Some(select) = el.dyn_ref::<HtmlSelectElement>() {
                STATE.with(|state| {
                    let mut s = state.borrow_mut();
                    s.saved.options.recommended = Some(select.value() == "recommended");
                    s.ranking_draft = None;
                    s.persist();
                });
                render();
            }
            return;
        }
        if id == "calibration-file" {
            if let Ok(i) = el.dyn_into::<HtmlInputElement>() {
                if let Some(file) = i.files().and_then(|f| f.get(0)) {
                    if file.size() > crate::calibration::MAX_BYTES as f64 {
                        notice("Gameplay worksheet exceeds 4 MB.", true);
                        return;
                    }
                    spawn_local(async move {
                        let raw = JsFuture::from(file.text()).await;
                        STATE.with(|state| {
                            let mut s = state.borrow_mut();
                            let plan = s.plan();
                            let key = simulation_key(&s, &plan);
                            let result = raw.map_err(|_| "Could not read gameplay worksheet.".to_string())
                                .and_then(|v| crate::calibration::parse(&v.as_string().unwrap_or_default()))
                                .and_then(|data| {
                                    let current = crate::simulation::Model::from_plan(&s.saved.snapshot, &s.saved.options, &plan)?;
                                    if !crate::calibration::same_inputs(&current, &data.inputs) {
                                        return Err("Worksheet uses a different setup or assumptions. Restore its original setup before comparing.".into());
                                    }
                                    crate::calibration::compare(&data)
                                });
                            s.calibration_result = Some((key, result));
                            s.calibration_open = true;
                        });
                        render();
                    });
                }
            }
            return;
        }
        if id == "import-file" {
            if let Ok(i) = el.dyn_into::<HtmlInputElement>() {
                if let Some(file) = i.files().and_then(|f| f.get(0)) {
                    if file.size() > 2_000_000.0 {
                        notice("File exceeds the 2 MB import limit.", true);
                        return;
                    }
                    spawn_local(async move {
                        match JsFuture::from(file.text()).await {
                            Ok(raw) => import_raw(&raw.as_string().unwrap_or_default()),
                            Err(_) => notice("Could not read this file.", true),
                        }
                    });
                }
            }
            return;
        }
        if id == "phase-select" {
            if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
                let value = select.value();
                act(&if value == "start" {
                    "starting".into()
                } else {
                    format!("phase:{value}")
                });
            }
            return;
        }
        if live_number(&id) {
            let changed =
                STATE.with(|state| apply_number(&mut state.borrow_mut(), &id, &input(&id)));
            if changed {
                schedule_number_render();
            }
            return;
        }
        // Draft form fields must survive blur: they are committed by their action button.
        if ![
            "max-switches",
            "switch-slots",
            "focus-style",
            "budget-window",
            "budget-incoming",
            "mark-darkness",
            "loaded-dart",
            "blue-moon-melee",
            "loaded-ammo-confirmed",
            "boost-melee",
            "boost-ranged",
            "boost-magic",
            "boost-defence",
            "boost-encounter",
            "reduction-accursed",
            "reduction-vulnerability",
            "kandarin-diary",
            "soulreaper-stacks",
            "bulwark-block",
            "obligator-style",
            "special-energy",
            "zorya-followups",
            "allow-void",
            "safety",
            "prayer-weight",
            "member",
            "shared",
            "fewer",
            "food",
            "potions",
            "raid-level",
            "access",
            "started",
            "inventory-filter",
        ]
        .contains(&id.as_str())
            && !id.starts_with("skill-")
            && !id.starts_with("gear-")
            && !id.starts_with("quest-")
            && !id.starts_with("priority-")
            && !id.starts_with("reduction-")
        {
            return;
        }
        STATE.with(|state| {
            let mut s = state.borrow_mut();
            if s.shared_setup.is_some() && !id.starts_with("skill-") {
                s.notice(
                    "Choose Rebuild using my items before editing this shared setup.",
                    true,
                );
                return;
            }
            match id.as_str() {
                "blue-moon-melee" => {
                    if let Some(sel) = el.dyn_ref::<HtmlSelectElement>() {
                        let value = sel.value();
                        if ["Off", "Stab", "Slash", "Crush"].contains(&value.as_str()) {
                            s.saved.options.combat.blue_moon_melee = value;
                        }
                    }
                }
                "budget-window" => {
                    if let Some(sel) = el.dyn_ref::<HtmlSelectElement>() {
                        s.budget_window = Some(sel.value());
                        s.budget_incoming = None;
                    }
                }
                "budget-incoming" => {
                    if let Some(sel) = el.dyn_ref::<HtmlSelectElement>() {
                        s.budget_incoming = Some(sel.value());
                    }
                }
                "loaded-dart" => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        s.saved.options.combat.loaded_dart = sel
                            .value()
                            .parse::<i32>()
                            .ok()
                            .filter(|id| crate::rules::all_internal_ammo_ids().contains(id));
                        s.saved.options.combat.loaded_ammo_confirmed = false;
                    }
                }
                "loaded-ammo-confirmed" => {
                    s.saved.options.combat.loaded_ammo_confirmed =
                        s.saved.options.combat.loaded_dart.is_some()
                            && el
                                .dyn_ref::<HtmlInputElement>()
                                .is_some_and(|e| e.checked())
                }
                _ if id.starts_with("boost-") => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        let value = sel.value();
                        let channel = &id[6..];
                        if value == "None"
                            || crate::modifiers::boost_options(channel)
                                .iter()
                                .any(|b| b.id == value)
                        {
                            let b = &mut s.saved.options.combat.boosts;
                            if channel == "magic" && value == "Forgotten brew" {
                                b.melee = "None".into();
                                b.defence = "None".into();
                                b.encounter = "None".into();
                            } else if ["melee", "defence", "encounter"].contains(&channel)
                                && value != "None"
                                && b.magic == "Forgotten brew"
                            {
                                b.magic = "None".into();
                            }
                            match channel {
                                "melee" => b.melee = value,
                                "ranged" => b.ranged = value,
                                "magic" => b.magic = value,
                                "defence" => b.defence = value,
                                "encounter" => b.encounter = value,
                                _ => {}
                            }
                        }
                    }
                }
                "reduction-accursed" | "reduction-vulnerability" => {
                    if let Some(target) = comparison_monster(&s).map(|m| m.id) {
                        let checked = el
                            .dyn_ref::<HtmlInputElement>()
                            .is_some_and(|e| e.checked());
                        let r = s.saved.options.combat.reductions.entry(target).or_default();
                        if id == "reduction-accursed" {
                            r.accursed = checked;
                        } else {
                            r.vulnerability = checked;
                        }
                        if r.is_empty() {
                            s.saved.options.combat.reductions.remove(&target);
                        }
                    }
                }
                "inventory-filter" => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        s.inventory_filter = sel.value();
                    }
                }
                "mark-darkness" => {
                    s.saved.options.combat.mark_darkness = el
                        .dyn_ref::<HtmlInputElement>()
                        .is_some_and(|e| e.checked())
                }
                "kandarin-diary" => {
                    s.saved.options.combat.kandarin_diary = el
                        .dyn_ref::<HtmlInputElement>()
                        .is_some_and(|e| e.checked())
                }
                "soulreaper-stacks" => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        s.saved.options.combat.soulreaper_stacks =
                            sel.value().parse::<u32>().unwrap_or(0).min(5);
                    }
                }
                "bulwark-block" => {
                    s.saved.options.combat.bulwark_block = el
                        .dyn_ref::<HtmlInputElement>()
                        .is_some_and(|e| e.checked());
                }
                "obligator-style" => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        let value = sel.value();
                        if ["Thrust accurate", "Thrust aggressive", "Smash"]
                            .contains(&value.as_str())
                        {
                            s.saved.options.combat.obligator_style = value;
                        }
                    }
                }
                "special-energy" | "zorya-followups" => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        if let Ok(value) = sel.value().parse::<u32>() {
                            if id == "special-energy" && value <= 100 {
                                s.saved.options.combat.special_energy = value;
                            }
                            if id == "zorya-followups" && value <= 3 {
                                s.saved.options.combat.zorya_empowered_hits = value;
                            }
                        }
                    }
                }
                "allow-void" => {
                    s.saved.options.exclude_void = !el
                        .dyn_ref::<HtmlInputElement>()
                        .is_some_and(|e| e.checked());
                }
                "safety" => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        s.saved.options.combat.safety =
                            sel.value().parse::<u32>().unwrap_or(1).min(3);
                    }
                }
                "prayer-weight" => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        s.saved.options.combat.prayer_weight =
                            sel.value().parse::<u32>().unwrap_or(0).min(3);
                    }
                }
                "focus-style" => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        let value = sel.value();
                        if value == "Balanced" || crate::rules::STYLES.contains(&value.as_str()) {
                            s.saved.options.focus_style = value;
                        }
                    }
                }
                _ if id.starts_with("quest-") => {
                    if let (Ok(i), Ok(sel)) =
                        (id[6..].parse::<usize>(), el.dyn_into::<HtmlSelectElement>())
                    {
                        if let Some(quest) = planner::GEAR_QUESTS.get(i) {
                            let selected = s.saved.snapshot.selected.clone();
                            if let Some(m) = s
                                .saved
                                .snapshot
                                .members
                                .iter_mut()
                                .find(|m| m.name == selected)
                            {
                                match sel.value().as_str() {
                                    "yes" => {
                                        m.quest_overrides.insert((*quest).into(), 2);
                                    }
                                    "no" => {
                                        m.quest_overrides.insert((*quest).into(), 1);
                                    }
                                    _ => {
                                        m.quest_overrides.remove(*quest);
                                    }
                                }
                            }
                        }
                    }
                }
                "member" => {
                    if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
                        s.saved.snapshot.selected = select.value();
                        s.saved.death_risk = Default::default();
                        s.saved.options.access_override = None;
                        s.saved.options.started_barrows = None;
                        s.phase = None;
                    }
                }
                "shared" => {
                    s.saved.options.shared = el
                        .dyn_ref::<HtmlInputElement>()
                        .is_some_and(|e| e.checked())
                }
                "fewer" => {
                    s.saved.options.fewer_switches = el
                        .dyn_ref::<HtmlInputElement>()
                        .is_some_and(|e| e.checked())
                }
                "access" => {
                    if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
                        s.saved.options.access_override = match select.value().as_str() {
                            "yes" => Some(true),
                            "no" => Some(false),
                            _ => None,
                        };
                    }
                }
                "started" => {
                    if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
                        s.saved.options.started_barrows = match select.value().as_str() {
                            "yes" => Some(true),
                            "no" => Some(false),
                            _ => None,
                        };
                    }
                }
                _ if id.starts_with("gear-") => {
                    if let Ok(sel) = el.dyn_into::<HtmlSelectElement>() {
                        if let Ok(value) = sel.value().parse::<i32>() {
                            s.saved.options.gear_overrides.insert(id[5..].into(), value);
                        }
                    }
                }
                _ => return,
            }
            s.export_text = None;
            s.persist();
        });
        render();
    });
    document().add_event_listener_with_callback("change", change.as_ref().unchecked_ref())?;
    change.forget();
    let hash_change = Closure::<dyn FnMut(web_sys::Event)>::new(|_| {
        let hash = window().location().hash().unwrap_or_default();
        STATE.with(|state| {
            let mut s = state.borrow_mut();
            if hash.starts_with("#setup=") {
                match share::Setup::parse(&hash) {
                    Ok(setup) => {
                        s.saved.options = setup.options();
                        s.shared_setup = Some(setup);
                        s.phase = None;
                    }
                    Err(e) => s.notice(e, true),
                }
            } else {
                s.shared_setup = None;
            }
        });
        render();
    });
    window()
        .add_event_listener_with_callback("hashchange", hash_change.as_ref().unchecked_ref())?;
    hash_change.forget();
    Ok(())
}

fn notice(message: &str, error: bool) {
    STATE.with(|s| s.borrow_mut().notice(message, error));
    render();
}
fn focus_equipment(slot: usize) {
    if let Ok(Some(button)) =
        document().query_selector(&format!("[data-action='edit-equipment:{slot}']"))
    {
        if let Ok(button) = button.dyn_into::<web_sys::HtmlElement>() {
            let _ = button.focus();
        }
    }
}
fn focus_inventory(slot: usize) {
    if let Ok(Some(button)) =
        document().query_selector(&format!("[data-action='edit-inventory:{slot}']"))
    {
        if let Ok(button) = button.dyn_into::<web_sys::HtmlElement>() {
            let _ = button.focus();
        }
    }
}
fn act(action: &str) {
    if action.starts_with("risk-") {
        STATE.with(|state| {
            let mut s = state.borrow_mut();
            if s.shared_setup.is_some() {
                return;
            }
            let plan = s.plan();
            s.saved.death_risk.bind(&plan);
            if let Some(scenario) = action
                .strip_prefix("risk-scenario:")
                .and_then(crate::death_risk::Scenario::parse)
            {
                s.saved.death_risk.scenario = scenario;
                s.saved.death_risk.kept.clear();
            } else if action == "risk-kept-mode" {
                s.saved.death_risk.manual_kept = !s.saved.death_risk.manual_kept;
                s.saved.death_risk.kept.clear();
            } else if action == "risk-reset" {
                s.saved.death_risk = Default::default();
            } else if let Some(id) = action
                .strip_prefix("risk-keep:")
                .and_then(|s| s.parse::<i32>().ok())
            {
                let report = crate::death_risk::report(&plan, &s.saved.death_risk);
                if let Some(row) = report.rows.iter().find(|r| r.id == id) {
                    let maximum = row.quantity.min(report.slots);
                    let current = s.saved.death_risk.kept.get(&id).copied().unwrap_or(0);
                    s.saved
                        .death_risk
                        .kept
                        .insert(id, if current >= maximum { 0 } else { current + 1 });
                }
            }
            s.risk_open = true;
            s.persist();
        });
        render();
        return;
    }

    if action == "wilderness-mode"
        || action.starts_with("wilderness-location:")
        || action.starts_with("wilderness-cap:")
    {
        STATE.with(|state| {
            let mut s = state.borrow_mut();
            if action == "wilderness-mode" {
                s.scaredy_cat = !s.scaredy_cat;
                if s.scaredy_cat {
                    s.wilderness_location = "wild".into();
                }
            } else if let Some(location) = action.strip_prefix("wilderness-location:") {
                if ["all", "safe", "wild"].contains(&location) {
                    s.wilderness_location = location.into();
                }
            } else if let Some(cap) = action
                .strip_prefix("wilderness-cap:")
                .and_then(|v| v.parse::<u32>().ok())
            {
                if crate::wilderness::CAPS.contains(&cap) {
                    s.filler_cap = cap;
                }
            }
            s.bestiary_open = true;
        });
        render();
        return;
    }

    if action.starts_with("gear-filter:")
        || action.starts_with("requirements-filter:")
        || action.starts_with("encounter-filter:")
    {
        STATE.with(|state| {
            let mut s = state.borrow_mut();
            if let Some(filter) = action
                .strip_prefix("gear-filter:")
                .and_then(crate::bestiary::GearFilter::parse)
            {
                s.gear_filter = filter;
            } else if let Some(filter) = action
                .strip_prefix("requirements-filter:")
                .and_then(crate::bestiary::RequirementsFilter::parse)
            {
                s.requirements_filter = filter;
            } else if let Some(filter) = action
                .strip_prefix("encounter-filter:")
                .filter(|v| crate::bestiary::CATEGORY_FILTERS.contains(v))
            {
                s.encounter_filter = filter.into();
            } else {
                return;
            }
            refresh_encounter_browser(&s);
        });
        return;
    }
    match action {
        "clear-encounters" => {
            STATE.with(|state| {
                let mut s = state.borrow_mut();
                s.encounter_query.clear();
                s.encounter_filter = "For you".into();
                s.gear_filter = Default::default();
                s.requirements_filter = Default::default();
                s.wilderness_location = "all".into();
                if let Some(input) = document()
                    .get_element_by_id("encounter-search")
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                {
                    input.set_value("");
                }
                refresh_encounter_browser(&s);
            });
            render();
            return;
        }
        "load-levels" => {
            if STATE.with(|s| s.borrow().demo_return.is_some()) {
                notice("Return to your account to look up your levels.", false);
                return;
            }
            let name = input("player-name").trim().to_string();
            if name.is_empty()
                || name.len() > 12
                || !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || " _-".contains(c))
            {
                notice(
                    "Enter a RuneScape name of up to 12 letters, numbers or spaces.",
                    true,
                );
                return;
            }
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.busy = true;
                s.notice("Looking up public combat levels…", false);
            });
            render();
            spawn_local(lookup_skills(name));
            return;
        }
        "share" => {
            let result = STATE.with(|s| {
                let s = s.borrow();
                share::Setup::from_plan(&s.plan(), &s.saved.options).fragment()
            });
            match result {
                Ok(hash) => {
                    let base = window()
                        .location()
                        .href()
                        .unwrap_or_default()
                        .split('#')
                        .next()
                        .unwrap_or_default()
                        .to_string();
                    let url = format!("{base}{hash}");
                    if STATE.with(|s| s.borrow().demo_return.is_none()) {
                        let _ = window()
                            .history()
                            .and_then(|h| h.replace_state_with_url(&JsValue::NULL, "", Some(&url)));
                    }
                    STATE.with(|s| s.borrow_mut().export_text = Some(url.clone()));
                    spawn_local(async move {
                        let copied =
                            JsFuture::from(window().navigator().clipboard().write_text(&url))
                                .await
                                .is_ok();
                        notice(
                            if copied {
                                "Setup link copied with carried items and combat settings. Your bank and levels stay private."
                            } else {
                                "Copy the setup link shown below. Your bank remains private."
                            },
                            false,
                        );
                    });
                }
                Err(e) => notice(&e, true),
            }
            render();
            return;
        }
        "use-bank" => {
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.shared_setup = None;
                s.persist();
            });
            let base = window().location().pathname().unwrap_or_default();
            let _ = window()
                .history()
                .and_then(|h| h.replace_state_with_url(&JsValue::NULL, "", Some(&base)));
            render();
            return;
        }
        "import" => {
            import_raw(&text_input("import-data"));
            return;
        }
        "copy" => {
            let content = STATE.with(|s| {
                let s = s.borrow();
                let p = s.plan();
                export::inventory_setup(&p, &s.saved.options, &s.saved.snapshot.selected)
            });
            match content {
                Ok(text) => {
                    STATE.with(|s| s.borrow_mut().export_text = Some(text.clone()));
                    spawn_local(async move {
                        if JsFuture::from(window().navigator().clipboard().write_text(&text))
                            .await
                            .is_ok()
                        {
                            notice(
                                "Setup copied. In Inventory Setups, choose Import setup and paste.",
                                false,
                            );
                        } else {
                            notice(
                                "Clipboard access unavailable. Copy the setup text below or download it.",
                                true,
                            );
                        }
                    });
                }
                Err(e) => notice(&e, true),
            }
            render();
            return;
        }
        "bank-tags" => {
            let result = STATE.with(|s| {
                let s = s.borrow();
                export::bank_tags(&s.plan(), &s.saved.options)
            });
            match result {
                Ok(text) => {
                    STATE.with(|s| s.borrow_mut().export_text = Some(text.clone()));
                    spawn_local(async move {
                        let _ = JsFuture::from(window().navigator().clipboard().write_text(&text))
                            .await;
                        notice(
                            "Bank Tags text ready. Right-click the bank’s + tab and choose Import tag tab. Tags do not store quantities.",
                            false,
                        );
                    });
                }
                Err(e) => notice(&e, true),
            }
            render();
            return;
        }
        "download-setup" => {
            let result = STATE.with(|s| {
                let s = s.borrow();
                export::inventory_setup(&s.plan(), &s.saved.options, &s.saved.snapshot.selected)
            });
            match result {
                Ok(text) => download("ironbound-inventory-setup.json", &text),
                Err(e) => notice(&e, true),
            }
            return;
        }
        "backup" => {
            let text =
                STATE.with(|s| serde_json::to_string_pretty(&s.borrow().saved).unwrap_or_default());
            download("ironbound-account-backup.json", &text);
            return;
        }
        "backup-unreadable" => {
            if let Some(raw) = STATE.with(|s| s.borrow().unreadable_save.clone()) {
                download("ironbound-unreadable-backup.json", &raw);
            }
            return;
        }
        "retry-save" => {
            if STATE.with(|s| s.borrow().unreadable_save.is_some()) {
                let _ = window().location().reload();
            } else {
                STATE.with(|s| s.borrow().persist());
                render();
            }
            return;
        }
        "forget" => {
            if STATE.with(|s| s.borrow().demo_return.is_some()) {
                notice("Return to your account before removing saved data.", true);
                return;
            }
            if !window().confirm_with_message("Remove the saved account and goals from this browser? Your bank export file will not be changed.").unwrap_or(false){return;}
            if let Some(storage) = window().local_storage().ok().flatten() {
                let _ = storage.remove_item(STORAGE);
            }
            let path = window().location().pathname().unwrap_or_default();
            let _ = window()
                .history()
                .and_then(|h| h.replace_state_with_url(&JsValue::NULL, "", Some(&path)));
            STATE.with(|s| *s.borrow_mut() = State::initial());
            render();
            return;
        }
        _ => {}
    }
    STATE.with(|state| {
        let mut s = state.borrow_mut();
        s.message.clear();
        s.export_text = None;
        if action == "wilderness-off" {
            s.saved.options.wilderness=None; s.saved.options.gear_overrides.clear();
            s.notice("Returned to unrestricted gear planning.",false);
        } else if action == "save-named" {
            let options = s.saved.options.clone();
            let result = s.saved.setups.save(&input("setup-name"), &options);
            s.setups_open = true;
            match result { Ok(()) => s.notice("Named setup added. Check the device save status before leaving.", false), Err(e) => s.notice(e, true) }
        } else if let Some(choice) = action.strip_prefix("named:") {
            s.setups_open = true;
            if let Some((verb, index)) = choice.split_once(':') {
                if let Ok(index) = index.parse::<usize>() {
                    if let Some(entry) = s.saved.setups.0.get(index).cloned() {
                        match verb {
                            "load" => {
                                s.saved.options = entry.options;
                                s.shared_setup = None;
                                s.phase = None;
                                s.edit_style = None;
                                s.compare_target = None;
                                s.view = "loadout".into();
                                s.notice(format!("Loaded {}. Ownership, levels and packing rechecked against your current bank.", entry.name), false);
                            }
                            "rename" => {
                                match s.saved.setups.rename(index, &input("setup-name")) {
                                    Ok(()) => s.notice("Setup renamed.", false), Err(e) => s.notice(e, true)
                                }
                            }
                            "replace" => {
                                if window().confirm_with_message(&format!("Replace {} with the current configuration?", entry.name)).unwrap_or(false) {
                                    let options = s.saved.options.clone();
                                    match s.saved.setups.replace(index, &options) {
                                        Ok(()) => s.notice("Named setup updated.", false), Err(e) => s.notice(e, true)
                                    }
                                }
                            }
                            "remove" => {
                                if window().confirm_with_message(&format!("Remove {} from this device? Download a private backup first if you want to restore it later.", entry.name)).unwrap_or(false) {
                                    s.saved.setups.0.remove(index);
                                    s.notice("Named setup removed.", false);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        } else if let Some(id) = action.strip_prefix("confirm:") {
            if crate::preparation::CONFIRMATIONS.iter().any(|(key, _)| *key == id) {
                if crate::preparation::confirmed(&s.saved.snapshot, &s.saved.options, id) {
                    s.saved.options.confirmations.remove(id);
                } else {
                    let token = crate::preparation::token(&s.saved.snapshot, &s.saved.options);
                    s.saved.options.confirmations.insert(id.into(), token);
                }
            }
        } else if let Some(value)=action.strip_prefix("simulation-warden:").and_then(|v|v.parse::<u32>().ok()) {
            s.saved.options.simulation.warden=value.min(1);
        } else if let Some(value)=action.strip_prefix("simulation-burn-reset:").and_then(|v|v.parse::<u32>().ok()) {
            s.saved.options.simulation.burn_phase_resets=value.min(1);
        } else if action == "simulate-fight" {
            let plan=s.plan();
            let key=simulation_key(&s,&plan);
            let report=crate::simulation::Model::from_plan(&s.saved.snapshot,&s.saved.options,&plan).and_then(|m|crate::simulation::simulate(&m));
            s.simulation_result=Some((key,report));s.simulation_open=true;
        } else if action == "download-simulation" {
            let plan=s.plan();
            if let Some((key,Ok(report)))=&s.simulation_result {
                if *key==simulation_key(&s,&plan) {if let Ok(raw)=serde_json::to_string_pretty(report) {download("ironbound-fight-scenario.json",&raw);}}
            }
        } else if action == "download-gameplay-worksheet" {
            let plan=s.plan();
            if let Some((key,Ok(report)))=&s.simulation_result {
                if *key==simulation_key(&s,&plan) {
                    if let Ok(raw)=serde_json::to_string_pretty(&crate::calibration::worksheet(&report.inputs)) {
                        download("ironbound-gameplay-worksheet.json",&raw);
                    }
                }
            }
        } else if action == "download-calibration" {
            let plan=s.plan();
            if let Some((key,Ok(report)))=&s.calibration_result {
                if *key==simulation_key(&s,&plan) {
                    if let Ok(raw)=serde_json::to_string_pretty(report) {download("ironbound-gameplay-comparison.json",&raw);}
                }
            }
        } else if action == "reset-simulation" {
            s.saved.options.simulation=Default::default();s.simulation_result=None;s.calibration_result=None;
        } else if action == "reset-target-hp" {
            if let Some(target) = comparison_monster(&s).map(|m| m.id) {
                s.saved.options.combat.target_hp.remove(&target);
            }
        } else if action == "reset-all-inventory" {
            s.saved.options.inventory_overrides.clear();
            s.saved.options.inventory_quantities.clear();
            s.notice("Manual inventory edits cleared. Automatic packing restored.", false);
        } else if let Some(tab) = action.strip_prefix("tab:") {
            s.tab = tab.into();
            s.edit_equipment = None;
            s.edit_inventory = None;
        } else if let Some(view) = action.strip_prefix("view:") {
            if ["loadout", "combat", "prepare"].contains(&view) {
                s.view = view.into();
                s.edit_equipment = None;
            }
        } else if action=="efficient-apply" || action=="efficient-compare" {
            if s.shared_setup.is_none() {
                if let Some(result)=efficient_result(&s) {
                    if let Some((compact,best))=result.compact.zip(result.best) {
                        let cap=result.candidates[compact].budget.cap;
                        if action=="efficient-compare" {
                            s.saved.budget_caps=[cap,result.candidates[best].budget.cap];
                            s.budget_comparison_open=true;
                            s.compact_open=false;
                        } else {
                            apply_number(&mut s,"max-switches",&cap.to_string());
                            s.saved.options.inventory_overrides.clear();
                            s.saved.options.inventory_quantities.clear();
                            s.phase=None; s.view="loadout".into();
                            s.notice("Compact setup applied. Automatic inventory packing restored.",false);
                        }
                    }
                }
            }
        } else if let Some(side) = action.strip_prefix("budget-apply:") {
            let index = usize::from(side == "right");
            if side == "left" || side == "right" {
                let cap = s.saved.budget_caps[index];
                let comparison = crate::tradeoffs::compare(
                    &s.saved.snapshot,
                    &s.saved.options,
                    s.saved.budget_caps[0],
                    s.saved.budget_caps[1],
                );
                let choice = if index == 0 {
                    &comparison.left
                } else {
                    &comparison.right
                };
                if choice.feasible && s.shared_setup.is_none() {
                    apply_number(&mut s, "max-switches", &cap.to_string());
                    s.phase = None;
                    s.view = "loadout".into();
                    s.budget_comparison_open = false;
                    s.notice(&format!("Applied the {cap}-item switch cap."), false);
                }
            }
        } else if let Some(selection) = action.strip_prefix("slayer-task:") {
            if s.shared_setup.is_none() {
                if let Some((id, value)) = selection.split_once(':') {
                    if let Ok(id) = id.parse::<i32>() {
                        if encounter_targets(&s).contains(&id) && ["on", "off"].contains(&value) {
                            crate::slayer::set_task(&mut s.saved.options.combat, id, value == "on");
                        }
                    }
                }
            }
        } else if let Some(selection) = action.strip_prefix("raid-room:") {
            if s.shared_setup.is_some() {
                s.notice("Rebuild using your items to change raid bosses.", true);
            } else if let Some((raid_id, room_id)) = selection.split_once(':') {
                if let Some(raid) = crate::raids::raid(raid_id).filter(|raid| {
                    crate::raids::for_targets(&s.saved.options.targets)
                        .is_some_and(|current| current.id == raid.id)
                }) {
                    match raid.toggle(&s.saved.options.targets, room_id) {
                        Ok(targets) => {
                            s.saved.options.targets = targets;
                            s.saved.options.inventory_overrides.clear();
                            s.saved.options.inventory_quantities.clear();
                            s.phase = None;
                            s.edit_style = None;
                            s.compare_target = None;
                            s.edit_equipment = None;
                            s.raid_rooms_open = true;
                        }
                        Err(reason) => s.notice(&reason, true),
                    }
                }
            }
        } else if action == "custom-done" {
            s.building_group = false;
        } else if let Some(boss) = action.strip_prefix("boss:") {
            s.saved.options.wilderness=None;
            s.saved.options.boss = if boss == "toa" {
                Boss::Toa
            } else if boss == "custom" {
                Boss::Custom
            } else {
                Boss::Barrows
            };
            s.saved.options.access_override = None;
            s.saved.options.started_barrows = None;
            s.saved.options.gear_overrides.clear();
            s.saved.options.inventory_overrides.clear();
            s.saved.options.inventory_quantities.clear();
            s.shared_setup = None;
            s.phase = None;
            s.edit_style = None;
            s.compare_target = None;
        } else if action == "load-encounter" || action.starts_with("choose-encounter:") {
            let name = input("encounter-search");
            let direct = action
                .strip_prefix("choose-encounter:")
                .and_then(|id| id.parse::<usize>().ok())
                .and_then(|index| crate::encounters::catalog().get(index));
            if let Some(entry) = direct.or_else(|| {
                crate::encounters::catalog()
                    .iter()
                    .find(|e| e.name.eq_ignore_ascii_case(name.trim()))
            }) {
                if entry.targets.is_empty() {
                    s.notice(&entry.note, true);
                } else {
                    let restricted_preview = if s.scaredy_cat && crate::wilderness::exposure(&entry.name) != crate::wilderness::Exposure::None {
                        let index=crate::encounters::catalog().iter().position(|e|e.name==entry.name).unwrap();
                        let Some(preview)=bestiary_assessments(&s)[index].preview.clone() else { s.notice("No supported three-item kit is available for this encounter.",true); return; };
                        Some(preview)
                    } else { None };
                    s.saved.options.wilderness=None;
                    s.saved.options.boss = match entry.name.as_str() {
                        "Barrows Chests" => Boss::Barrows,
                        "Tombs of Amascut" => Boss::Toa,
                        _ => Boss::Custom,
                    };
                    s.saved.options.targets = entry.targets.clone();
                    if crate::guidance::profile(&s.saved.options)
                        .is_some_and(|profile| profile.recommended_mark_darkness)
                    {
                        s.saved.options.combat.mark_darkness = s
                            .saved
                            .snapshot
                            .selected()
                            .and_then(|m| m.level("Magic"))
                            .is_some_and(|level| level >= 59);
                    }
                    s.saved.options.gear_overrides.clear();
                    s.saved.options.targets = planner::recommend_encounter(
                        &s.saved.snapshot,
                        &s.saved.options,
                        &s.saved.options.targets,
                    );
                    s.saved.options.inventory_overrides.clear();
                    s.saved.options.inventory_quantities.clear();
                    s.shared_setup = None;
                    s.phase = None;
                    s.edit_style = None;
                    s.compare_target = None;
                    s.edit_equipment = None;
                    s.view = "loadout".into();
                    s.building_group = false;
                    s.raid_rooms_open = false;
                    s.bestiary_open = false;
                    if let Some(preview) = restricted_preview { crate::wilderness::apply(&mut s.saved.options,entry,&preview); }
                }
            } else {
                s.notice("Choose a listed boss, raid or encounter.", true);
            }
        } else if action == "compare-owned-styles" {
            s.saved.options.targets = planner::recommend_encounter(
                &s.saved.snapshot,
                &s.saved.options,
                &s.saved.options.targets,
            );
            s.phase = None;
            s.edit_style = None;
            s.notice("Compared complete owned loadouts for this boss.", false);
        } else if let Some(id) = action
            .strip_prefix("goal-item:")
            .and_then(|v| v.parse::<i32>().ok())
        {
            if planner::gear(id).is_some() {
                let key = goal_key(&s.saved.snapshot.selected, &s.saved.options);
                s.saved.pins.insert(key, vec![format!("item:{id}")]);
                s.edit_equipment = None;
                s.notice("Upgrade goal saved for this encounter.", false);
            }
        } else if action == "close-picker" {
            s.edit_equipment = None;
        } else if let Some(slot) = action
            .strip_prefix("edit-equipment:")
            .and_then(|v| v.parse::<usize>().ok())
        {
            if slot < 14 && s.shared_setup.is_none() {
                s.edit_inventory = None;
                let plan = s.plan();
                let style = s
                    .phase
                    .and_then(|i| plan.phases.get(i))
                    .map(|phase| {
                        if phase.name == "Akkha" {
                            "Magic"
                        } else if phase.name == "Wardens" {
                            "Ranged"
                        } else {
                            phase.style.as_str()
                        }
                    })
                    .unwrap_or(planner::primary_style(&s.saved.options))
                    .to_string();
                s.edit_style = Some(style);
                s.edit_equipment = Some(slot);
                s.gear_search.clear();
            }
        } else if let Some(slot) = action
            .strip_prefix("auto-gear:")
            .and_then(|v| v.parse::<usize>().ok())
        {
            let style = comparison_style(&s).to_string();
            let query = s.gear_search.to_lowercase();
            let top = ranked_gear(&s).0.into_iter().find(|row| {
                row.usable
                    && row.score.is_some()
                    && (query.is_empty()
                        || planner::item_name(row.gear.id)
                            .to_lowercase()
                            .contains(&query))
            });
            if let Some(row) = top {
                let key_style =
                    if s.saved.options.fewer_switches && [0, 1, 2, 9, 10, 12].contains(&slot) {
                        planner::primary_style(&s.saved.options)
                    } else {
                        &style
                    };
                let key = format!("{key_style}:{slot}");
                if let Some(preferences) = s.ranking_draft.take() {
                    s.saved.options.ranking = preferences;
                }
                s.saved.options.gear_overrides.insert(key, row.gear.id);
                s.edit_equipment = None;
                s.notice("Top result equipped using your ranking weights.", false);
            } else {
                s.notice(
                    "Set an available stat above 0 to choose a ranked result.",
                    true,
                );
            }
        } else if action == "rank-apply" {
            s.saved.options.recommended = Some(false);
            if let Some(preferences) = s.ranking_draft.take() {
                s.saved.options.ranking = preferences;
            }
            s.notice(
                "Gear rebuilt using your priorities. Manual equipment choices stay pinned.",
                false,
            );
        } else if action == "rank-reset" {
            if let Some(preferences) = s.ranking_draft.take() {
                s.saved.options.ranking = preferences;
            }
            s.saved.options.gear_overrides.clear();
            s.notice(
                "Manual gear choices cleared. Automatic gear uses your priorities.",
                false,
            );
        } else if let Some(preset) = action.strip_prefix("rank-preset:") {
            if let Some(weights) = crate::ranking::Weights::preset(preset) {
                s.saved.options.recommended = Some(false);
                s.saved.options.ranking.weights = weights;
                s.ranking_draft = None;
            }
        } else if action == "casual" {
            s.saved.options.max_switches = Some(4);
            s.saved.options.switch_slots = 8;
            s.saved.options.focus_style = "Balanced".into();
            s.saved.options.fewer_switches = false;
        } else if let Some(choice) = action.strip_prefix("pick-gear:") {
            let parts: Vec<_> = choice.split(':').collect();
            if parts.len() == 3 {
                if let (Ok(slot), Ok(id)) = (parts[1].parse::<usize>(), parts[2].parse::<i32>()) {
                    if slot < 14
                        && (id == 0
                            || (planner::gear(id)
                                .is_some_and(|g| g.slot == planner::SLOT_KEYS[slot])
                                && s.saved.snapshot.available(s.saved.options.shared).contains_key(&id)
                                && !s.saved.options.item_excluded(id)
                                && s.saved
                                    .snapshot
                                    .selected()
                                    .is_some_and(|m| planner::can_use(id, m))))
                    {
                        let mut selected_style = parts[0].to_string();
                        if slot == 3 {
                            s.saved.options.combat.loaded_ammo_confirmed = false;
                            if let Some(rule) = crate::rules::weapon(id) {
                                if s.saved.options.combat.loaded_dart.is_some_and(|dart| {
                                    !rule.family.internal_ammo.is_empty()
                                        && !rule.family.internal_ammo.contains(&dart)
                                }) {
                                    s.saved.options.combat.loaded_dart = None;
                                }
                            }
                        }
                        if slot == 3
                            && s.saved.options.boss == Boss::Custom
                            && s.saved.options.targets.len() == 1
                        {
                            if let Some(rule) = crate::rules::weapon(id) {
                                if !rule.permits(&selected_style) {
                                    selected_style = rule.family.style.clone();
                                    s.saved.options.targets[0].style = selected_style.clone();
                                    s.edit_style = Some(selected_style.clone());
                                    s.phase = None;
                                }
                                if rule.family.style == "Ranged"
                                    && s.saved.options.gear_overrides.get("Ranged:13").is_some_and(
                                        |ammo| !crate::rules::ammo_ids(id).contains(ammo),
                                    )
                                {
                                    s.saved.options.gear_overrides.remove("Ranged:13");
                                }
                            }
                        }
                        let key = format!(
                            "{}:{slot}",
                            if s.saved.options.fewer_switches
                                && [0, 1, 2, 9, 10, 12].contains(&slot)
                            {
                                planner::primary_style(&s.saved.options)
                            } else {
                                &selected_style
                            }
                        );
                        s.saved.options.gear_overrides.insert(key, id);
                        s.edit_equipment = None;
                        s.notice(
                            "Equipment updated. Inventory edits kept; review any packing conflicts below.",
                            false,
                        );
                    }
                }
            }
        } else if let Some(i) = action.strip_prefix("edit-inventory:") {
            if s.shared_setup.is_some() {
                s.notice(
                    "Rebuild using your items before editing this shared packing list.",
                    true,
                );
            } else {
                s.edit_inventory = i.parse::<usize>().ok().filter(|i| *i < 28);
                s.edit_equipment = None;
                s.inventory_query.clear();
                s.inventory_filter = "All".into();
            }
        } else if action == "close-inventory" {
            s.edit_inventory = None;
        } else if action == "reset-inventory" || action == "empty-inventory" {
            if let Some(index) = s.edit_inventory {
                if action == "empty-inventory" {
                    s.saved.options.inventory_overrides.insert(index, 0);
                } else {
                    s.saved.options.inventory_overrides.remove(&index);
                }
                s.saved.options.inventory_quantities.remove(&index);
                s.edit_inventory = None;
            }
        } else if let Some(id) = action
            .strip_prefix("pick-inventory:")
            .and_then(|v| v.parse::<i32>().ok())
        {
            if let Some(index) = s.edit_inventory {
                let free = inventory_available(&s, &s.plan(), index, id);
                if free > 0 {
                    let quantity = if planner::is_stackable(id) {
                        input("inventory-stack")
                            .parse::<u32>()
                            .unwrap_or(1)
                            .clamp(1, free.min(i32::MAX as u32))
                    } else {
                        1
                    };
                    s.saved.options.inventory_overrides.insert(index, id);
                    s.saved.options.inventory_quantities.insert(index, quantity);
                    s.edit_inventory = None;
                }
            }
        } else if action == "compare-spell-auto" {
            choose_plan_spell(&mut s, crate::rules::SpellSelection::Auto);
        } else if let Some(element) = action.strip_prefix("compare-element:") {
            if ["air", "water", "earth", "fire"].contains(&element) {
                choose_plan_spell(&mut s, crate::rules::SpellSelection::Element(element.into()));
            }
        } else if let Some(name) = action.strip_prefix("compare-spell:") {
            if crate::rules::spells()
                .iter()
                .any(|recipe| recipe.name == name)
            {
                choose_plan_spell(&mut s, crate::rules::SpellSelection::Spell(name.into()));
            }
        } else if let Some(kind) = action.strip_prefix("ranged-type:") {
            use crate::rules::RangedType;
            if let Some(selection) = [RangedType::Auto, RangedType::Light, RangedType::Standard, RangedType::Heavy]
                .into_iter().find(|t| t.key().unwrap_or("auto") == kind)
            {
                choose_plan_ranged(&mut s, selection);
            }
        } else if let Some(stance) = action.strip_prefix("ranged-stance:") {
            if ["Rapid", "Accurate"].contains(&stance) {
                s.saved.options.combat.ranged_stance = stance.into();
            }
        } else if let Some(value) = action.strip_prefix("protect:") {
            let minimum = match value {
                "None" => Some(0),
                "Magic" => Some(37),
                "Ranged" => Some(40),
                "Melee" => Some(43),
                _ => None,
            };
            if minimum.is_some_and(|minimum| {
                minimum == 0
                    || s.saved.options.combat.protection == value
                    || s.saved
                        .snapshot
                        .selected()
                        .and_then(|m| m.level("Prayer"))
                        .is_some_and(|level| level >= minimum)
            }) {
                s.saved.options.combat.protection = if s.saved.options.combat.protection == value {
                    "None".into()
                } else {
                    value.into()
                };
            }
        } else if let Some(name) = action.strip_prefix("stat-prayer:") {
            if s.saved.options.combat.offensive == name {
                s.saved.options.combat.offensive = "None".into();
            } else if crate::prayer::prayers()
                .iter()
                .find(|p| p.name == name)
                .is_some_and(|p| {
                    s.saved
                        .snapshot
                        .selected()
                        .is_some_and(|m| crate::prayer::eligible(m, p))
                })
            {
                s.saved.options.combat.offensive = name.into();
            }
        } else if action == "toggle-preserve" {
            if s.saved.options.combat.preserve
                || s.saved
                    .snapshot
                    .selected()
                    .and_then(|m| m.level("Prayer"))
                    .is_some_and(|level| level >= 55)
            {
                s.saved.options.combat.preserve = !s.saved.options.combat.preserve;
            }
        } else if let Some(attack) = action.strip_prefix("incoming-choice:") {
            if comparison_monster(&s).is_some_and(|target| {
                crate::defence::attacks(target)
                    .iter()
                    .any(|name| name == attack)
            }) {
                s.incoming_attack = Some(attack.into());
            }
        } else if let Some(id) = action
            .strip_prefix("compare-target:")
            .and_then(|v| v.parse::<i32>().ok())
        {
            if encounter_targets(&s).contains(&id) {
                s.compare_target = Some(id);
                s.incoming_attack = None;
                if s.saved.options.boss == Boss::Custom {
                    if let Some(target) = s.saved.options.targets.iter().find(|t| t.id == id) {
                        s.edit_style = Some(target.style.clone());
                    }
                }
            }
        } else if let Some(style) = action.strip_prefix("edit-style:") {
            if crate::rules::STYLES.contains(&style) {
                // Clicking the active Melee tab retains its selected attack type.
                if style != "Melee" || crate::rules::base_style(comparison_style(&s)) != "Melee" {
                    s.edit_style = Some(style.into());
                }
            }
        } else if let Some(i) = action.strip_prefix("phase:") {
            s.phase = i.parse().ok();
            let p = s.plan();
            if let Some(phase) = s.phase.and_then(|i| p.phases.get(i)) {
                s.edit_style = Some(
                    if phase.name == "Akkha" {
                        "Magic"
                    } else if phase.name == "Wardens" {
                        "Ranged"
                    } else {
                        phase.style.as_str()
                    }
                    .into(),
                );
                s.compare_target = match s.saved.options.boss {
                    Boss::Barrows => Some(if s.phase == Some(1) { 1672 } else { 1673 }),
                    Boss::Toa => match phase.name.as_str() {
                        "Ba-ba" => Some(11778),
                        "Kephri" => Some(11719),
                        "Zebak" => Some(11730),
                        "Akkha" => Some(11789),
                        _ => None,
                    },
                    Boss::Custom => s
                        .phase
                        .and_then(|i| s.saved.options.targets.get(i))
                        .map(|t| t.id),
                };
            }
        } else if action == "starting" {
            s.phase = None;
            s.edit_style = None;
            s.compare_target = None;
        } else if action == "demo" {
            if s.demo_return.is_none() && s.saved.snapshot.source != "Demo" {
                s.demo_return = Some(s.saved.clone());
            }
            s.saved.snapshot = account::demo();
            s.saved.death_risk = Default::default();
            s.saved.options = Options::default();
            s.saved.pins = BTreeMap::new();
            s.shared_setup = None;
            s.tab = "bossing".into();
            s.phase = None;
            s.edit_style = None;
            s.compare_target = None;
            s.message.clear();
        } else if action == "restore-account" {
            if let Some(saved) = s.demo_return.take() {
                s.saved = saved;
                s.shared_setup = None;
                s.phase = None;
                s.edit_style = None;
                s.compare_target = None;
                s.tab = "bossing".into();
            }
        } else if action == "add-target" {
            let raw = input("monster");
            let id = raw.rsplit(" · ").next().and_then(|v| v.parse().ok());
            let style = document()
                .get_element_by_id("target-style")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|e| e.value())
                .unwrap_or_default();
            if let Some(id) = id {
                let t = planner::Target { id, style };
                if t.valid() && s.saved.options.targets.len() < crate::rules::MAX_TARGETS {
                    s.saved.options.boss = Boss::Custom;
                    s.building_group = true;
                    s.saved.options.targets.push(t);
                    s.shared_setup = None;
                    s.phase = None;
                } else {
                    s.notice(
                        "Choose a valid monster; at most 64 attack windows per group.",
                        true,
                    );
                }
            } else {
                s.notice("Choose a monster from the search list.", true);
            }
        } else if let Some(i) = action
            .strip_prefix("remove-target:")
            .and_then(|v| v.parse::<usize>().ok())
        {
            if i < s.saved.options.targets.len() {
                s.saved.options.targets.remove(i);
                s.shared_setup = None;
                s.phase = None;
            }
        } else if let Some(id) = action.strip_prefix("pin:") {
            let selected = goal_key(&s.saved.snapshot.selected, &s.saved.options);
            let pins = s.saved.pins.entry(selected).or_default();
            if pins.iter().any(|v| v == id) {
                pins.retain(|v| v != id);
            } else if pins.len() < 1 {
                pins.push(id.into());
            } else {
                s.notice(
                    "One upgrade is tracked per boss. Unpin it before choosing another.",
                    true,
                );
            }
        } else if let Some(id) = action
            .strip_prefix("exclude:")
            .and_then(|s| s.parse::<i32>().ok())
        {
            if !s.saved.options.excluded.remove(&id) {
                s.saved.options.excluded.insert(id);
            }
            s.notice(
                "Gear preference updated. The packing list has been rebuilt.",
                false,
            );
        }
        s.persist();
    });
    render();
    if action == "efficient-compare" {
        if let Some(panel) = document().get_element_by_id("budget-comparison") {
            panel.scroll_into_view();
        }
    }
    if action.starts_with("tab:")
        || action.starts_with("choose-encounter:")
        || ["demo", "restore-account"].contains(&action)
    {
        window().scroll_to_with_x_and_y(0.0, 0.0);
    }
}

async fn lookup_skills(name: String) {
    let previous = STATE.with(|s| s.borrow().saved.snapshot.fetched_at.clone());
    let path = js_sys::encode_uri_component(&name)
        .as_string()
        .unwrap_or_default();
    let result = async {
        let response = JsFuture::from(
            window().fetch_with_str(&format!("https://api.wiseoldman.net/v2/players/{path}")),
        )
        .await
        .map_err(|_| "Skill lookup unavailable. Enter your levels manually.")?
        .dyn_into::<web_sys::Response>()
        .map_err(|_| "Unexpected response.")?;
        if !response.ok() {
            return Err("No public snapshot found. Enter levels manually.");
        }
        JsFuture::from(response.text().map_err(|_| "Could not read skills.")?)
            .await
            .map_err(|_| "Could not read skills.")?
            .as_string()
            .ok_or("Invalid skills response.")
    }
    .await;
    STATE.with(|state|{let mut s=state.borrow_mut();s.busy=false;if s.saved.snapshot.fetched_at!=previous{s.notice("Account changed during lookup; result discarded.",true);return;}
        match result {Ok(raw)=>{
            let mut m=if s.saved.snapshot.source=="Demo"{account::Member::default()}else{s.saved.snapshot.selected().cloned().unwrap_or_default()};
            match m.apply_hiscores(&raw){Ok(())=>{m.name=name.clone();s.saved.snapshot.members=vec![m];s.saved.snapshot.selected=name;s.saved.snapshot.source="RuneLite bank / public levels".into();s.saved.options.access_override=None;s.saved.options.started_barrows=None;s.persist();s.notice("Public levels saved locally. Check the snapshot date before relying on cached levels.",false);},Err(e)=>s.notice(e,true)}
        },Err(e)=>s.notice(e,true)}
    });
    render();
}

fn import_raw(raw: &str) {
    let now = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default();
    if raw.trim_start_matches('\u{feff}').starts_with("Item id\t") {
        let name = input("player-name");
        match Snapshot::from_bank_tsv(raw, &name, now) {
            Ok(mut snapshot) => {
                STATE.with(|state| {
                    let mut s = state.borrow_mut();
                    if s.saved.snapshot.source != "Demo" {
                        if let Some(old) = s.saved.snapshot.selected() {
                            snapshot.members[0].skills = old.skills.clone();
                            snapshot.members[0].level_overrides = old.level_overrides.clone();
                            snapshot.members[0].last_updated = old.last_updated.clone();
                            snapshot.members[0].name = old.name.clone();
                            snapshot.selected = old.name.clone();
                        }
                    }
                    s.demo_return = None;
                    s.saved.snapshot = snapshot;
                    s.saved.death_risk = Default::default();
                    s.unreadable_save = None;
                    s.shared_setup = None;
                    s.saved.options.access_override = None;
                    s.saved.options.started_barrows = None;
                    s.saved.options.gear_overrides.clear();
                    let ready = s.saved.snapshot.selected().is_some_and(|m| {
                        [
                            "Attack",
                            "Strength",
                            "Defence",
                            "Ranged",
                            "Magic",
                            "Prayer",
                            "Hitpoints",
                        ]
                        .iter()
                        .all(|skill| m.level(skill).is_some())
                    });
                    s.tab = if ready { "bossing" } else { "account" }.into();
                    s.phase = None;
                    s.persist();
                    s.notice(
                        if ready {
                            "Bank updated on this device."
                        } else {
                            "Bank imported. Add your combat levels below to build a setup."
                        },
                        false,
                    );
                });
                render();
            }
            Err(e) => notice(&e, true),
        }
        return;
    }
    if let Ok(mut saved) = Saved::parse(raw) {
        if saved.version != 1 || saved.snapshot.selected().is_none() {
            notice("This backup version is not supported.", true);
            return;
        }
        if Snapshot::parse(
            &serde_json::to_string(&saved.snapshot).unwrap_or_default(),
            now.clone(),
        )
        .is_err()
        {
            notice(
                "This backup contains invalid account data; the current account is unchanged.",
                true,
            );
            return;
        }
        saved.options.food = saved.options.food.min(20);
        saved.options.prayer_potions = saved.options.prayer_potions.min(12);
        saved.options.raid_level = saved.options.raid_level.min(150);
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.demo_return = None;
            s.saved = saved;
            s.unreadable_save = None;
            s.shared_setup = None;
            s.edit_style = None;
            s.compare_target = None;
            s.edit_equipment = None;
            s.edit_inventory = None;
            s.tab = "bossing".into();
            s.phase = None;
            s.persist();
            s.notice("Account and goals restored from your backup.", false);
        });
        render();
        return;
    }
    match Snapshot::parse(raw, now) {
        Ok(snapshot) => STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.demo_return = None;
            s.saved.snapshot = snapshot;
            s.saved.death_risk = Default::default();
            s.unreadable_save = None;
            s.saved.options = Options::default();
            s.shared_setup = None;
            s.edit_style = None;
            s.compare_target = None;
            s.edit_equipment = None;
            s.edit_inventory = None;
            s.phase = None;
            s.tab = "bossing".into();
            s.persist();
            s.notice(
                "Account imported. Review your levels and packing list.",
                false,
            );
        }),
        Err(e) => {
            notice(&e, true);
            return;
        }
    }
    render();
}
fn download(name: &str, text: &str) {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(text));
    let props = web_sys::BlobPropertyBag::new();
    props.set_type("application/json");
    let result = (|| -> Result<(), JsValue> {
        let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &props)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob)?;
        let a = document()
            .create_element("a")?
            .dyn_into::<web_sys::HtmlAnchorElement>()?;
        a.set_href(&url);
        a.set_download(name);
        a.click();
        web_sys::Url::revoke_object_url(&url)?;
        Ok(())
    })();
    if result.is_err() {
        notice(
            "Download unavailable. Use the copyable export instead.",
            true,
        );
    }
}

/// Refresh siblings around the active form control without removing it or its
/// ancestors. Recreating a number input loses its caret and interrupts native
/// spinner/keyboard repetition, even if focus is restored afterwards.
fn patch_around_edit(
    old: &web_sys::Node,
    fresh: &web_sys::Node,
    edit: &web_sys::Node,
    fresh_edit: &web_sys::Node,
) -> Result<(), JsValue> {
    if old.is_same_node(Some(edit)) {
        return Ok(());
    }
    let children = |node: &web_sys::Node| {
        let list = node.child_nodes();
        (0..list.length())
            .filter_map(|i| list.item(i))
            .collect::<Vec<_>>()
    };
    let old_children = children(old);
    let fresh_children = children(fresh);
    let kept = old_children
        .iter()
        .find(|node| node.contains(Some(edit)))
        .ok_or_else(|| JsValue::from_str("Active field path missing"))?;
    let fresh_index = fresh_children
        .iter()
        .position(|node| node.contains(Some(fresh_edit)))
        .ok_or_else(|| JsValue::from_str("Updated field path missing"))?;
    for node in &old_children {
        if !node.is_same_node(Some(kept)) {
            old.remove_child(node)?;
        }
    }
    for (index, node) in fresh_children.iter().enumerate() {
        if index < fresh_index {
            old.insert_before(&node.clone_node_with_deep(true)?, Some(kept))?;
        } else if index > fresh_index {
            old.append_child(&node.clone_node_with_deep(true)?)?;
        }
    }
    patch_around_edit(kept, &fresh_children[fresh_index], edit, fresh_edit)
}

fn update_markup(root: &Element, html: &str, keep: Option<&str>) {
    let retained = (|| -> Result<bool, JsValue> {
        let Some(id) = keep else { return Ok(false) };
        let Some(edit) = document()
            .get_element_by_id(id)
            .filter(|e| root.contains(Some(e)))
        else {
            return Ok(false);
        };
        let fresh = document().create_element("div")?;
        fresh.set_inner_html(html);
        let Some(fresh_edit) = fresh.query_selector(&format!("[id=\"{id}\"]"))? else {
            return Ok(false);
        };
        patch_around_edit(root, &fresh, &edit, &fresh_edit)?;
        Ok(true)
    })()
    .unwrap_or(false);
    if !retained {
        root.set_inner_html(html);
    }
}

fn render() {
    hide_item_tooltip();
    // A full render already includes the latest inputs; cancel older delayed renders.
    LIVE_EDIT.with(|generation| generation.set(generation.get().wrapping_add(1)));
    render_with_edit(None);
}

fn render_with_edit(edit: Option<&str>) {
    STATE.with(|state|{let s=state.borrow();let snapshot=&s.saved.snapshot;let options=&s.saved.options;let plan=s.plan();
    let demo=snapshot.source=="Demo";
    let nav=|(key,label,icon):(&str,&str,i32)|format!("<button class=\"nav-item {}\" data-action=\"tab:{key}\">{}<span>{label}</span>{}</button>",if s.tab==key{"active"}else{""},image(icon),if key=="goals"{format!("<span class=\"nav-count\">{}</span>",s.pins().len())}else{String::new()});
    let mut html=format!(r#"<div class="app-shell"><header class="sidebar"><button class="brand" data-action="tab:bossing" aria-label="Ironbound home"><span class="brand-mark">{}</span><span>Ironbound<small>IRONMAN BOSSING</small></span></button><nav aria-label="Main navigation">{}{}</nav></header><main class="main"><header class="topbar"><span class="local-status">Saved on this device</span><button class="account-pill text-button" data-action="tab:account" aria-label="Manage your account">{}<span>{}<small>{}</small></span></button></header>"#,image(2347),nav(("bossing","Bossing",4151)),nav(("account","Your bank",1712)),image(10828),esc(&snapshot.selected),if demo{"Example account"}else{"Local account"});
    html = html.replace("Saved on this device", if s.demo_return.is_some() { "Example preview · account unchanged" } else { s.save_status.get().label() });
    if demo{html.push_str(&format!("<div class=\"demo-banner\"><p>{}</p>{}</div>","Example bank",if s.demo_return.is_some(){button("restore-account","Return to your account","text-button")}else{button("tab:account","Import your bank","button gold")}));}
    html.push_str(&save_banner(&s));
    if !s.message.is_empty(){html.push_str(&format!("<div role=\"status\" class=\"notice {}\">{}</div>",if s.error{"error"}else{""},esc(&s.message)));}
    html.push_str("<div class=\"page-content\">");
    if s.tab=="account"{html.push_str(&account_page(&s));}
    else if s.shared_setup.is_none()&&!planner::missing_combat_levels(snapshot).is_empty(){html.push_str(&format!("<section class=\"page-heading level-gate\"><h1>Add your combat levels</h1><p>Your bank is loaded. Combat levels are needed before a setup can be recommended.</p>{}</section>",button("tab:account","Add levels or look up your character","button gold")));}
    else{
        html.push_str("<div class=\"encounter-header\">");
        html.push_str(&encounter_detail(&s,&plan));
        html.push_str("<div class=\"encounter-actions\">");
        html.push_str(&format!("<details class=\"encounter-picker\" id=\"boss-bestiary\" {}><summary aria-label=\"Change encounter\">Change<span class=\"wide-label\"> encounter</span></summary>",if s.bestiary_open||s.building_group||(options.boss==Boss::Custom&&options.targets.is_empty()){"open"}else{""}));
        html.push_str(&encounter_catalog(&s));
        if s.bestiary_open||s.building_group||(options.boss==Boss::Custom&&options.targets.is_empty()) {
        html.push_str(&format!("<details class=\"custom-builder\" {}><summary>Build a custom group</summary>",if s.building_group{"open"}else{""}));
        html.push_str(&custom_group(&s));
        html.push_str(&button("custom-done","Done","button ghost"));
        html.push_str("</details>");
        }
        html.push_str("</details>");
        html.push_str(&raid_rooms(&s));
        html.push_str("</div></div>");
        html.push_str(&wilderness_banner(&s));
        html.push_str(&death_risk_panel(&s, &plan));
        if s.shared_setup.is_some(){html.push_str(&format!("<p class=\"notice\">Viewing a shared setup. Your saved bank is unchanged. {}</p>",button("use-bank","Rebuild using my items","text-button")));}
        if s.shared_setup.is_none(){html.push_str(&switch_controls(&s,&plan));}
        html.push_str("<nav class=\"planner-tabs\" aria-label=\"Planner sections\">");
        for (view,label) in [("loadout","Gear"),("combat","Combat"),("prepare","Preparation")] {let disabled=if view=="combat"&&s.shared_setup.is_some(){"disabled title=\"Rebuild using your items to calculate combat stats\""}else{""};html.push_str(&format!("<button class=\"workspace-tab {}\" data-action=\"view:{view}\" aria-pressed=\"{}\" {disabled}>{label}{}</button>",if s.view==view{"active"}else{""},s.view==view,if view=="prepare"&&!plan.missing.is_empty(){format!(" <span class=\"issue-count\">{}</span>",plan.missing.len())}else{String::new()}));}
        html.push_str("</nav><section class=\"workspace-view\">");
        match s.view.as_str(){
            "combat"=>{html.push_str("<div class=\"analysis-layout\">");html.push_str(&combat_panel(&s,&plan));html.push_str(&simulation_panel(&s,&plan));html.push_str(&switch_panel(&s,&plan));html.push_str("</div>");},
            "prepare"=>{html.push_str("<div class=\"prepare-layout\">");html.push_str(&checks_panel(&s,&plan));if s.shared_setup.is_none(){html.push_str(&manual_checks(&s));html.push_str(&coverage_panel(&s.saved.options));}if !plan.missing.is_empty(){html.push_str("<section class=\"panel\"><div class=\"panel-heading\"><h2>Missing trip items</h2></div><ul class=\"missing-list\">");for missing in &plan.missing{html.push_str(&format!("<li>{}</li>",esc(missing)));}html.push_str("</ul></section>");}if s.shared_setup.is_none(){html.push_str(&next_goal(&s));}html.push_str("</div>");},
            _=>{html.push_str(&phase_navigation(&s,&plan));html.push_str("<div class=\"loadout-layout\">");html.push_str(&loadout_panel(&s,&plan));html.push_str(&inventory_panel(&s,&plan));html.push_str("</div>");}
        }
        html.push_str("</section><div class=\"plan-actions\">");html.push_str(&export_panel(&s,&plan));html.push_str("</div>");
        if s.shared_setup.is_none(){html.push_str("<div class=\"plan-tools\">");html.push_str(&efficient_panel(&s,&plan));html.push_str(&budget_comparison(&s));html.push_str(&named_setups(&s));html.push_str("</div>");}
        if s.view=="loadout" && s.shared_setup.is_none(){html.push_str("<details class=\"secondary-section\"><summary>All owned gear</summary>");html.push_str(&tier_table(&s));html.push_str("</details>");}
    }
    html.push_str("</div><footer><span>Ironbound — an unofficial OSRS fan site.</span><a href=\"https://oldschool.runescape.wiki/\" target=\"_blank\" rel=\"noreferrer\">OSRS Wiki ↗</a><a href=\"./assets/osrs/NOTICE.txt\" target=\"_blank\" rel=\"noreferrer\">Asset credits</a><span>Starter catalog · 9 Nov 2026</span></footer></main></div>");
    html.push_str("<div id=\"item-explanation\" class=\"item-explanation-tooltip\" role=\"dialog\" aria-label=\"Equipment comparison\" hidden></div>");
    if s.edit_equipment.is_some(){html.push_str(&gear_picker(&s));}
    if let Some(index)=s.edit_inventory{html.push_str(&inventory_editor(&s,index));}
    if let Some(el)=document().get_element_by_id("app"){update_markup(&el,&html,edit);}
    if s.edit_equipment.is_some(){if let Ok(Some(main))=document().query_selector("main"){let _=main.set_attribute("inert","");}if let Some(search)=document().get_element_by_id("gear-search").and_then(|e|e.dyn_into::<web_sys::HtmlElement>().ok()){let _=search.focus();}}
    if s.edit_inventory.is_some(){if let Ok(Some(main))=document().query_selector("main"){let _=main.set_attribute("inert","");}if let Some(search)=document().get_element_by_id("inventory-search").and_then(|e|e.dyn_into::<web_sys::HtmlElement>().ok()){let _=search.focus();}}
});
}

fn encounter_detail(s: &State, p: &Plan) -> String {
    let boss = s.saved.options.boss;
    let guide = selected_encounter(&s.saved.options)
        .map(|entry| entry.guide_url())
        .unwrap_or_else(|| boss.source().into());
    let mut html = format!(
        "<section class=\"encounter-detail planner-summary\"><h1>{}</h1><div class=\"detail-facts\"><span>Gear · {} bag slot{}</span><a href=\"{}\" target=\"_blank\" rel=\"noreferrer\">Wiki guide ↗</a>",
        encounter_label(&s.saved.options),
        p.switches,
        if p.switches == 1 { "" } else { "s" },
        esc(&guide)
    );
    if boss == Boss::Toa {
        html.push_str(&format!("<label class=\"raid-control\">Raid level <input id=\"raid-level\" type=\"number\" min=\"0\" max=\"150\" value=\"{}\" title=\"Solo, path level 0. Supports levels 0–150; invocation mechanics remain partial.\"></label>",s.saved.options.raid_level));
    }
    html.push_str("</div>");
    if s.shared_setup.is_some() {
        html.push_str("<p class=\"plan-coverage\">Shared packing list · levels not included</p>");
    } else {
        let note = if boss == Boss::Toa
            || selected_encounter(&s.saved.options)
                .is_some_and(|e| e.coverage().contains("DPS partial"))
        {
            "Partial DPS · "
        } else if boss == Boss::Custom && selected_encounter(&s.saved.options).is_none() {
            "Mechanics unverified · "
        } else {
            ""
        };
        let rows = crate::coverage::windows(&s.saved.options);
        html.push_str(&format!("<details class=\"coverage-details\"><summary>{note}Coverage · {}/{} attack windows</summary>{}</details>", rows.iter().filter(|r|r.supported).count(),rows.len(),coverage_panel(&s.saved.options)));
    }
    html.push_str("</section>");
    html
}

fn raid_rooms(s: &State) -> String {
    if s.saved.options.boss != Boss::Custom {
        return String::new();
    }
    let Some(raid) = crate::raids::for_targets(&s.saved.options.targets) else {
        return String::new();
    };
    let count = raid
        .rooms
        .iter()
        .filter(|room| room.selected(&s.saved.options.targets))
        .count();
    let mut html = format!(
        "<details id=\"raid-room-picker\" class=\"raid-route\" {}><summary>Raid bosses <span>{count} selected</span></summary><div class=\"raid-route-groups\">",
        if s.raid_rooms_open { "open" } else { "" }
    );
    let mut stages = Vec::<&str>::new();
    for room in &raid.rooms {
        if !stages.contains(&room.stage.as_str()) {
            stages.push(&room.stage);
        }
    }
    for stage in stages {
        html.push_str(&format!(
            "<section class=\"raid-stage\"><h3>{}</h3>",
            esc(stage)
        ));
        let rooms = raid
            .rooms
            .iter()
            .filter(|room| room.stage == stage)
            .collect::<Vec<_>>();
        if rooms.iter().any(|room| room.choice_group.is_some()) {
            html.push_str("<small>Choose one</small>");
        }
        for room in rooms {
            let selected = room.selected(&s.saved.options.targets);
            html.push_str(&format!("<button class=\"raid-room {}\" data-action=\"raid-room:{}:{}\" aria-pressed=\"{selected}\" {}><strong>{}</strong>{}</button>",if selected{"active"}else{""},esc(&raid.id),esc(&room.id),if room.required||s.shared_setup.is_some(){"disabled"}else{""},esc(&room.name),if room.required{"<small>Required</small>"}else{""}));
        }
        html.push_str("</section>");
    }
    html.push_str("</div></details>");
    html
}
fn slot(id: Option<i32>, label: &str, class: &str, quantity: Option<u32>) -> String {
    let title = id
        .map(|id| {
            format!(
                "{}{} · {}",
                planner::item_name(id),
                quantity.map(|q| format!(" × {q}")).unwrap_or_default(),
                label
            )
        })
        .unwrap_or_else(|| format!("{label} · empty"));
    format!(
        "<div class=\"item-slot {class} {}\" aria-label=\"{}\">{}{}<span class=\"slot-caption\">{}</span></div>",
        if id.is_some() { "filled" } else { "empty" },
        esc(&title),
        id.map(image)
            .unwrap_or_else(|| "<span class=\"empty-symbol\">·</span>".into()),
        quantity
            .filter(|q| *q > 1)
            .map(|q| format!("<span class=\"quantity\">{q}</span>"))
            .unwrap_or_default(),
        esc(label)
    )
}
fn phase_navigation(s: &State, p: &Plan) -> String {
    if p.phases.is_empty() {
        return String::new();
    }
    let mut html = String::from(
        "<nav class=\"panel loadout-preview\" aria-label=\"Loadout preview\"><div class=\"phase-tabs\">",
    );
    if p.phases.len() > 6 {
        html.push_str("<label class=\"phase-select-label\">Preview<select id=\"phase-select\"><option value=\"start\">Start gear</option>");
        for (i, phase) in p.phases.iter().enumerate() {
            html.push_str(&format!(
                "<option value=\"{i}\" {}>{}</option>",
                if s.phase == Some(i) { "selected" } else { "" },
                esc(&phase.name)
            ));
        }
        html.push_str("</select></label>");
    } else if !p.phases.is_empty() {
        html.push_str(&format!("<button class=\"phase {}\" data-action=\"starting\" aria-pressed=\"{}\">Start gear</button>",if s.phase.is_none(){"active"}else{""},s.phase.is_none()));
        for (i, phase) in p.phases.iter().enumerate() {
            let label = if s.saved.options.boss == Boss::Barrows {
                if i == 0 { "Five brothers" } else { "Ahrim" }
            } else {
                s.saved
                    .options
                    .targets
                    .get(i)
                    .and_then(|target| crate::encounters::window(target.id))
                    .filter(|window| !window.label.is_empty())
                    .map(|window| window.label.as_str())
                    .unwrap_or(&phase.name)
            };
            html.push_str(&format!("<button class=\"phase {}\" data-action=\"phase:{i}\" aria-pressed=\"{}\">{}</button>",if s.phase==Some(i){"active"}else{""},s.phase==Some(i),esc(label)));
        }
    }
    html.push_str("</div></nav>");
    html
}
fn loadout_panel(s: &State, p: &Plan) -> String {
    let current = s.phase.and_then(|i| p.phases.get(i));
    let set = current.map(|p| &p.equipment).unwrap_or(&p.equipment);
    let mut html = format!(
        "<section class=\"panel loadout-panel\"><div class=\"panel-heading\"><h2>{} {}</h2>{}</div><div class=\"loadout-content\"><div class=\"equipment-paper\"><div class=\"paper-header\">{}</div><div class=\"equipment-grid\">",
        image(12954),
        "Equipment",
        badge(
            if s.shared_setup.is_some() {
                "SHARED SETUP"
            } else {
                "FROM YOUR BANK"
            },
            "neutral"
        ),
        current.map(|p| esc(&p.name)).unwrap_or("Equipment".into())
    );
    for (slot_index, class) in [
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
    ] {
        let tooltip = format!(
            "{}: {}",
            planner::SLOTS[slot_index],
            set[slot_index]
                .map(planner::item_name)
                .unwrap_or_else(|| "Empty".into())
        );
        let reason = crate::explanations::equipment(
            &s.saved.snapshot,
            &s.saved.options,
            p,
            s.phase,
            slot_index,
        );
        let explanation = format!("{tooltip}\n{reason}");
        html.push_str(&format!(r#"<button class="equipment-edit {class}" data-action="edit-equipment:{slot_index}" data-item-id="{}" data-explanation="{}" aria-describedby="item-explanation" aria-controls="item-explanation" aria-haspopup="dialog" aria-expanded="false" aria-label="Change {}">{}</button>"#,set[slot_index].map(|id| id.to_string()).unwrap_or_default(),esc(&explanation),esc(&tooltip),slot(set[slot_index],planner::SLOTS[slot_index],"",None)));
    }
    html.push_str(
        "</div></div><details class=\"loadout-notes\"><summary>Spell & exclusions</summary>",
    );
    if let Some((_, effect)) = set[3].and_then(|id| {
        reference::KERIS_VARIANTS
            .iter()
            .find(|(variant, _)| *variant == id)
    }) {
        html.push_str(&format!(
            "<p class=\"weapon-effect\"><strong>{}</strong><br>{}</p>",
            esc(&set[3].map(planner::item_name).unwrap_or_default()),
            esc(effect)
        ));
    }
    if !p.spell.is_empty() {
        html.push_str(&format!(
            "<div class=\"spell-line\">{}<span>Trip spells<strong>{}</strong></span></div>",
            image(556),
            esc(&p.spell)
        ));
    }
    html.push_str("<details><summary>Exclude items</summary><div class=\"gear-list\">");
    for id in set.iter().flatten() {
        html.push_str(&format!(
            "<div><span>{}</span>{}</div>",
            esc(&planner::item_name(*id)),
            button(&format!("exclude:{id}"), "Exclude", "text-button")
        ));
    }
    for id in &s.saved.options.excluded {
        html.push_str(&format!(
            "<div><span class=\"muted\">{} (excluded)</span>{}</div>",
            esc(&planner::item_name(*id)),
            button(&format!("exclude:{id}"), "Restore", "text-button")
        ));
    }
    html.push_str("</div></details>");
    html.push_str("</details></div>");
    if let Some(phase) = current {
        html.push_str(&format!(
            "<p class=\"phase-note\"><strong>{}</strong> · {}</p>",
            esc(&phase.style),
            esc(&phase.note)
        ));
    }
    html.push_str("</section>");
    html
}
fn inventory_panel(s: &State, p: &Plan) -> String {
    let phase = s.phase.and_then(|index| p.phases.get(index));
    let inventory = p.preview_inventory(s.phase);
    let occupied = inventory
        .iter()
        .filter(|item| item.id > 0 && item.quantity > 0)
        .count();
    let mut html = format!(
        "<section class=\"panel packing-panel\"><div class=\"panel-heading\"><h2>{} Inventory</h2><span class=\"slot-count {}\">{} / 28 slots</span></div><div class=\"packing-body\"><div class=\"inventory-grid\">",
        image(385),
        if inventory.len() > 28 { "over" } else { "" },
        occupied
    );
    for i in 0..28 {
        let item = inventory.get(i).filter(|p| p.id > 0 && p.quantity > 0);
        let explanation = item
            .map(|item| {
                format!(
                    "{} × {}\n{}",
                    planner::item_name(item.id),
                    item.quantity,
                    crate::explanations::packed(&s.saved.snapshot, &s.saved.options, p, item)
                )
            })
            .unwrap_or_else(|| format!("Inventory slot {}: Empty\nChoose an owned item.", i + 1));
        html.push_str(&format!("<button class=\"inventory-edit\" data-action=\"edit-inventory:{i}\" data-item-id=\"{}\" data-explanation=\"{}\" aria-describedby=\"item-explanation\" aria-controls=\"item-explanation\" aria-haspopup=\"dialog\" aria-expanded=\"false\" aria-label=\"Inventory slot {}\" {}>",item.map(|p| p.id.to_string()).unwrap_or_default(),esc(&explanation),i+1,if phase.is_some() { "disabled" } else { "" }));
        html.push_str(&slot(
            item.map(|p| p.id),
            &format!(
                "Slot {}{}",
                i + 1,
                item.and_then(|p| s.saved.snapshot.selected()?.item_names.get(&p.id))
                    .map(|n| format!(" · {n}"))
                    .unwrap_or_default()
            ),
            "inventory-item",
            item.map(|p| p.quantity),
        ));
        html.push_str("</button>");
    }
    html.push_str("</div>");
    if let Some(phase) = phase {
        html.push_str(&format!("<p class=\"phase-note\">{} inventory preview. {} to edit the packing list. Exports use Start gear.</p>",esc(&phase.name),button("phase:start","Back to Start","text-button")));
    }
    html.push_str(&format!("<details class=\"supply-settings\"><summary>Adjust supplies</summary><label>Food<input id=\"food\" type=\"number\" min=\"0\" max=\"20\" value=\"{}\"></label><label>Prayer / restores<input id=\"potions\" type=\"number\" min=\"0\" max=\"12\" value=\"{}\"></label><label>Spell casts for trip<input id=\"spell-casts\" type=\"number\" min=\"1\" max=\"10000\" value=\"{}\"></label><label>Ammunition per weapon<input id=\"ammo-shots\" type=\"number\" min=\"1\" max=\"10000\" value=\"{}\"></label><p class=\"small-note\">Rune stacks cover the worst-case mix of the total attack casts. The ammunition budget applies to each arrow/bolt stack or loaded blowpipe; recovered projectiles may last longer. Check actual consumption and charges; these counts do not predict trip length.</p><div class=\"small-note\">{} slots free</div></details></div>",s.saved.options.food,s.saved.options.prayer_potions,s.saved.options.combat.spell_casts,s.saved.options.combat.ammo_shots,28usize.saturating_sub(occupied)));
    if s.shared_setup.is_none()
        && (!s.saved.options.inventory_overrides.is_empty()
            || !s.saved.options.inventory_quantities.is_empty())
    {
        html.push_str(&format!("<p class=\"small-note\">Manual inventory edits are kept when gear changes. Resolve conflicts below or {}.</p>", button("reset-all-inventory", "Reset inventory edits", "text-button")));
    }
    if !p.missing.is_empty() {
        html.push_str(&format!(
            "<details class=\"missing-list\"><summary>{} trip item{} to review</summary><ul>",
            p.missing.len(),
            if p.missing.len() == 1 { "" } else { "s" }
        ));
        for missing in &p.missing {
            html.push_str(&format!("<li>{}</li>", esc(missing)));
        }
        html.push_str("</ul></details>");
    }
    html.push_str("<details class=\"assumptions\"><summary>Trip notes</summary><ul>");
    for w in &p.warnings {
        html.push_str(&format!("<li>{}</li>", esc(w)));
    }
    html.push_str(&format!("<li>{} unique item IDs observed; {} match the reviewed gear priorities. Other items remain in your snapshot.</li></ul></details></section>",p.owned,p.supported));
    html
}
fn checks_panel(s: &State, p: &Plan) -> String {
    let status = if s.shared_setup.is_some() {
        p.status()
    } else {
        crate::preparation::summary(&s.saved.snapshot, &s.saved.options, p)
    };
    let mut html = format!(
        "<section class=\"panel readiness\"><div class=\"panel-heading\"><h2>Preparation check</h2><span class=\"mini-star\">✦</span></div><div class=\"readiness-status\">{}</div><div class=\"check-list\">",
        badge(
            status,
            if status.starts_with("Manual checks confirmed") {
                "green"
            } else {
                "amber"
            }
        )
    );
    for c in &p.checks {
        html.push_str(&format!("<div class=\"check\"><span class=\"check-mark {}\">{}</span><div><strong>{}</strong><small>{}</small></div></div>",match c.state{Some(true)=>"yes",Some(false)=>"no",None=>"unknown"},match c.state{Some(true)=>"✓",Some(false)=>"!",None=>"?"},esc(&c.name),esc(&c.detail)));
    }
    html.push_str(&format!("</div><p class=\"panel-footnote\">Preparation guidelines don't predict a successful kill. {} to review levels.</p></section>",button("tab:account","Open your account","text-button")));
    html
}

fn save_banner(s: &State) -> String {
    let status = s.save_status.get();
    if s.demo_return.is_some() {
        return String::new();
    }
    if matches!(status, SaveStatus::Saved | SaveStatus::Pending) {
        return String::new();
    }
    format!(
        "<div class=\"notice error save-warning\" role=\"alert\"><strong>{}.</strong> {} {} {}</div>",
        status.label(),
        if status == SaveStatus::Unreadable {
            "The original saved data is preserved. Download it before importing a replacement backup."
        } else {
            "Download a private backup before leaving this page."
        },
        button(
            if status == SaveStatus::Unreadable {
                "backup-unreadable"
            } else {
                "backup"
            },
            "Download private backup",
            "button ghost"
        ),
        button(
            "retry-save",
            if status == SaveStatus::Unreadable {
                "Reload saved account"
            } else {
                "Retry saving"
            },
            "button ghost"
        )
    )
}

fn named_setups(s: &State) -> String {
    let mut html = format!(
        "<details class=\"secondary-section setup-library\" {}><summary>Saved setups ({})</summary><label>Setup name<input id=\"setup-name\" maxlength=\"64\" placeholder=\"e.g. Yama learning\"></label>{}<p class=\"small-note\">Configurations stay on this device and in private backups. Loading rechecks your current bank. Enter a name above to rename a row.</p><div class=\"saved-setup-list\">",
        if s.setups_open { "open" } else { "" },
        s.saved.setups.0.len(),
        button("save-named", "Save current setup", "button gold")
    );
    for (i, entry) in s.saved.setups.0.iter().enumerate() {
        html.push_str(&format!(
            "<div class=\"saved-setup\"><strong>{}</strong><div>",
            esc(&entry.name)
        ));
        for (verb, label) in [
            ("load", "Load"),
            ("rename", "Rename"),
            ("replace", "Replace"),
            ("remove", "Remove"),
        ] {
            html.push_str(&button(&format!("named:{verb}:{i}"), label, "button ghost"));
        }
        html.push_str("</div></div>");
    }
    html.push_str("</div></details>");
    html
}

fn manual_checks(s: &State) -> String {
    let mut html = String::from(
        "<section class=\"panel manual-checks\"><h2>Check before leaving</h2><p>Bank imports cannot verify these. Confirm them yourself for this setup; changing the account or plan requires checking again.</p>",
    );
    let token = crate::preparation::token(&s.saved.snapshot, &s.saved.options);
    for (id, label) in crate::preparation::CONFIRMATIONS {
        let checked = s.saved.options.confirmations.get(id) == Some(&token);
        html.push_str(&format!("<button class=\"manual-confirm\" data-action=\"confirm:{id}\" aria-pressed=\"{checked}\">{} {label}</button>", if checked { "✓" } else { "□" }));
    }
    html.push_str("<p class=\"small-note\">Your confirmations are private and do not prove combat mechanics are supported or predict a successful kill.</p></section>");
    html
}

fn coverage_panel(options: &Options) -> String {
    let mut html = String::from(
        "<section class=\"panel coverage-panel\"><h2>Selected attack windows</h2><div class=\"table-wrap\"><table><thead><tr><th>Target / style</th><th>Coverage</th></tr></thead><tbody>",
    );
    for row in crate::coverage::windows(options) {
        html.push_str(&format!(
            "<tr><th>{}<small>{}</small></th><td><strong>{}</strong><small>{}</small></td></tr>",
            esc(&row.name),
            esc(&row.style),
            if row.supported {
                "Basic attack model"
            } else {
                "Packing only"
            },
            esc(&row.reason)
        ));
    }
    html.push_str(&format!(
        "</tbody></table></div><p class=\"small-note\">{}</p></section>",
        crate::coverage::LIMITS
    ));
    html
}
fn export_panel(s: &State, p: &Plan) -> String {
    let disabled = if p.exportable() { "" } else { "disabled" };
    let mut html = format!(
        "<section class=\"export-card\"><div class=\"export-heading\"><h2>RuneLite</h2><p>{}</p></div><div class=\"export-actions\"><button data-action=\"copy\" class=\"button gold\" {disabled}>Copy Inventory Setup</button><button data-action=\"share\" class=\"button ghost\">Copy setup link</button></div><details><summary>Other formats & help</summary><div class=\"export-secondary\"><button data-action=\"download-setup\" class=\"button ghost\" {disabled}>Download setup JSON</button><button data-action=\"bank-tags\" class=\"button ghost\">Copy Bank Tags</button></div><ol><li>Enable Inventory Setups and Bank Tags in RuneLite.</li><li>Choose Import setup in Inventory Setups.</li><li>Paste the copied text.</li></ol><p>Check staff charges before your trip.</p></details>",
        if p.exportable() {
            if s.phase.is_some() {
                "Exports Start gear."
            } else {
                ""
            }
        } else {
            "Review missing items in Preparation."
        }
    );
    if let Some(text) = &s.export_text {
        html.push_str(&format!("<label class=\"export-label\">Setup text<textarea readonly rows=\"5\">{}</textarea></label>",esc(text)));
    }
    html.push_str("</section>");
    html
}
fn next_goal(s: &State) -> String {
    let Some(_) = s.saved.snapshot.selected() else {
        return String::new();
    };
    let goals = planner::goals(&s.saved.snapshot, s.saved.options.boss);
    let pins = s.pins();
    if let Some(id) = pins.iter().find_map(|p| {
        p.strip_prefix("item:")
            .and_then(|id| id.parse::<i32>().ok())
    }) {
        if let Some(gear) = planner::gear(id) {
            let owned = s.saved.snapshot.available(false).contains_key(&id);
            let wiki = js_sys::encode_uri_component(&gear.name.replace(' ', "_"))
                .as_string()
                .unwrap_or_default();
            return format!(
                r#"<section class="panel goal-teaser"><h3>Your bossing upgrade</h3><p>{}<strong>{}</strong></p><p>{}</p>{} · <a href="https://oldschool.runescape.wiki/w/{wiki}" target="_blank" rel="noreferrer">How to obtain</a></section>"#,
                image(id),
                esc(&gear.name),
                if owned {
                    "This item is now in your bank. Compare it in your equipment slots."
                } else {
                    "Not owned yet. This goal stays private and does not change your current setup."
                },
                button(&format!("pin:item:{id}"), "Untrack upgrade", "text-button")
            );
        }
    }

    if s.saved.options.boss == Boss::Custom {
        return "<section class=\"panel goal-teaser\"><h3>Your bossing upgrade</h3><p>Choose an upgrade from a gear slot's BIS list.</p></section>".into();
    }
    let goal = goals
        .iter()
        .find(|g| pins.iter().any(|p| p == g.id) && !g.complete)
        .or_else(|| goals.iter().find(|g| !g.complete));
    let Some(g) = goal else {
        return "<section class=\"panel goal-teaser\"><h3>Starter milestones reached</h3><p>Review your encounter supplies and unlocks.</p></section>".into();
    };
    format!(
        "<section class=\"panel goal-teaser\"><h3>Next bossing upgrade</h3><p><strong>{}</strong></p><p>{}</p><div class=\"progress-track\"><span style=\"width:{}%\"></span></div><small>{}</small>{} · <a href=\"{}\" target=\"_blank\" rel=\"noreferrer\">Wiki guide</a></section>",
        g.title,
        esc(g.reason),
        g.progress,
        esc(&g.next),
        button(
            &format!("pin:{}", g.id),
            if pins.iter().any(|p| p == g.id) {
                "Untrack upgrade"
            } else {
                "Track this upgrade"
            },
            "text-button"
        ),
        g.link
    )
}
fn account_page(s: &State) -> String {
    let m = s.saved.snapshot.selected();
    let mut html = format!(
        r#"<div class="page-heading"><div><h1>Your bank</h1><p>Saved on this device.</p></div></div><div class="account-grid"><section class="panel account-form"><h2>1. Import bank</h2><p>Export from RuneLite Bank Memory. Paste the table or choose a file.</p><label>Bank table or private backup<textarea id="import-data" rows="6" placeholder="Item id    Item name    Item quantity"></textarea></label>{}<label>Choose bank file<input id="import-file" type="file" accept=".txt,.tsv,.json,text/plain,application/json"></label><p>Backups include your bank. Keep them private.</p></section><section class="panel account-form"><h2>2. Combat levels</h2><label>RuneScape name (optional)<input id="player-name" maxlength="12" value="{}" autocomplete="off"></label><button class="button ghost" data-action="load-levels" {}>Look up public levels</button><p>Uses Wise Old Man’s public levels. Or enter them below.</p><small>Skill snapshot: {}</small><div class="skills-grid">"#,
        button("import", "Import bank", "button gold full"),
        esc(if s.saved.snapshot.source == "Demo" {
            ""
        } else {
            &s.saved.snapshot.selected
        }),
        if s.busy || s.demo_return.is_some() {
            "disabled"
        } else {
            ""
        },
        esc(m
            .and_then(|m| m.last_updated.as_deref())
            .unwrap_or("Unknown / manual"))
    );
    for skill in [
        "Attack",
        "Strength",
        "Defence",
        "Ranged",
        "Magic",
        "Prayer",
        "Hitpoints",
        "Mining",
    ] {
        let i = SKILLS.iter().position(|s| *s == skill).unwrap();
        html.push_str(&format!(r#"<label>{skill}<input id="skill-{i}" type="number" min="1" max="99" value="{}" placeholder="?"></label>"#,m.and_then(|m|m.level(skill)).map(|l|l.to_string()).unwrap_or_default()));
    }
    html.push_str("</div></section></div>");
    html.push_str(&format!(
        "<div class=\"account-tools\">{} {} {}</div>",
        button("backup", "Download private backup", "text-button"),
        if s.demo_return.is_some() {
            button("restore-account", "Return to your account", "button gold")
        } else {
            button("demo", "Explore example", "text-button")
        },
        if s.demo_return.is_some() {
            String::new()
        } else {
            button("forget", "Forget saved account", "text-button danger")
        }
    ));
    html
}
fn selected_encounter(options: &Options) -> Option<&'static crate::encounters::Entry> {
    if options.boss != Boss::Custom {
        return None;
    }
    if let Some(raid) = crate::raids::for_targets(&options.targets) {
        return crate::encounters::catalog()
            .iter()
            .find(|entry| entry.name == raid.name);
    }
    crate::encounters::catalog()
        .iter()
        .filter(|e| e.matches(&options.targets))
        .min_by_key(|e| !crate::encounters::groups().iter().any(|g| g.name == e.name))
}
fn encounter_label(options: &Options) -> &str {
    selected_encounter(options)
        .map(|e| e.name.as_str())
        .unwrap_or(options.boss.name())
}
fn bestiary_assessments(s: &State) -> Vec<crate::bestiary::Assessment> {
    if s.shared_setup.is_some() {
        return crate::encounters::catalog()
            .iter()
            .map(|_| crate::bestiary::Assessment {
                readiness: crate::bestiary::Readiness::Unknown,
                requirements: Default::default(),
                reason: "Shared setups do not include the sender's bank or levels.".into(),
                detail: String::new(),
                wilderness: false,
                preview: None,
            })
            .collect();
    }
    thread_local! {
        static CACHE: RefCell<Option<(String, Vec<crate::bestiary::Assessment>)>> = const { RefCell::new(None) };
    }
    let key = serde_json::to_string(&(
        &s.saved.snapshot,
        s.saved.options.shared,
        &s.saved.options.excluded,
        s.saved.options.exclude_void,
        s.scaredy_cat,
        s.filler_cap,
    ))
    .unwrap_or_default();
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.as_ref().is_none_or(|(old, _)| *old != key) {
            *cache = Some((
                key,
                if s.scaredy_cat {
                    crate::bestiary::assess_wilderness(
                        &s.saved.snapshot,
                        &s.saved.options,
                        s.filler_cap,
                    )
                } else {
                    crate::bestiary::assess_catalog(&s.saved.snapshot, &s.saved.options)
                },
            ));
        }
        cache.as_ref().unwrap().1.clone()
    })
}
fn extra_filter_label(s: &State) -> String {
    let mut labels = vec!["Gear & requirements"];
    if s.gear_filter != crate::bestiary::GearFilter::Any {
        labels.push(s.gear_filter.label());
    }
    if s.requirements_filter != crate::bestiary::RequirementsFilter::Any {
        labels.push(s.requirements_filter.label());
    }
    labels.join(" · ")
}
fn encounter_filter_button(prefix: &str, label: &str, active: bool) -> String {
    format!(
        r#"<button id="{prefix}-{}" class="encounter-filter {}" data-action="{prefix}:{label}" aria-pressed="{active}">{label}</button>"#,
        label.replace(' ', "-"),
        if active { "active" } else { "" }
    )
}
fn encounter_catalog(s: &State) -> String {
    if !s.bestiary_open
        && !s.building_group
        && !(s.saved.options.boss == Boss::Custom && s.saved.options.targets.is_empty())
    {
        return String::new();
    }
    let mut html = format!(
        r#"<section class="encounter-browser"><h2>Choose your fight</h2><p>Based on your bank and levels.</p><label><span class="slot-caption">Search encounters</span><input id="encounter-search" type="search" placeholder="Search bosses or raids" value="{}" autocomplete="off"></label><nav class="encounter-filters" aria-label="Encounter categories">"#,
        esc(&s.encounter_query)
    );
    for filter in crate::bestiary::CATEGORY_FILTERS {
        html.push_str(&encounter_filter_button(
            "encounter-filter",
            filter,
            s.encounter_filter == filter,
        ));
    }
    html.push_str(&format!("</nav><div class=\"encounter-tools\"><details id=\"encounter-extra-filters\" class=\"secondary-section\" {}><summary id=\"encounter-filter-label\">{}</summary><fieldset class=\"bestiary-filter-group\"><legend>Gear readiness</legend><nav class=\"encounter-filters\" aria-label=\"Gear readiness filters\">",if s.encounter_filters_open {"open"}else{""},esc(&extra_filter_label(s))));
    for filter in crate::bestiary::GearFilter::ALL {
        html.push_str(&encounter_filter_button(
            "gear-filter",
            filter.label(),
            s.gear_filter == filter,
        ));
    }
    html.push_str("</nav></fieldset><fieldset class=\"bestiary-filter-group\"><legend>Requirements</legend><nav class=\"encounter-filters\" aria-label=\"Requirements filters\">");
    for filter in crate::bestiary::RequirementsFilter::ALL {
        html.push_str(&encounter_filter_button(
            "requirements-filter",
            filter.label(),
            s.requirements_filter == filter,
        ));
    }
    html.push_str("</nav></fieldset><p class=\"bestiary-filter-note\">Known levels met checks recorded Slayer requirements only. Quest, key and task access remain unverified. Gear and requirements filters apply together.</p>");
    let location = match s.wilderness_location.as_str() {
        "safe" => " · avoiding Wilderness",
        "wild" => " · Wilderness only",
        _ => "",
    };
    let kit = if s.scaredy_cat {
        " · 3-item kit on"
    } else {
        ""
    };
    html.push_str(&format!("</details><details id=\"wilderness-options\" class=\"secondary-section\" {}><summary>Wilderness options{location}{kit}</summary>",if s.wilderness_options_open {"open"}else{""}));
    html.push_str(&wilderness_controls(s));
    html.push_str("</details></div><p class=\"bestiary-disclaimer\">Gear readiness and entry requirements are shown separately. Check supplies and mechanics in each card.</p><div id=\"encounter-results\">");
    html.push_str(&encounter_results(s));
    html.push_str("</div></section>");
    html
}

fn encounter_results(s: &State) -> String {
    let query = s.encounter_query.trim().to_lowercase();
    let popular = &crate::encounters::browser_catalog().popular;
    let assessments = bestiary_assessments(s);
    let mut entries = crate::encounters::catalog()
        .iter()
        .enumerate()
        .filter(|(index, e)| {
            let exposed = crate::wilderness::exposure(&e.name) != crate::wilderness::Exposure::None;
            if s.wilderness_location == "safe" && exposed
                || s.wilderness_location == "wild" && !exposed
            {
                return false;
            }
            if crate::encounters::groups()
                .iter()
                .any(|g| g.aliases.contains(&e.name))
            {
                return false;
            }
            let matches = e.name.to_lowercase().contains(&query)
                || crate::encounters::groups()
                    .iter()
                    .filter(|g| g.name == e.name)
                    .any(|g| g.aliases.iter().any(|a| a.to_lowercase().contains(&query)));
            matches
                && crate::bestiary::matches_filters(
                    &assessments[*index],
                    s.gear_filter,
                    s.requirements_filter,
                )
                && if s.encounter_filter == "Popular" {
                    popular.contains(&e.name)
                } else {
                    ["Popular", "For you", "All"].contains(&s.encounter_filter.as_str())
                        || e.category() == s.encounter_filter
                }
        })
        .collect::<Vec<_>>();
    if query.is_empty() && s.encounter_filter == "Popular" {
        entries.sort_by_key(|(_, e)| {
            popular
                .iter()
                .position(|name| name == &e.name)
                .unwrap_or(99)
        });
    }
    if s.encounter_filter == "For you" {
        entries.sort_by(|(a, ea), (b, eb)| {
            assessments[*a]
                .readiness
                .cmp(&assessments[*b].readiness)
                .then_with(|| ea.name.cmp(&eb.name))
        });
    }
    let mut html = String::new();
    for (index, e) in entries {
        let assessment = if s.shared_setup.is_some() {
            crate::bestiary::Assessment {
                readiness: crate::bestiary::Readiness::Unknown,
                requirements: Default::default(),
                reason: "Shared setups do not include the sender's bank or levels.".into(),
                detail: String::new(),
                wilderness: false,
                preview: None,
            }
        } else {
            assessments[index].clone()
        };
        html.push_str(&crate::bestiary::card(e, index, &assessment));
    }
    if html.is_empty() {
        html.push_str(&format!(
            "<p>No encounters match these filters. {}</p>",
            button("clear-encounters", "Clear filters", "text-button")
        ));
    }
    html
}
fn refresh_encounter_browser(s: &State) {
    if let Some(label) = document().get_element_by_id("encounter-filter-label") {
        label.set_text_content(Some(&extra_filter_label(s)));
    }
    if let Some(results) = document().get_element_by_id("encounter-results") {
        results.set_inner_html(&encounter_results(s));
    }
    let buttons = crate::bestiary::CATEGORY_FILTERS
        .into_iter()
        .map(|v| {
            (
                format!("encounter-filter-{}", v.replace(' ', "-")),
                s.encounter_filter == v,
            )
        })
        .chain(crate::bestiary::GearFilter::ALL.into_iter().map(|v| {
            (
                format!("gear-filter-{}", v.label().replace(' ', "-")),
                s.gear_filter == v,
            )
        }))
        .chain(
            crate::bestiary::RequirementsFilter::ALL
                .into_iter()
                .map(|v| {
                    (
                        format!("requirements-filter-{}", v.label().replace(' ', "-")),
                        s.requirements_filter == v,
                    )
                }),
        );
    for (id, active) in buttons {
        if let Some(el) = document().get_element_by_id(&id) {
            el.set_class_name(if active {
                "encounter-filter active"
            } else {
                "encounter-filter"
            });
            let _ = el.set_attribute("aria-pressed", if active { "true" } else { "false" });
        }
    }
}
fn custom_group(s: &State) -> String {
    let mut html = String::from(
        r#"<section class="panel account-form"><h2>Your encounter group</h2><p>Choose attack styles yourself. The planner packs one shared set of switches; special mechanics, movement and supplies are outside its combat model.</p><label>Boss or monster<input id="monster" list="monsters" placeholder="Search monsters…"></label><datalist id="monsters">"#,
    );
    for m in planner::monsters() {
        html.push_str(&format!(
            r#"<option value="{} · {}"></option>"#,
            esc(&m.name),
            m.id
        ));
    }
    html.push_str(r#"</datalist><label>Attack style<select id="target-style"><option>Melee</option><option>Stab</option><option>Slash</option><option>Crush</option><option>Ranged</option><option>Magic</option></select></label>"#);
    if let Some(entry) = selected_encounter(&s.saved.options) {
        html.push_str(&format!(
            "<p class=\"notice\"><strong>{}</strong>: {}</p>",
            esc(&entry.name),
            esc(&entry.note)
        ));
    }
    html.push_str(&button("add-target", "Add to group", "button ghost"));
    for (i, t) in s.saved.options.targets.iter().enumerate() {
        html.push_str(&format!(
            "<p>{}. {} · {} {}</p>",
            i + 1,
            esc(&t.name()),
            esc(&t.style),
            button(&format!("remove-target:{i}"), "Remove", "text-button")
        ));
    }
    html.push_str("</section>");
    html
}

fn inventory_editor(s: &State, index: usize) -> String {
    let mut filters = String::from("<option value=\"All\">All types</option>");
    for category in crate::inventory::Category::ALL {
        let label = category.label();
        filters.push_str(&format!(
            "<option value=\"{label}\" {}>{label}</option>",
            if s.inventory_filter == label {
                "selected"
            } else {
                ""
            }
        ));
    }
    let quantity = s
        .plan()
        .inventory
        .get(index)
        .map(|i| i.quantity)
        .unwrap_or(1);
    format!(
        r#"<div class="picker-backdrop"><section class="gear-picker inventory-picker" role="dialog" aria-modal="true" aria-labelledby="inventory-picker-title"><div class="panel-heading"><h2 id="inventory-picker-title">Inventory · slot {}</h2>{}</div><div class="picker-controls"><label>Search your items<input id="inventory-search" type="search" placeholder="Search owned items…" value="{}" autocomplete="off"></label><label>Item type<select id="inventory-filter">{filters}</select></label></div><div class="picker-actions"><label class="inventory-stack-label">Stack quantity<input id="inventory-stack" type="number" min="1" max="2147483647" value="{}"></label>{} {}</div><p class="picker-note">Food: HP per slot. Potions: restoration or boost within each role, then doses. Stack quantity applies to reviewed runes/ammo.</p><div class="picker-scroll" id="inventory-pick-results">{}</div></section></div>"#,
        index + 1,
        button("close-inventory", "Close ×", "phase"),
        esc(&s.inventory_query),
        quantity.max(1),
        button("reset-inventory", "Restore automatic", "phase"),
        button("empty-inventory", "Leave empty", "phase"),
        inventory_choices(s)
    )
}

fn inventory_available(s: &State, p: &Plan, slot: usize, id: i32) -> u32 {
    let owned = s
        .saved
        .snapshot
        .available(false)
        .get(&id)
        .copied()
        .unwrap_or(0) as u64;
    inventory_remaining(p, slot, id, owned)
}
fn inventory_remaining(p: &Plan, slot: usize, id: i32, owned: u64) -> u32 {
    let worn: u64 = p
        .equipment
        .iter()
        .enumerate()
        .filter(|(_, item)| **item == Some(id))
        .map(|(slot, _)| u64::from(planner::equipped_quantity(slot, id, p.ammo_shots)))
        .sum();
    let packed: u64 = p
        .inventory
        .iter()
        .enumerate()
        .filter(|(index, item)| *index != slot && item.id == id)
        .map(|(_, item)| item.quantity as u64)
        .sum();
    owned.saturating_sub(worn + packed).min(u32::MAX as u64) as u32
}

fn inventory_choices(s: &State) -> String {
    let Some(index) = s.edit_inventory else {
        return String::new();
    };
    let Some(member) = s.saved.snapshot.selected() else {
        return String::new();
    };
    let plan = s.plan();
    let current = plan.inventory.get(index).map(|p| p.id);
    let query = s.inventory_query.trim().to_lowercase();
    let owned = s.saved.snapshot.available(false);
    let mut choices: Vec<_> = owned
        .iter()
        .map(|(&id, &quantity)| (id, quantity))
        .filter(|(_, quantity)| *quantity > 0)
        .filter_map(|(id, quantity)| {
            let name = member
                .item_names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| planner::item_name(id));
            let category = crate::inventory::category(id, &name);
            if (!query.is_empty() && !name.to_lowercase().contains(&query))
                || (s.inventory_filter != "All" && s.inventory_filter != category.label())
            {
                return None;
            }
            let value = crate::supplies::value(category, id, &name, member);
            Some((category, name, id, quantity, value))
        })
        .collect();
    choices.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.4.compare(&b.4))
            .then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase()))
            .then(a.2.cmp(&b.2))
    });
    let mut html = String::new();
    let mut last = None;
    for (category, name, id, quantity, value) in choices {
        if last != Some((category, value.role)) {
            html.push_str(&format!(
                "<h3 class=\"owned-heading\">{}</h3>",
                if value.role.is_empty() {
                    category.label().into()
                } else {
                    format!("{} · {}", category.label(), value.role)
                }
            ));
            last = Some((category, value.role));
        }
        let available = inventory_remaining(&plan, index, id, owned[&id] as u64);
        html.push_str(&format!(r#"<button class="gear-pick {}" data-action="pick-inventory:{id}" {}>{}<span><strong>{}</strong><small>{}</small><small>{}{available} available</small></span><b>× {quantity}</b></button>"#,if current==Some(id){"selected"}else{""},if available==0{"disabled"}else{""},image(id),esc(&name),esc(if value.detail.is_empty(){category.label()}else{&value.detail}),if current==Some(id){"In this slot · "}else{""}));
    }
    if html.is_empty() {
        html.push_str(
            "<p class=\"picker-note\">No owned items match. Try another type or search.</p>",
        );
    }
    html
}

fn encounter_targets(s: &State) -> Vec<i32> {
    let mut targets = match s.saved.options.boss {
        Boss::Barrows => vec![1673, 1674, 1676, 1677, 1675, 1672],
        Boss::Toa => vec![
            11778, 11719, 11730, 11789, 11751, 11753, 11756, 11755, 11758, 11761, 11762,
        ],
        Boss::Custom => s.saved.options.targets.iter().map(|t| t.id).collect(),
    };
    if targets.contains(&8059) {
        targets.push(100020);
    }
    if targets.contains(&14176) || targets.contains(&100011) {
        targets.push(100030);
    }
    let sources = targets.clone();
    targets.extend(
        crate::encounters::mechanics()
            .windows
            .iter()
            .filter(|w| {
                w.comparison_alternative && sources.contains(&w.npc_id)
                    || w.comparison_sources.iter().any(|id| sources.contains(id))
            })
            .map(|w| w.id),
    );
    let mut seen = std::collections::BTreeSet::new();
    targets.retain(|id| seen.insert(*id));
    targets
}
fn comparison_style(s: &State) -> &str {
    s.edit_style
        .as_deref()
        .unwrap_or(if s.saved.options.boss == Boss::Barrows {
            "Magic"
        } else if s.saved.options.boss == Boss::Custom {
            s.saved
                .options
                .targets
                .first()
                .map(|t| t.style.as_str())
                .unwrap_or("Melee")
        } else {
            "Melee"
        })
}
fn comparison_monster(s: &State) -> Option<&'static planner::Monster> {
    let targets = encounter_targets(s);
    let id = s
        .compare_target
        .filter(|id| targets.contains(id))
        .or_else(|| targets.first().copied())?;
    planner::monsters().iter().find(|m| m.id == id)
}
fn comparison_spell(
    s: &State,
    member: &account::Member,
    set: &[Option<i32>],
    target: &planner::Monster,
) -> &'static str {
    let mut owned = s.saved.snapshot.available(s.saved.options.shared);
    owned.retain(|id, _| !s.saved.options.item_excluded(*id));
    crate::rules::spell_name_for_target(
        member,
        set,
        &owned,
        &s.saved.options.combat,
        target,
        s.saved.options.raid_level,
    )
}
fn incoming_choice(s: &State, target: &planner::Monster) -> Option<String> {
    let choices = crate::defence::attacks(target);
    s.incoming_attack
        .as_ref()
        .filter(|a| choices.contains(a))
        .cloned()
        .or_else(|| choices.first().cloned())
}
fn sustain_panel(
    s: &State,
    p: &Plan,
    set: &[Option<i32>],
    member: &account::Member,
    target: &planner::Monster,
) -> String {
    let settings = &s.saved.options.combat;
    let mut html = String::from("<section class=\"sustain-panel\"><h3>Damage taken & prayer</h3>");
    html.push_str(&prayer_book(s, member));
    let choices = crate::defence::attacks(target);
    if !choices.is_empty() {
        html.push_str("<div class=\"incoming-choices\" role=\"group\" aria-label=\"Incoming attack scenario\"><h4>Incoming attack scenario</h4><div class=\"combat-subtype-options\">");
        for choice in &choices {
            let active = incoming_choice(s, target).as_deref() == Some(choice);
            html.push_str(&format!("<button class=\"phase {}\" data-action=\"incoming-choice:{}\" aria-pressed=\"{active}\">{}</button>",if active {"active"}else{""},esc(choice),esc(choice)));
        }
        html.push_str("</div></div>");
    }
    match incoming_choice(s, target)
        .ok_or("Incoming attack data unavailable.".to_string())
        .and_then(|attack| crate::defence::estimate(member, set, target, &attack, settings))
    {
        Ok(r) => {
            let healing = if r.healing_modelled {
                format!("{:.3}", r.healing_per_second)
            } else {
                "Unmodelled".into()
            };
            html.push_str(&format!("<dl class=\"combat-values\"><div><dt>Incoming DPS · scenario</dt><dd>{:.3}</dd></div><div><dt>Enemy hit chance</dt><dd>{:.1}%</dd></div><div><dt>Boss healing / second</dt><dd>{}</dd></div></dl><details><summary>Damage model details</summary><p class=\"small-note\">{}</p></details>",r.damage_per_second,r.hit_chance*100.0,healing,esc(&r.note)));
            if r.healing_modelled && r.healing_per_second > 0.0 {
                if let Ok(outgoing) = combat::calculate_configured(
                    member,
                    set,
                    comparison_style(s),
                    target,
                    comparison_spell(s, member, set, target),
                    s.saved.options.raid_level,
                    settings,
                ) {
                    if outgoing.dps <= r.healing_per_second {
                        html.push_str("<p class=\"combat-unavailable\">Boss healing cancels your damage in this scenario. Adjust your gear.</p>");
                    }
                }
            }
        }
        Err(reason) => html.push_str(&format!(
            "<p class=\"small-note\">Incoming DPS unavailable: {}</p>",
            esc(&reason)
        )),
    }
    let bonus = crate::prayer::bonus(set);
    let drain = crate::prayer::drain_per_minute(bonus, settings);
    let duration = crate::prayer::minutes(member, set, &p.inventory, settings)
        .map(|v| format!("{v:.1} minutes"))
        .unwrap_or_else(|| {
            if drain == 0.0 {
                "No active prayer drain".into()
            } else {
                "Level unavailable".into()
            }
        });
    html.push_str(&format!("<table class=\"bonus-table\"><tbody><tr><th>Prayer bonus</th><td>{bonus:+}</td></tr><tr><th>Drain</th><td>{drain:.2} points / minute</td></tr><tr><th>Prayer supply</th><td>{}</td></tr></tbody></table><details><summary>Prayer estimate details</summary><p class=\"small-note\">Full starting prayer, continuous use and this equipment. Excludes flicking, regeneration, restoration bonuses and encounter drains. Incoming damage covers one attack scenario; unknown chip damage and specials are not zero.</p></details>",esc(&duration)));
    html.push_str("</section>");
    html
}
fn combat_assumptions(
    s: &State,
    p: &Plan,
    m: &account::Member,
    target: &planner::Monster,
    set: &[Option<i32>],
) -> String {
    let settings = &s.saved.options.combat;
    let mut html = format!(
        "<details id=\"combat-assumptions\" {}><summary>Boosts, darts &amp; debuffs</summary><div class=\"sustain-controls\">",
        if s.combat_options_open { "open" } else { "" }
    );
    for (channel, label, value) in [
        ("melee", "Melee boost", &settings.boosts.melee),
        ("ranged", "Ranged boost", &settings.boosts.ranged),
        ("magic", "Magic boost", &settings.boosts.magic),
        ("defence", "Defence boost", &settings.boosts.defence),
        ("encounter", "Raid boost", &settings.boosts.encounter),
    ] {
        html.push_str(&format!(
            "<label>{label}<select id=\"boost-{channel}\"><option {}>None</option>",
            if value == "None" { "selected" } else { "" }
        ));
        for boost in crate::modifiers::boost_options(channel) {
            html.push_str(&format!(
                "<option value=\"{}\" {}>{}</option>",
                esc(&boost.id),
                if value == &boost.id { "selected" } else { "" },
                esc(&boost.label)
            ));
        }
        html.push_str("</select></label>");
    }
    html.push_str(&format!("<label>Current Hitpoints (Dharok / Torag)<input id=\"current-hitpoints\" type=\"number\" min=\"1\" max=\"255\" placeholder=\"Full health\" value=\"{}\"></label>",
        settings.current_hitpoints.map(|hp|hp.to_string()).unwrap_or_default()));
    if set[3]
        .and_then(crate::rules::weapon)
        .is_some_and(|r| r.weapon.family == "blue_moon")
    {
        html.push_str("<label>Blue Moon free melee<select id=\"blue-moon-melee\">");
        for value in ["Off", "Stab", "Slash", "Crush"] {
            html.push_str(&format!(
                "<option {}>{value}</option>",
                if settings.blue_moon_melee == value {
                    "selected"
                } else {
                    ""
                }
            ));
        }
        html.push_str("</select></label><p class=\"small-note\">Enable Blue Moon's automatic melee in game and stay in melee range to count Frostweaver strikes. Off counts spell damage only.</p>");
    }
    let owned = s.saved.snapshot.available(s.saved.options.shared);
    if owned
        .keys()
        .any(|id| crate::rules::weapon(*id).is_some_and(|r| r.has(crate::rules::Effect::Obligator)))
    {
        html.push_str("<label>Obligator attack<select id=\"obligator-style\">");
        for value in ["Thrust accurate", "Thrust aggressive", "Smash"] {
            html.push_str(&format!(
                "<option {}>{value}</option>",
                if settings.obligator_style == value {
                    "selected"
                } else {
                    ""
                }
            ));
        }
        html.push_str("</select></label><p class=\"small-note\">Charged Thrust keeps the best of 2–4 damage rolls and gains size damage. Smash is a three-tick single hit without either passive. Inquisitor and encounter minimum-hit ordering for charged Thrust remain unverified.</p>");
    }
    if owned.keys().any(|id| {
        crate::rules::weapon(*id).is_some_and(|r| r.has(crate::rules::Effect::Soulreaper))
    }) {
        html.push_str("<label>Soulreaper held stacks<select id=\"soulreaper-stacks\">");
        for stacks in 0..=5 {
            html.push_str(&format!(
                "<option value=\"{stacks}\" {}>{stacks}</option>",
                if settings.soulreaper_stacks == stacks {
                    "selected"
                } else {
                    ""
                }
            ));
        }
        html.push_str("</select></label>");
    }
    if owned
        .keys()
        .any(|id| crate::rules::equipment_effect(*id, "bulwark"))
    {
        html.push_str(&format!("<label><input id=\"bulwark-block\" type=\"checkbox\" {}> Dinh's Block active · prevents attacks</label>", if settings.bulwark_block { "checked" } else { "" }));
    }
    for note in crate::equipment_effects::notes(set, settings) {
        html.push_str(&format!("<p class=\"small-note\">{}</p>", esc(&note)));
    }
    html.push_str("<p class=\"small-note\">Normal attacks only. Special-attack damage, energy recovery and rotations are excluded; selected defence reductions assume successful effects already applied.</p>");
    let internal = set[3]
        .and_then(crate::rules::weapon)
        .filter(|r| !r.family.internal_ammo.is_empty())
        .map(|r| r.family.internal_ammo.clone());
    if internal.is_some()
        || owned
            .keys()
            .any(|id| crate::rules::weapon(*id).is_some_and(|r| !r.family.internal_ammo.is_empty()))
    {
        html.push_str("<label>Darts to load<select id=\"loaded-dart\"><option value=\"0\">Auto · best owned compatible darts</option>");
        for id in internal.unwrap_or_else(crate::rules::all_internal_ammo_ids) {
            html.push_str(&format!(
                "<option value=\"{id}\" {}>{} · {} owned</option>",
                if settings.loaded_dart == Some(id) {
                    "selected"
                } else {
                    ""
                },
                esc(&planner::item_name(id)),
                owned.get(&id).copied().unwrap_or(0)
            ));
        }
        html.push_str(&format!("</select></label><label><input id=\"loaded-ammo-confirmed\" type=\"checkbox\" {} {}> Already loaded: at least {} darts</label>",if settings.loaded_ammo_confirmed{"checked"}else{""},if settings.loaded_dart.is_none(){"disabled"}else{""}, settings.ammo_shots));
        if let Some(ammo) = &p.loaded_ammo {
            html.push_str(&format!("<p class=\"small-note\">Load {} × {} inside the weapon. Check charges; darts occupy no worn ammo or bag slot.</p>",esc(&planner::item_name(ammo.id)),ammo.quantity));
        }
    }
    html.push_str(
        "</div><details><summary>Current levels</summary><table class=\"bonus-table\"><tbody>",
    );
    for skill in ["Attack", "Strength", "Defence", "Ranged", "Magic"] {
        if let Some(level) = crate::modifiers::level(m, skill, &settings.boosts) {
            html.push_str(&format!(
                "<tr><th>{skill}</th><td>{} → {level}</td></tr>",
                m.level(skill).unwrap_or(0)
            ));
        }
    }
    html.push_str(&format!("</tbody></table></details><p class=\"small-note\">Reductions against {} · successful hits or damage already dealt</p><div class=\"sustain-controls\">",esc(&target.name)));
    let r = settings
        .reductions
        .get(&target.id)
        .cloned()
        .unwrap_or_default();
    for (id, label, value) in [
        ("elder_maul", "Elder maul hits", r.elder_maul),
        ("dwh", "Dragon warhammer hits", r.dwh),
        ("arclight", "Arclight hits", r.arclight),
        ("emberlight", "Emberlight hits", r.emberlight),
        ("tonalztic", "Tonalztic hits", r.tonalztic),
        ("seercull", "Seercull damage", r.seercull),
        ("bgs", "BGS damage", r.bgs),
        ("ayak", "Ayak defence drain", r.ayak),
    ] {
        html.push_str(&format!("<label>{label}<input id=\"reduction-{id}\" type=\"number\" min=\"0\" max=\"{}\" step=\"1\" value=\"{value}\"></label>",if ["bgs","seercull","ayak"].contains(&id){10000}else{20}));
    }
    for (id, label, checked) in [
        ("accursed", "Accursed sceptre landed", r.accursed),
        ("vulnerability", "Vulnerability active", r.vulnerability),
    ] {
        html.push_str(&format!(
            "<label><input id=\"reduction-{id}\" type=\"checkbox\" {}> {label}</label>",
            if checked { "checked" } else { "" }
        ));
    }
    html.push_str("</div><p class=\"small-note\">Peak active boosts and successful reductions are assumptions. No decay, drinking delay, venom or special-attack damage. Accursed takes priority over Vulnerability. Wear requirements use base levels.</p></details>");
    html
}
fn combat_breakdown(
    s: &State,
    member: &account::Member,
    set: &[Option<i32>],
    target: &planner::Monster,
    r: &combat::ResultStats,
) -> String {
    let mut html = String::from("<div class=\"combat-breakdown\">");
    if ["light", "standard", "heavy"].contains(&r.attack_type.as_str()) {
        html.push_str("<h3>Ranged defence by ammunition</h3><table class=\"wiki-table\"><thead><tr><th>Type</th><th>Ammunition</th><th>Target bonus</th><th>Used</th></tr></thead><tbody>");
        for (kind, name, ammo) in [
            ("light", "Light", "Darts, knives, thrown"),
            ("standard", "Standard", "Arrows, crystal bows"),
            ("heavy", "Heavy", "Bolts, javelins"),
        ] {
            let reduced = s
                .saved
                .options
                .combat
                .reductions
                .get(&target.id)
                .and_then(|reductions| crate::modifiers::reduce(target, reductions).ok());
            let effective_target = reduced.as_ref().unwrap_or(target);
            let value = effective_target
                .defensive
                .get(kind)
                .map(|v| format!("{v:+}"))
                .unwrap_or("Unknown".into());
            html.push_str(&format!(
                "<tr><th>{name}</th><td>{ammo}</td><td>{value}</td><td>{}</td></tr>",
                if r.attack_type == kind {
                    "Selected weapon"
                } else {
                    "—"
                }
            ));
        }
        html.push_str(&format!("</tbody></table><p class=\"small-note\">Your ammunition determines the category. Lower target defence improves accuracy; it does not increase the base max hit. Selected defence roll: {}.</p>",r.defence_roll));
    } else if r.attack_type == "magic" {
        let mut owned = s.saved.snapshot.available(s.saved.options.shared);
        owned.retain(|id, _| !s.saved.options.item_excluded(*id));
        let candidates =
            crate::rules::elemental_candidates(member, set, &owned, &s.saved.options.combat);
        html.push_str("<h3>Magic elements</h3><table class=\"wiki-table\"><thead><tr><th>Element</th><th>Target weakness</th><th>Spell</th><th>Max hit</th><th>Hit chance</th><th>DPS</th></tr></thead><tbody>");
        for (element, label) in [
            ("air", "Air"),
            ("water", "Water"),
            ("earth", "Earth"),
            ("fire", "Fire"),
        ] {
            let weakness = target
                .weakness
                .as_ref()
                .filter(|w| w.element == element)
                .map(|w| w.severity)
                .unwrap_or(0);
            html.push_str(&format!("<tr><th>{label}</th><td>{weakness}%</td>"));
            let manual = (!matches!(
                s.saved.options.combat.spell_selection,
                crate::rules::SpellSelection::Auto
            ))
            .then(|| {
                crate::rules::spells().iter().find(|spell| {
                    spell.name == r.spell && spell.element.as_deref() == Some(element)
                })
            })
            .flatten();
            let candidate = manual.or_else(|| {
                candidates
                    .iter()
                    .copied()
                    .find(|spell| spell.element.as_deref() == Some(element))
            });
            if let Some(spell) = candidate {
                match combat::calculate_configured(
                    member,
                    set,
                    "Magic",
                    target,
                    &spell.name,
                    s.saved.options.raid_level,
                    &s.saved.options.combat,
                ) {
                    Ok(calc) => html.push_str(&format!(
                        "<td>{}{}</td><td>{}</td><td>{:.1}%</td><td>{:.3}</td>",
                        esc(&spell.name),
                        if spell.name == r.spell {
                            " · selected"
                        } else {
                            ""
                        },
                        calc.max_hit,
                        calc.accuracy * 100.0,
                        calc.dps
                    )),
                    Err(reason) => html.push_str(&format!(
                        "<td>{}</td><td colspan=\"3\">Estimate unavailable: {}</td>",
                        esc(&spell.name),
                        esc(&reason)
                    )),
                }
            } else {
                html.push_str(&format!(
                    "<td colspan=\"4\">{}</td>",
                    esc(&crate::rules::elemental_unavailability(
                        member,
                        set,
                        &s.saved.options.combat,
                        element
                    ))
                ));
            }
            html.push_str("</tr>");
        }
        html.push_str(&format!("</tbody></table><p class=\"small-note\">Rows show the automatic available spell for each element, or your planned spell. Rune availability uses your {}-cast budget (Gear → Adjust supplies). Missing runes for a manual choice are shown above. Staff-provided runes are included; check tome pages and charges.</p>", s.saved.options.combat.spell_casts));
        if let Some(e) = &r.elemental {
            html.push_str(&format!("<h3>{} · calculation</h3><dl class=\"combat-values\"><div><dt>Base spell max</dt><dd>{}</dd></div><div><dt>After gear, prayer & task</dt><dd>{}</dd></div><div><dt>Elemental damage · {}%</dt><dd>+{}</dd></div><div><dt>Elemental accuracy roll · {}%</dt><dd>+{}</dd></div></dl><p class=\"small-note\">Weakness adds a percentage of base spell damage and the base accuracy roll ({}), after other bonuses. Damage is rounded down at each stage. {} Final max hit: {}. Damage tiers use base Magic; active boosts can unlock spells and improve accuracy.</p>",esc(&r.spell),e.base_max,e.damage_before_weakness,e.weakness_percent,e.weakness_damage,e.weakness_percent,e.weakness_accuracy,e.base_attack_roll,if e.tome { "Matching charged tome multiplies damage by 1.10 after weakness." } else { "" },r.max_hit));
        } else {
            html.push_str("<p class=\"small-note\">This powered or non-elemental spell receives no elemental weakness bonus.</p>");
        }
    }
    html.push_str("</div>");
    html
}
fn book_icon(name: &str) -> String {
    static ICONS: std::sync::OnceLock<BTreeMap<String, String>> = std::sync::OnceLock::new();
    let icons = ICONS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/book-icons.json")).expect("book icons")
    });
    icons
        .get(name)
        .map(|src| format!("<img src=\"{src}\" alt=\"\" aria-hidden=\"true\">"))
        .unwrap_or_else(|| "<span class=\"book-clear\" aria-hidden=\"true\">×</span>".into())
}
fn book_tile(
    action: &str,
    name: &str,
    label: &str,
    detail: &str,
    active: bool,
    enabled: bool,
) -> String {
    format!(
        "<button class=\"book-tile {}\" data-action=\"{}\" aria-label=\"{}\" aria-pressed=\"{active}\" title=\"{}\" {}><span class=\"book-tile-icon\">{}</span><span class=\"book-tile-name\">{}</span><small>{}</small></button>",
        if active { "active" } else { "" },
        esc(action),
        esc(name),
        esc(&format!("{name}: {detail}")),
        if enabled { "" } else { "disabled" },
        book_icon(name),
        esc(label),
        esc(detail)
    )
}
fn prayer_book(s: &State, member: &account::Member) -> String {
    let settings = &s.saved.options.combat;
    let mut html = String::from(
        "<section class=\"prayer-book\" aria-label=\"Prayer book\"><h4>Prayer book</h4><p class=\"small-note\">Click a supported prayer to toggle it. Protection and stat prayers can be active together.</p><div class=\"prayer-book-groups\"><div role=\"group\" aria-label=\"Protection prayers\"><h5>Protection</h5><div class=\"book-grid\">",
    );
    html.push_str(&book_tile(
        "protect:None",
        "No protection",
        "Off",
        "No overhead",
        settings.protection == "None",
        true,
    ));
    for (value, name, label, level) in [
        ("Magic", "Protect from Magic", "Magic", 37),
        ("Ranged", "Protect from Missiles", "Missiles", 40),
        ("Melee", "Protect from Melee", "Melee", 43),
    ] {
        let active = settings.protection == value;
        let ready = member.level("Prayer").is_some_and(|l| l >= level);
        html.push_str(&book_tile(
            &format!("protect:{value}"),
            name,
            label,
            &format!("{level} Prayer"),
            active,
            ready || active,
        ));
    }
    html.push_str("</div></div><div role=\"group\" aria-label=\"Stat prayers\"><h5>Stat prayers</h5><div class=\"book-grid\">");
    html.push_str(&book_tile(
        "stat-prayer:None",
        "No stat prayer",
        "Off",
        "No stat boost",
        settings.offensive == "None",
        true,
    ));
    for p in crate::prayer::prayers().iter().filter(|p| p.name != "None") {
        let active = settings.offensive == p.name;
        let detail = if p.defence_level > 1 {
            format!("{} Prayer · {} Defence", p.level, p.defence_level)
        } else {
            format!("{} Prayer", p.level)
        };
        html.push_str(&book_tile(
            &format!("stat-prayer:{}", p.name),
            &p.name,
            &p.name,
            &detail,
            active,
            crate::prayer::eligible(member, p) || active,
        ));
    }
    html.push_str("</div></div><div role=\"group\" aria-label=\"Utility prayers\"><h5>Utility</h5><div class=\"book-grid\">");
    html.push_str(&book_tile(
        "toggle-preserve",
        "Preserve",
        "Preserve",
        "55 Prayer",
        settings.preserve,
        settings.preserve || member.level("Prayer").is_some_and(|l| l >= 55),
    ));
    html.push_str("</div></div></div><p class=\"small-note\">Grey icons need higher levels. Prayer and spellbook unlocks still need your confirmation.</p></section>");
    html
}
fn choose_plan_spell(s: &mut State, selection: crate::rules::SpellSelection) {
    if !selection.is_auto()
        && s.saved.options.boss == Boss::Custom
        && !s.saved.options.targets.iter().any(|t| t.style == "Magic")
    {
        let id = comparison_monster(s).map(|t| t.id);
        if let Some(target) = s
            .saved
            .options
            .targets
            .iter_mut()
            .find(|t| Some(t.id) == id)
        {
            target.style = "Magic".into();
            s.phase = None;
        }
    }
    s.saved.options.combat.spell_selection = selection;
    s.edit_style = Some("Magic".into());
}
fn magic_spell_book(s: &State, set: &[Option<i32>]) -> String {
    use crate::rules::{Formula, SpellSelection};
    let rule = set.get(3).copied().flatten().and_then(crate::rules::weapon);
    let elemental = rule
        .as_ref()
        .is_some_and(|r| r.family.formula == Formula::Elemental);
    let mut html = String::from(
        "<section class=\"combat-subtypes spell-book\" aria-label=\"Spell selection\"><h4>Spellbook</h4><div class=\"combat-subtype-options\" role=\"group\" aria-label=\"Magic elements\">",
    );
    let auto = s.saved.options.combat.spell_selection.is_auto();
    html.push_str(&format!("<button class=\"phase {}\" data-action=\"compare-spell-auto\" aria-pressed=\"{auto}\">Auto</button>",if auto{"active"}else{""}));
    let filter = match &s.saved.options.combat.spell_selection {
        SpellSelection::Element(e) => Some(e.as_str()),
        SpellSelection::Spell(name) => crate::rules::spells()
            .iter()
            .find(|r| r.name == *name)
            .and_then(|r| r.element.as_deref()),
        _ => None,
    };
    for (element, label) in [
        ("air", "Air"),
        ("water", "Water"),
        ("earth", "Earth"),
        ("fire", "Fire"),
    ] {
        let active = elemental && filter == Some(element);
        html.push_str(&format!("<button class=\"phase {}\" data-action=\"compare-element:{element}\" aria-pressed=\"{active}\" {} title=\"{}\">{label}</button>",if active{"active"}else{""},if elemental{""}else{"disabled"},if elemental{"Plan with this element"}else{"Elemental spells need a compatible spellcasting staff"}));
    }
    html.push_str("</div>");
    let Some(member) = s.saved.snapshot.selected() else {
        html.push_str("<p>Enter your Magic level to compare spells.</p></section>");
        return html;
    };
    let mut owned = s.saved.snapshot.available(s.saved.options.shared);
    owned.retain(|id, _| !s.saved.options.item_excluded(*id));
    let selected = comparison_monster(s)
        .map(|target| comparison_spell(s, member, set, target))
        .unwrap_or("");
    let mut recipes: Vec<_> = crate::rules::spells()
        .iter()
        .filter(|recipe| {
            rule.as_ref()
                .is_some_and(|r| crate::rules::spell_compatible(r, recipe, set))
                && (!elemental || filter.is_none_or(|e| recipe.element.as_deref() == Some(e)))
        })
        .collect();
    recipes.sort_by_key(|recipe| recipe.level);
    if recipes.is_empty() {
        html.push_str("<p class=\"small-note\">This weapon uses a built-in spell. Equip a spellcasting staff to compare elemental spells.</p>");
    } else {
        html.push_str(
            "<div class=\"book-grid spell-grid\" role=\"group\" aria-label=\"Combat spells\">",
        );
        for recipe in recipes {
            let requirement = crate::rules::spell_requirement(
                member,
                set,
                &owned,
                &s.saved.options.combat,
                &recipe.name,
            );
            let detail = if crate::modifiers::level(member, "Magic", &s.saved.options.combat.boosts)
                .is_none_or(|l| l < recipe.level)
            {
                format!("Requires {} Magic", recipe.level)
            } else if requirement.is_some() {
                format!("{} Magic · needs runes", recipe.level)
            } else {
                format!("{} Magic · runes ready", recipe.level)
            };
            html.push_str(&book_tile(
                &format!("compare-spell:{}", recipe.name),
                &recipe.name,
                &recipe.name,
                &detail,
                recipe.name == selected,
                true,
            ));
        }
        html.push_str("</div>");
        if !selected.is_empty() {
            html.push_str(&format!(
                "<p class=\"spell-selection\">{}: <strong>{}</strong></p>",
                if auto { "Auto choice" } else { "Planned spell" },
                esc(selected)
            ));
            if let Some(requirement) = crate::rules::spell_requirement(
                member,
                set,
                &owned,
                &s.saved.options.combat,
                selected,
            ) {
                html.push_str(&format!(
                    "<p class=\"spell-requirement\" role=\"status\">{}</p>",
                    esc(&requirement)
                ));
            }
        }
    }
    html.push_str("<p class=\"small-note\">Your spell choice is used for Magic phases, rune packing and exported setups. Auto restores automatic spell selection.</p></section>");
    html
}
fn choose_plan_ranged(s: &mut State, selection: crate::rules::RangedType) {
    s.saved.options.combat.ranged_type = selection;
    s.saved.options.gear_overrides.remove("Ranged:3");
    s.saved.options.gear_overrides.remove("Ranged:13");
    if selection.key().is_some()
        && s.saved.options.boss == Boss::Custom
        && !s.saved.options.targets.iter().any(|t| t.style == "Ranged")
    {
        let id = comparison_monster(s).map(|t| t.id);
        if let Some(target) = s
            .saved
            .options
            .targets
            .iter_mut()
            .find(|t| Some(t.id) == id)
        {
            target.style = "Ranged".into();
            s.phase = None;
        }
    }
    s.edit_style = Some("Ranged".into());
}
fn ranged_ammunition_card(s: &State, set: &[Option<i32>]) -> String {
    let weapon = set.get(3).copied().flatten();
    let rule = weapon.and_then(crate::rules::weapon);
    let equipped = rule
        .as_ref()
        .filter(|r| r.family.style == "Ranged")
        .and_then(|r| r.family.attack_types.first());
    let selection = s.saved.options.combat.ranged_type;
    let mut html = format!(
        "<section class=\"combat-subtypes ranged-ammunition\" aria-label=\"Ranged ammunition\"><div class=\"ranged-type-heading\"><h4>Ammunition type</h4><button class=\"phase {}\" data-action=\"ranged-type:auto\" aria-pressed=\"{}\">Auto</button></div><div class=\"ranged-type-grid\" role=\"group\" aria-label=\"Ranged ammunition type\">",
        if selection.key().is_none() {
            "active"
        } else {
            ""
        },
        selection.key().is_none()
    );
    for (kind, label, detail, icon) in [
        ("light", "Light", "Darts & thrown", 811),
        ("standard", "Standard", "Arrows & crystal", 892),
        ("heavy", "Heavy", "Bolts & javelins", 9144),
    ] {
        let chosen = selection.key() == Some(kind);
        let active = chosen || selection.key().is_none() && equipped.is_some_and(|s| s == kind);
        html.push_str(&format!("<button type=\"button\" class=\"ranged-type {}\" data-action=\"ranged-type:{kind}\" aria-label=\"{label}\" aria-pressed=\"{chosen}\">{}<strong>{label}</strong><small>{detail}</small><span class=\"ranged-type-state\">{}</span></button>", if active {"active"}else{""}, image(icon), if chosen {"✓ Selected"}else if active {"In use"}else{""}));
    }
    html.push_str("</div><div class=\"ranged-equipped\">");
    if let Some(id) = weapon.filter(|_| equipped.is_some()) {
        html.push_str(&format!(
            "<span>{}<span>{}</span></span>",
            image(id),
            esc(&planner::item_name(id))
        ));
        let ammo = set.get(13).copied().flatten().or_else(|| {
            s.saved.snapshot.selected().and_then(|m| {
                let mut owned = s.saved.snapshot.available(s.saved.options.shared);
                owned.retain(|id, _| !s.saved.options.item_excluded(*id));
                let member = planner::member_with_stock(m, &owned);
                crate::rules::internal_ammo(&member, id, &s.saved.options.combat)
            })
        });
        if let Some(id) = ammo {
            html.push_str(&format!(
                "<span>{}<span>{} × {}{}</span></span>",
                image(id),
                esc(&planner::item_name(id)),
                planner::equipped_quantity(13, id, s.saved.options.combat.ammo_shots),
                if set.get(13).copied().flatten().is_none() {
                    " · loaded"
                } else {
                    ""
                }
            ));
        } else if rule
            .as_ref()
            .is_some_and(|r| !r.family.ammo.is_empty() || !r.family.internal_ammo.is_empty())
        {
            html.push_str(&format!("<p class=\"spell-requirement\" role=\"status\">Needs compatible ammunition for {} shots. Check Preparation.</p>", s.saved.options.combat.ammo_shots));
        }
    } else {
        html.push_str(&format!(
            "<span>No usable owned {}ranged weapon. Check your bank and Ranged level.</span>",
            if selection.key().is_some() {
                format!("{} ", selection.label())
            } else {
                String::new()
            }
        ));
    }
    html.push_str("</div>");
    if weapon.is_some()
        && selection.key().is_some()
        && !weapon.is_some_and(|id| selection.permits_weapon(id))
    {
        html.push_str("<p class=\"spell-requirement\">This weapon does not match the selected type. Pick the type again to rebuild the weapon and ammo.</p>");
    }
    html.push_str(&format!("<p class=\"small-note\">Choose a type to plan with compatible owned gear. Uses your {}-shot budget. Auto restores automatic selection.</p></section>",s.saved.options.combat.ammo_shots));
    html
}
fn combat_style_controls(s: &State, set: &[Option<i32>], style: &str) -> String {
    let main_style = crate::rules::base_style(style);
    let rule = set
        .get(3)
        .copied()
        .flatten()
        .and_then(|id| crate::rules::weapon_for_style(id, style));
    let mut html = String::from("<nav class=\"combat-style-tabs\" aria-label=\"Combat style\">");
    for candidate in ["Melee", "Ranged", "Magic"] {
        html.push_str(&format!("<button class=\"phase {}\" data-action=\"edit-style:{candidate}\" aria-pressed=\"{}\">{candidate}</button>",if candidate==main_style{"active"}else{""},candidate==main_style));
    }
    html.push_str("</nav>");
    if main_style == "Magic" {
        html.push_str(&magic_spell_book(s, set));
        return html;
    }
    if main_style == "Ranged" {
        html.push_str(&ranged_ammunition_card(s, set));
        return html;
    }
    html.push_str("<div class=\"combat-subtypes\" role=\"group\" aria-label=\"Melee subtypes\"><span class=\"combat-subtype-label\">Attack type</span><div class=\"combat-subtype-options\">");
    let selected = if style == "Melee" {
        rule.as_ref()
            .filter(|r| r.family.style == "Melee")
            .and_then(|r| r.family.attack_types.first())
            .map(String::as_str)
            .unwrap_or("")
    } else {
        style
    };
    for kind in ["Stab", "Slash", "Crush"] {
        let active = selected.eq_ignore_ascii_case(kind);
        html.push_str(&format!("<button class=\"phase combat-subtype {}\" data-action=\"edit-style:{kind}\" aria-pressed=\"{active}\">{kind}</button>",if active{"active"}else{""}));
    }
    html.push_str(
        "</div><p class=\"small-note\">Choose an attack type to compare its loadout.</p>",
    );
    html.push_str("</div>");
    html
}
fn task_buttons(s: &State, target: &planner::Monster) -> String {
    let on_task = crate::slayer::on_task(&s.saved.options.combat, target.id);
    let mut html = format!(
        "<div class=\"slayer-target\" role=\"group\" aria-label=\"Slayer task for {}\"><strong>{}</strong><div class=\"combat-subtype-options\">",
        esc(&target.name),
        esc(&target.name)
    );
    for (value, label, active) in [("off", "Off task", !on_task), ("on", "On task", on_task)] {
        html.push_str(&format!("<button class=\"phase {}\" data-action=\"slayer-task:{}:{value}\" aria-pressed=\"{active}\">{label}</button>", if active {"active"} else {""}, target.id));
    }
    html.push_str("</div></div>");
    html
}

fn slayer_tasks(s: &State) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let targets: Vec<_> = encounter_targets(s)
        .into_iter()
        .map(crate::encounters::source_npc_id)
        .filter(|id| seen.insert(*id) && crate::encounters::slayer_eligible(*id))
        .filter_map(planner::monster)
        .collect();
    if targets.is_empty() {
        return String::new();
    }
    let mut html = String::from(
        "<section class=\"slayer-tasks\" aria-label=\"Slayer task settings\"><h3>Slayer task</h3><p class=\"small-note\">Set each assigned target on task. Gear recommendations and combat comparisons include eligible Slayer bonuses.</p>",
    );
    for target in targets {
        html.push_str(&task_buttons(s, target));
    }
    html.push_str("</section>");
    html
}

fn special_attack_panel(
    s: &State,
    member: &account::Member,
    set: &[Option<i32>],
    style: &str,
    target: &planner::Monster,
) -> String {
    if !set
        .get(3)
        .copied()
        .flatten()
        .is_some_and(crate::special_attacks::supported)
    {
        return String::new();
    }
    let settings = &s.saved.options.combat;
    let zorya = set[3]
        .and_then(crate::rules::weapon)
        .and_then(|r| r.weapon.special.as_ref())
        .is_some_and(|r| r.kind == crate::rules::SpecialKind::Zorya);
    let mut html = String::from(
        "<section class=\"special-attacks\" aria-label=\"Special attack burst\"><h3>Special attack burst</h3><p>One activation or already-held Zorya follow-ups. Normal attack DPS and gear ranking stay separate.</p><label>Available special energy<select id=\"special-energy\">",
    );
    let mut energies = vec![0, 25, 50, 75, 100, settings.special_energy];
    energies.sort();
    energies.dedup();
    for value in energies {
        html.push_str(&format!(
            "<option value=\"{value}\" {}>{value}%</option>",
            if settings.special_energy == value {
                "selected"
            } else {
                ""
            }
        ));
    }
    html.push_str("</select></label>");
    if zorya {
        html.push_str("<label>Zorya starting state<select id=\"zorya-followups\">");
        for (value, label) in [
            (0, "Activate special"),
            (1, "1 empowered attack held"),
            (2, "2 empowered attacks held"),
            (3, "3 empowered attacks held"),
        ] {
            html.push_str(&format!(
                "<option value=\"{value}\" {}>{label}</option>",
                if settings.zorya_empowered_hits == value {
                    "selected"
                } else {
                    ""
                }
            ));
        }
        html.push_str("</select></label>");
    }
    match crate::special_attacks::evaluate(
        member,
        set,
        style,
        target,
        s.saved.options.raid_level,
        settings,
    ) {
        Ok(r) => {
            html.push_str(&format!("<dl class=\"combat-values\"><div><dt>Effective burst damage</dt><dd>{:.2}</dd></div><div><dt>Damage before overkill</dt><dd>{:.2}</dd></div><div><dt>Defeat chance</dt><dd>{:.1}%</dd></div><div><dt>Energy spent</dt><dd>{}%</dd></div><div><dt>Expected attacks</dt><dd>{:.2}</dd></div><div><dt>Attack cooldown budget</dt><dd>{:.2}s</dd></div></dl>",r.effective_damage,r.damage,r.kill_chance*100.0,r.energy_spent,r.expected_attacks,r.cooldown_ticks*0.6));
            html.push_str("<table class=\"bonus-table\"><thead><tr><th>Attack</th><th>Hit chance</th><th>Max hit</th><th>Ticks</th></tr></thead><tbody>");
            for (label, attack) in [
                (
                    if zorya {
                        "Opener"
                    } else {
                        "Brutal Swing · any of four rolls"
                    },
                    r.opener.as_ref(),
                ),
                ("Empowered · starting HP", r.empowered.as_ref()),
            ] {
                if let Some(a) = attack {
                    html.push_str(&format!(
                        "<tr><th>{label}</th><td>{:.1}%</td><td>{}</td><td>{}</td></tr>",
                        a.accuracy * 100.0,
                        a.max_hit,
                        a.attack_ticks
                    ));
                }
            }
            html.push_str("</tbody></table>");
            if zorya {
                if let Some(chance) = r.buff_chance {
                    html.push_str(&format!("<p>Opener grants follow-ups: {:.1}% chance. A failed accuracy roll grants none; an accurate hit reduced to zero still qualifies.</p>",chance*100.0));
                }
                html.push_str(&format!("<p>Expected empowered attacks left: {:.2}. The burst stops when this target is defeated; held attacks can be compared against another target using the starting-state control.</p>",r.remaining_empowered_hits));
            }
        }
        Err(reason) => html.push_str(&format!(
            "<p>Special burst unavailable: {}</p>",
            esc(&reason)
        )),
    }
    html.push_str("<p class=\"small-note\">Includes misses, overkill and the finishing cooldown in one target window. No energy recovery, chained activations, movement, projectile travel or phase transitions. Verify charges and the held-buff state in game.</p></section>");
    html
}

fn combat_panel(s: &State, p: &Plan) -> String {
    if s.shared_setup.is_some() {
        return String::new();
    }
    let style = comparison_style(s);
    let set = planned_style(s, p, style);
    let bonus = combat::bonuses(&set);
    let mut html = String::from(
        r#"<section class="panel combat-panel"><div class="panel-heading"><h2>Combat stats</h2></div><div class="combat-body">"#,
    );
    let results_index = html.len();
    let mut headline = String::new();
    html.push_str(&combat_style_controls(s, &set, style));
    html.push_str(r#"<div class="target-choices" role="group" aria-label="Against"><h4>Against</h4><div class="combat-subtype-options">"#);
    let mut priority_ids = std::collections::BTreeSet::new();
    for id in encounter_targets(s)
        .into_iter()
        .filter(|id| priority_ids.insert(*id))
    {
        if let Some(m) = planner::monsters().iter().find(|m| m.id == id) {
            let name = crate::encounters::window(id)
                .filter(|w| {
                    id < 100_000 && w.damage_model != crate::encounters::DamageModel::Standard
                })
                .map(|w| w.phase.as_str())
                .unwrap_or(&m.name);
            html.push_str(&format!(
                r#"<button class="phase {}" data-action="compare-target:{id}" aria-pressed="{}">{}</button>"#,
                if comparison_monster(s).is_some_and(|m| m.id == id) {
                    "active"
                } else {
                    ""
                },
                comparison_monster(s).is_some_and(|m| m.id == id),
                esc(name)
            ));
        }
    }
    html.push_str("</div></div>");
    if let (Some(m), Some(target)) = (s.saved.snapshot.selected(), comparison_monster(s)) {
        if let Some(hp) = s.saved.options.combat.target_hp.get(&target.id) {
            html.push_str(&format!(
                "<p class=\"small-note\">{}: {hp}. {}</p>",
                crate::encounters::window(target.id)
                    .map(|w| w.resource)
                    .unwrap_or_default()
                    .input_label(),
                button("reset-target-hp", "Reset target HP", "text-button")
            ));
        }
        let mut stock = s.saved.snapshot.available(s.saved.options.shared);
        stock.retain(|id, _| !s.saved.options.item_excluded(*id));
        let stock_member = planner::member_with_stock(m, &stock);
        let m = &stock_member;
        html.push_str(&combat_assumptions(s, p, m, target, &set));
        if crate::encounters::slayer_eligible(target.id) {
            html.push_str(&task_buttons(s, target));
        }
        html.push_str(&format!(
            "<p class=\"small-note slayer-bonus\">{}</p>",
            crate::slayer::bonus_note(&s.saved.options.combat, target, &set, style)
        ));
        if style == "Ranged" {
            html.push_str("<div class=\"ranged-stances\" role=\"group\" aria-label=\"Ranged stance\"><h4>Ranged stance</h4><div class=\"combat-subtype-options\">");
            for stance in ["Rapid", "Accurate"] {
                html.push_str(&format!(
                    "<button class=\"phase {}\" data-action=\"ranged-stance:{stance}\" aria-pressed=\"{}\">{stance}</button>",
                    if s.saved.options.combat.ranged_stance == stance { "active" } else { "" },
                    s.saved.options.combat.ranged_stance == stance
                ));
            }
            html.push_str("</div></div>");
            if set[3].is_some_and(|id| crate::rules::ammo_ids(id).contains(&9243)) {
                html.push_str(&format!("<label><input id=\"kandarin-diary\" type=\"checkbox\" {}> Kandarin diary bolt bonus</label>", if s.saved.options.combat.kandarin_diary { "checked" } else { "" }));
            }
        }
        if crate::guidance::profile(&s.saved.options)
            .is_some_and(|profile| profile.recommended_mark_darkness)
        {
            html.push_str(&format!("<label><input id=\"mark-darkness\" type=\"checkbox\" {}> Mark of Darkness active</label>", if s.saved.options.combat.mark_darkness { "checked" } else { "" }));
        }
        let spell = comparison_spell(s, m, &set, target);
        match combat::calculate_configured(
            m,
            &set,
            style,
            target,
            spell,
            s.saved.options.raid_level,
            &s.saved.options.combat,
        ) {
            Ok(r) => {
                headline = format!(
                    r#"<dl class="combat-values"><div><dt>Outgoing DPS</dt><dd>{:.3}</dd></div><div><dt>Hit chance</dt><dd>{:.1}%</dd></div><div><dt>Max hit</dt><dd>{}</dd></div><div><dt>Attack speed</dt><dd>{:.2} ticks</dd></div></dl><p>{} attacks · {:.2} seconds per attack (expected)</p>"#,
                    r.dps,
                    r.accuracy * 100.0,
                    r.max_hit,
                    r.expected_attack_ticks,
                    esc(&r.attack_type),
                    r.expected_attack_ticks * 0.6
                );
                if r.burn.is_some() {
                    headline = headline.replace("Outgoing DPS", "Sustained DPS");
                }
                html.push_str(&attack_window_budget(s, target, &set, &r));
                if r.burn_dps > 0.0 {
                    html.push_str(&format!("<p>Eclipse's long-run burn reference is {:.3} HP/s. The finite-HP estimate above instead starts empty and advances actual burn pulses, stack expiry and target death. Each separate attack window or target starts fresh; use the full-fight scenario for transitions and downtime.</p>", r.burn_dps));
                }
                if r.expected_attack_ticks != f64::from(r.attack_ticks) {
                    html.push_str(&format!("<p>Bloodrager average delay: {:.3} ticks ({:.3}s), including the 33% chance per accurate splat to attack one tick sooner.</p>",r.expected_attack_ticks,r.expected_attack_ticks*0.6));
                }
                if crate::rules::set_effect(&set, "blue_moon") {
                    html.push_str(&format!("<p>Blue Moon free melee: {}. Only accurate binding, Ice and Grasp spells can trigger Frostweaver.</p>", esc(&s.saved.options.combat.blue_moon_melee)));
                }
                html.push_str(&combat_breakdown(s, m, &set, target, &r));
                html.push_str(&special_attack_panel(s, m, &set, style, target));
                if r.potential_healing_per_second > 0.0 {
                    html.push_str(&format!("<p>Guthan's potential healing: {:.3} HP/s before overheal and remaining-target-HP limits. This is not subtracted from damage taken.</p>", r.potential_healing_per_second));
                }
                if crate::rules::set_effect(&set, "dharok") {
                    html.push_str(&format!("<p>Dharok's uses {} current Hitpoints. Change this under Boosts, darts &amp; debuffs.</p>", s.saved.options.combat.current_hitpoints.map(|hp|hp.to_string()).unwrap_or_else(||"full-health".into())));
                }
            }
            Err(e) => {
                headline = format!(
                    "<p class=\"combat-unavailable\">DPS unavailable: {}</p>",
                    esc(&e)
                )
            }
        }
        if crate::yama::has_judge_cycle(&s.saved.options) {
            let judge_set = |style| {
                s.saved
                    .options
                    .targets
                    .iter()
                    .zip(&p.phases)
                    .find(|(target, _)| {
                        crate::yama::is_judge_window(target.id)
                            && crate::rules::base_style(&target.style) == style
                    })
                    .map(|(_, phase)| phase.equipment.as_slice())
            };
            let melee = judge_set("Melee");
            let magic = judge_set("Magic");
            if let (Some(melee), Some(magic)) = (melee, magic) {
                match crate::yama::judge_cycle(m, melee, magic, &s.saved.options, &s.saved.snapshot.available(s.saved.options.shared)) {
                    Ok(cycle) => html.push_str(&format!("<details><summary>Judge cycle · {:.1}s across both interludes</summary><p class=\"small-note\">Magic first, then alternate melee and magic. Maximum hits: magic {}, melee {}. {}</p></details>", cycle.seconds, cycle.magic_max, cycle.melee_max, esc(cycle.note))),
                    Err(reason) => html.push_str(&format!("<p class=\"small-note\">Judge cycle unavailable: {}</p>", esc(&reason))),
                }
            }
        }
        html.push_str(&sustain_panel(s, p, &set, m, target));
    }
    html.push_str(
        "<details><summary>Base equipment bonuses</summary><table class=\"bonus-table\"><tbody>",
    );
    for (label, map, key) in [
        ("Stab attack", &bonus.offensive, "stab"),
        ("Slash attack", &bonus.offensive, "slash"),
        ("Crush attack", &bonus.offensive, "crush"),
        ("Magic attack", &bonus.offensive, "magic"),
        ("Ranged attack", &bonus.offensive, "ranged"),
        ("Strength", &bonus.other, "str"),
        ("Ranged strength", &bonus.other, "ranged_str"),
        ("Prayer bonus", &bonus.other, "prayer"),
        ("Stab defence", &bonus.defensive, "stab"),
        ("Slash defence", &bonus.defensive, "slash"),
        ("Crush defence", &bonus.defensive, "crush"),
        ("Magic defence", &bonus.defensive, "magic"),
        ("Ranged defence", &bonus.defensive, "ranged"),
    ] {
        html.push_str(&format!(
            "<tr><th>{label}</th><td>{:+}</td></tr>",
            map.get(key).copied().unwrap_or(0)
        ));
    }
    html.push_str(&format!(
        "<tr><th>Magic damage</th><td>{:.1}%</td></tr>",
        bonus.other.get("magic_str").copied().unwrap_or(0) as f64 / 10.0
    ));
    html.push_str("</tbody></table></details><details><summary>DPS assumptions</summary><p class=\"small-note\">Selected current-level boosts and stat prayer; no boost decay, special-attack damage, venom or recoil. ToA assumes solo, path level 0, raid levels 0–150. Ruby bolts use selected target HP (phase start by default) and the selected Kandarin diary setting. Outgoing DPS covers uninterrupted attacks while vulnerable. Incoming scenarios are separate and their coverage notes apply. Movement, special-phase uptime, curses and eating are not a full encounter simulation. Unsupported mechanics show no DPS.</p></details></div></section>");
    html.insert_str(results_index, &headline);
    html
}
fn attack_window_budget(
    s: &State,
    target: &planner::Monster,
    set: &[Option<i32>],
    result: &combat::ResultStats,
) -> String {
    let resource = crate::encounters::window(target.id)
        .map(|w| w.resource)
        .unwrap_or_default();
    let (min, max) = combat::target_hp_bounds(target, s.saved.options.raid_level).unwrap_or((1, 1));
    let current = s
        .saved
        .options
        .combat
        .target_hp
        .get(&target.id)
        .map(u32::to_string)
        .unwrap_or_default();
    let mut html = format!(
        "<details id=\"attack-budget\" class=\"attack-budget\" {}><summary>Attack window & supply budget</summary><label>{}<input id=\"target-hp\" type=\"number\" min=\"{min}\" max=\"{max}\" placeholder=\"{max}\" value=\"{current}\"></label><p class=\"small-note\">{}</p>",
        if s.attack_budget_open || result.burn.is_some() {
            "open"
        } else {
            ""
        },
        resource.input_label(),
        if resource == crate::encounters::Resource::Hitpoints {
            "Leave empty for this window's starting HP. Ruby bolts and HP-dependent NPC stats use this HP; the attack budget recalculates them as HP falls."
        } else {
            "Leave empty for this window's starting counter. Shield and totem charge are separate from physical HP; unverified ruby effects against these counters are blocked."
        }
    );
    if let Some(window) = crate::encounters::window(target.id) {
        html.push_str(&format!("<p>{}</p>", esc(&window.description)));
    }
    let budget = if crate::encounters::window(target.id).is_some_and(|w| w.hp_scaling.is_some()) {
        s.saved
            .snapshot
            .selected()
            .ok_or_else(|| "Select an account first.".to_string())
            .and_then(|m| {
                crate::attack_budget::for_dynamic_window(
                    m,
                    set,
                    comparison_style(s),
                    target,
                    comparison_spell(s, m, set, target),
                    &s.saved.options,
                )
            })
    } else {
        crate::attack_budget::for_window(target, result, &s.saved.options, set)
    };
    match budget {
        Ok(budget) if budget.burn.is_some() => {
            let burn=budget.burn.as_ref().unwrap();
            let spread=f64::from(budget.entities);
            html.push_str(&format!("<dl class=\"combat-values\"><div><dt>Estimated attacks</dt><dd>{:.2} ± {:.2}</dd></div><div><dt>Attack cooldown budget</dt><dd>{:.2}s ± {:.2}s</dd></div><div><dt>HP window · each of {} target(s)</dt><dd>{} → {}</dd></div><div><dt>Burn damage · each target</dt><dd>{:.2}</dd></div></dl><p>Time to HP boundary from the first attack: {:.2}s ± {:.2}s per target, excluding projectile travel. No attacks or burn damage are credited after that boundary.</p><p class=\"small-note\">{} seeded trials; ± is an approximate 95% confidence interval for the estimated mean, not an outcome range or safe supply minimum. Five-stack cap, expiry, misses, accurate zeroes and overkill are included. The cooldown budget retains the finishing attack's committed delay. Each entity/window starts with no burns. Transition clearing and downtime are handled separately in the full-fight scenario.</p>",budget.attacks,1.96*burn.attacks_error*spread,budget.seconds,1.96*burn.cooldown_error*0.6*spread,budget.entities,budget.start_hp,budget.stop_hp,burn.burn_damage,burn.death_ticks*0.6,1.96*burn.death_error*0.6,burn.trials));
        },
        Ok(budget) => html.push_str(&format!("<dl class=\"combat-values\"><div><dt>Expected attacks</dt><dd>{:.1}</dd></div><div><dt>Attack cooldown budget</dt><dd>{:.1}s</dd></div><div><dt>{} · each of {} target(s)</dt><dd>{} → {}</dd></div></dl><p class=\"small-note\">Includes misses, overkill and the finishing cooldown. This is an expectation, not a safe supply minimum or kill-time prediction. Compare with your {}-cast / {}-projectile packing budgets; projectile recovery, ruby self-damage, movement, specials, regeneration and eating are excluded.</p>", budget.attacks, budget.seconds, resource.window_label(), budget.entities, budget.start_hp, budget.stop_hp, s.saved.options.combat.spell_casts, s.saved.options.combat.ammo_shots)),
        Err(reason) => html.push_str(&format!("<p class=\"small-note\">{}</p>", esc(&reason))),
    }
    html.push_str("</details>");
    html
}
fn simulation_key(s: &State, p: &Plan) -> String {
    let phases: Vec<_> = p.phases.iter().map(|v| (&v.style, &v.equipment)).collect();
    serde_json::to_string(&(
        &s.saved.snapshot,
        &s.saved.options,
        &p.equipment,
        &p.inventory,
        phases,
        &p.loaded_ammo,
    ))
    .unwrap_or_default()
}
fn simulation_panel(s: &State, p: &Plan) -> String {
    use crate::simulation::Encounter;
    let Some(encounter) = Encounter::selected(&s.saved.options) else {
        return String::new();
    };
    let mut html = format!(
        "<section class=\"panel fight-simulation-panel\"><details id=\"fight-simulation\" {}><summary>Full-fight & consumption simulation · {}</summary><p>Seeded encounter scenarios using the gear actually packed. Each run carries HP, Prayer, attack cooldowns and supplies through every phase. A trip reuses the remaining supplies and HP; it does not bank or receive drops.</p><p class=\"notice\">Execution assumptions: incoming damage, timing and mistakes are editable. Defaults illustrate a scenario; they are not measured performance or a guarantee of survival.</p>",
        if s.simulation_open { "open" } else { "" },
        encounter.name()
    );
    html.push_str("<div class=\"simulation-inputs\">");
    for (key, label, value, min, max) in s.saved.options.simulation.fields() {
        let relevant = if ["acid_ticks", "freeze_ticks", "acid_uptime"].contains(&key) {
            encounter == Encounter::Vorkath
        } else if [
            "judge_travel",
            "flare_interval",
            "flare_charge",
            "flare_heal",
            "flare_regen",
            "glyph_hp",
            "glyph_prayer",
            "melee_autos",
        ]
        .contains(&key)
        {
            encounter == Encounter::Yama
        } else if ["warden", "prayer_rotation", "skull_ticks", "skull_damage"].contains(&key) {
            encounter == Encounter::Wardens
        } else {
            true
        };
        if relevant && key == "burn_phase_resets" {
            html.push_str("<div role=\"group\" aria-label=\"Burn stacks at unverified phase changes\"><span>Burn stacks at unverified phase changes</span><div class=\"combat-subtype-options\">");
            for (n, label) in [(1, "Clear stacks"), (0, "Keep aging stacks")] {
                html.push_str(&format!("<button class=\"phase {}\" data-action=\"simulation-burn-reset:{n}\" aria-pressed=\"{}\">{label}</button>",if value==n {"active"}else{""},value==n));
            }
            html.push_str("</div><p class=\"small-note\">");
            if encounter == Encounter::Vorkath {
                html.push_str("Freeze/spawn only; clearing remains an assumption. Retained burns expire during immunity without dealing damage.");
            } else {
                html.push_str("Scenario assumption: confirm phase clearing for this encounter. Retained burns expire during invulnerable downtime without damaging the target.");
            }
            html.push_str("</p></div>");
        } else if relevant && key == "warden" {
            html.push_str("<div role=\"group\" aria-label=\"P2 Warden\"><span>Fight in P2</span><div class=\"combat-subtype-options\">");
            for (n, label) in [(0, "Elidinis"), (1, "Tumeken")] {
                html.push_str(&format!("<button class=\"phase {}\" data-action=\"simulation-warden:{n}\" aria-pressed=\"{}\">{label}</button>",if value==n {"active"}else{""},value==n));
            }
            html.push_str("</div></div>");
        } else if relevant {
            html.push_str(&format!("<label>{}<input id=\"sim-{key}\" type=\"number\" min=\"{min}\" max=\"{max}\" value=\"{value}\"></label>",esc(label)));
        }
    }
    html.push_str("</div>");
    if encounter == Encounter::Vorkath {
        html.push_str("<p class=\"small-note\">Vorkath acid entry/exit retain Eclipse burns regardless of the freeze/spawn control, following inspected 2024 gameplay. Current-patch confirmation is still needed.</p>");
    }
    html.push_str("<p class=\"small-note\">A tick is 0.6 seconds. Residual incoming HP/min is after protection and excludes failed mechanics added separately. Recovery and loaded charges are assumptions you must verify. Wardens uses supplies at room entry, excludes the earlier raid rooms and mechanic-changing invocations, and supports solo raid levels 0–150.</p>");
    html.push_str(&button(
        "simulate-fight",
        "Run fight & trip simulation",
        "button gold",
    ));
    html.push_str(&button("reset-simulation", "Reset scenario", "text-button"));
    if let Some((key, result)) = &s.simulation_result {
        if *key != simulation_key(s, p) {
            html.push_str("<p role=\"status\">Setup or assumptions changed. Run again for current results.</p>");
        } else {
            match result {
            Err(reason)=>html.push_str(&format!("<p class=\"combat-unavailable\" role=\"status\">Simulation unavailable: {}</p>",esc(reason))),
            Ok(report)=>{
                let total=report.runs.len();let complete=report.completed();
                html.push_str(&format!("<div aria-live=\"polite\"><h3>Scenario results</h3><p>{complete} / {total} first fights completed in the model; {} / {total} completed with the packed supply budget. Failures and timeouts are retained; percentiles below include completed first fights only.</p>",report.covered()));
                let mut failures=BTreeMap::new();for run in report.runs.iter().filter(|r|!r.completed){*failures.entry(run.reason.as_str()).or_insert(0usize)+=1;}
                if !failures.is_empty(){html.push_str("<p>Unfinished first fights: ");for (reason,n) in failures {html.push_str(&format!("{}: {n}. ",esc(reason)));}html.push_str("</p>");}
                html.push_str("<table class=\"wiki-table\"><thead><tr><th>First-fight demand</th><th>Median</th><th>90th percentile</th></tr></thead><tbody>");
                let rows:[(&str,fn(&crate::simulation::Usage)->f64);19]=[
                    ("Burn damage (HP)",|u|u.burn_damage as f64),("Burn stacks applied",|u|u.burns_applied as f64),
                    ("Burn procs discarded at cap",|u|u.burns_discarded as f64),("Burn stacks cleared",|u|u.burns_cleared as f64),
                    ("Duration (seconds)",|u|u.ticks as f64*0.6),("Attacks",|u|u.attacks as f64),
                    ("Rune casts",|u|u.casts as f64),("Projectiles fired",|u|u.shots as f64),
                    ("Weapon charges",|u|u.charges.values().sum::<u32>() as f64),("Blowpipe scales",|u|u.scales as f64),
                    ("Food bites",|u|u.food_bites as f64),("Extra shark-equivalent food",|u|u.extra_food as f64),
                    ("Prayer points drained",|u|u.prayer_points),("Prayer doses",|u|u.prayer_doses as f64),
                    ("Extra prayer-potion doses",|u|u.extra_doses as f64),("Ruby self-damage (HP)",|u|u.ruby_damage),
                    ("Crumble Undead casts",|u|u.crumble as f64),("Mark of Darkness casts",|u|u.mark as f64),
                    ("Boss healing (HP)",|u|u.healing as f64),
                ];
                for (label,metric) in rows {let a=report.quantile(metric,0.5);let b=report.quantile(metric,0.9);
                    html.push_str(&format!("<tr><th>{label}</th><td>{}</td><td>{}</td></tr>",a.map(|v|format!("{v:.1}")).unwrap_or_else(||"—".into()),b.map(|v|format!("{v:.1}")).unwrap_or_else(||"—".into())));}
                html.push_str("</tbody></table><h4>Carried rune and projectile demand</h4><table class=\"wiki-table\"><thead><tr><th>Item</th><th>90th percentile</th><th>Carried</th></tr></thead><tbody>");
                let ids:std::collections::BTreeSet<_>=report.runs.iter().flat_map(|r|r.usage.resources.keys().copied()).collect();
                let model=crate::simulation::Model::from_plan(&s.saved.snapshot,&s.saved.options,p).ok();
                for id in ids {let q=report.quantile(|u|u.resources.get(&id).copied().unwrap_or(0) as f64,0.9);
                    html.push_str(&format!("<tr><th>{}</th><td>{}</td><td>{}</td></tr>",esc(&planner::item_name(id)),q.map(|n|format!("{n:.0}")).unwrap_or_else(||"—".into()),model.as_ref().and_then(|m|m.available.get(&id)).copied().unwrap_or(0)));}
                html.push_str("</tbody></table><h4>Phase time (median seconds)</h4><table class=\"wiki-table\"><tbody>");
                let phases:std::collections::BTreeSet<_>=report.runs.iter().flat_map(|r|r.usage.phase_ticks.keys().cloned()).collect();
                for phase in phases {let q=report.quantile(|u|u.phase_ticks.get(&phase).copied().unwrap_or(0) as f64*0.6,0.5);html.push_str(&format!("<tr><th>{}</th><td>{}</td></tr>",esc(&phase),q.map(|n|format!("{n:.1}")).unwrap_or_else(||"—".into())));}
                html.push_str("</tbody></table><h4>Mechanics observed (median, all first attempts)</h4><table class=\"wiki-table\"><tbody>");
                let mechanics:[(&str,fn(&crate::simulation::Usage)->u32);8]=[("Acid specials",|u|u.acid),("Freeze specials",|u|u.freeze),("Judges",|u|u.judges),("Flares spawned",|u|u.flares),("Flares exploded",|u|u.exploded),("Core exposures",|u|u.core_downs),("Skull sets",|u|u.skulls),("Enrage transitions",|u|u.enrage)];
                for (label,metric) in mechanics {let mut values=report.runs.iter().map(|r|metric(&r.usage)).collect::<Vec<_>>();values.sort();if let Some(n)=values.get(values.len()/2){html.push_str(&format!("<tr><th>{label}</th><td>{n}</td></tr>"));}}
                html.push_str("</tbody></table><h4>Fights covered per trip</h4><p>");
                let mut counts=BTreeMap::new();for run in &report.runs {*counts.entry(run.trip_kills).or_insert(0usize)+=1;}
                for (kills,n) in counts {html.push_str(&format!("{kills} fight(s): {n} / {total} trials. "));}
                html.push_str("The configured trip cap is a censoring limit, not proof that another fight is impossible. Additional food uses 20-HP shark equivalents; extra prayer doses use ordinary prayer-potion restoration. These extras are demand forecasts and are never added to your bag.</p><details><summary>Model assumptions & exclusions</summary><ul>");
                for note in &report.notes {html.push_str(&format!("<li>{}</li>",esc(note)));}
                html.push_str("</ul></details></div>");html.push_str(&button("download-simulation","Download scenario results","text-button"));
                html.push_str(&button("download-gameplay-worksheet","Download gameplay worksheet","text-button"));
            }
        }
        }
    }
    html.push_str(&format!("<details id=\"gameplay-calibration\" class=\"gameplay-calibration\" {}><summary>Compare with recorded gameplay</summary><p>No matched gameplay recordings are checked in for this encounter. Public mechanics references and synthetic tests do not validate fight duration, consumption or survival.</p><p>Run a scenario, download its gameplay worksheet, and annotate recordings with the same stats, equipment, supplies and solo rules. Keep later attempts separate as holdouts. Unknown quantities can stay blank.</p><p><a class=\"text-button\" href=\"https://github.com/nathansbradshaw/ironbound/blob/main/docs/gameplay-calibration.md\" target=\"_blank\" rel=\"noopener\">Recording guide and worksheet format ↗</a></p><label>Recorded gameplay worksheet<input id=\"calibration-file\" type=\"file\" accept=\".json,application/json\"></label><p class=\"small-note\">Read locally for this session. This does not upload recordings, save evidence in your account, or change scenario defaults. Download the comparison to keep it.</p>",if s.calibration_open {"open"}else{""}));
    html.push_str("<ul>");
    for note in crate::calibration::capture_notes(encounter) {
        html.push_str(&format!("<li>{}</li>", esc(note)));
    }
    html.push_str("</ul>");
    if let Some((key, result)) = &s.calibration_result {
        if *key != simulation_key(s, p) {
            html.push_str("<p role=\"status\">Setup or assumptions changed. Restore the worksheet setup and compare again.</p>");
        } else {
            match result {
                Err(reason) => html.push_str(&format!(
                    "<p role=\"status\">Gameplay comparison unavailable: {}</p>",
                    esc(reason)
                )),
                Ok(report) => {
                    html.push_str(&format!("<div aria-live=\"polite\"><h3>Gameplay comparison</h3><p>{}. {} accepted attempts; {} skipped.</p>",esc(&report.status),report.accepted,report.skipped.len()));
                    for (id, reason) in &report.skipped {
                        html.push_str(&format!("<p>{}: {}</p>", esc(id), esc(reason)));
                    }
                    for row in &report.completion {
                        let uncertainty = row
                            .wilson_95
                            .map(|(lo, hi)| {
                                format!(
                                    " Observed completion 95% interval: {:.1}–{:.1}%.",
                                    lo * 100.0,
                                    hi * 100.0
                                )
                            })
                            .unwrap_or_default();
                        html.push_str(&format!("<p>{:?}: {} / {} completed; {} deaths, {} teleports, {} timeouts. Model completion: {:.1}%.{uncertainty}</p>",row.split,row.completed,row.attempts,row.deaths,row.teleports,row.timeouts,row.predicted_rate*100.0));
                    }
                    if !report.summaries.is_empty() {
                        html.push_str("<table class=\"wiki-table\"><thead><tr><th>Metric / split</th><th>Attempts</th><th>Mean bias / absolute error</th><th>10–90% interval coverage</th></tr></thead><tbody>");
                        for row in &report.summaries {
                            html.push_str(&format!("<tr><th>{} · {:?}</th><td>{}</td><td>{:.1} / {:.1}</td><td>{:.0}%</td></tr>",esc(&crate::calibration::metric_label(&row.metric)),row.split,row.samples,row.mean_error,row.mean_absolute_error,row.interval_coverage*100.0));
                        }
                        html.push_str("</tbody></table><p class=\"small-note\">Positive bias means the model predicts more than observed. Timing uses ticks. Consumption uses the worksheet's named units. Only completed attempts enter this table.</p>");
                    }
                    if !report.proposals.is_empty() {
                        html.push_str("<h4>Training-only suggestions</h4><p>These use directly measured controls. Check the recording, then use new holdout attempts after changing settings.</p><ul>");
                        for row in &report.proposals {
                            let fields = s.saved.options.simulation.fields();
                            let label = fields
                                .iter()
                                .find(|f| f.0 == row.setting)
                                .map(|f| f.1)
                                .unwrap_or(&row.setting);
                            html.push_str(&format!(
                                "<li>{}: {:.1} (current {}; {} run(s), {} measurements; {})</li>",
                                esc(label),
                                row.proposed,
                                row.current,
                                row.training_runs,
                                row.measurements,
                                if row.within_bounds {
                                    "within allowed bounds"
                                } else {
                                    "outside allowed bounds; investigate model mismatch"
                                }
                            ));
                        }
                        html.push_str("</ul>");
                    }
                    html.push_str(&format!(
                        "<p class=\"small-note\">{}</p></div>",
                        esc(crate::calibration::LIMITATION)
                    ));
                    html.push_str(&button(
                        "download-calibration",
                        "Download gameplay comparison",
                        "text-button",
                    ));
                }
            }
        }
    }
    html.push_str("</details></details></section>");
    html
}
fn tier_table(s: &State) -> String {
    let style = comparison_style(s);
    let plan = s.plan();
    let set = planned_style(s, &plan, style);
    let Some(member) = s.saved.snapshot.selected() else {
        return String::new();
    };
    let target = comparison_monster(s);
    let mut html = format!(
        r#"<section class="owned-tiers"><h2>Your bank: equipment tiers</h2><p>{style} · click an owned item to use it. Rows compare one slot at a time with the other equipment held fixed.</p><p>When DPS is supported, choices are sorted by calculated DPS against the selected monster. Otherwise the reviewed starter order is shown. This is not a global best-in-slot search.</p><table class="wiki-table"><thead><tr><th>Slot</th><th>Owned choices, best supported DPS first</th></tr></thead><tbody>"#
    );
    for slot in [0, 1, 2, 3, 4, 5, 7, 9, 10, 12, 13] {
        let mut choices = planner::owned_tiers(&s.saved.snapshot, &s.saved.options, style, slot)
            .into_iter()
            .map(|g| {
                let mut trial = set.clone();
                trial[slot] = Some(g.id);
                if slot == 3
                    && planner::equipment()
                        .iter()
                        .any(|e| e.id == g.id && e.two_handed)
                {
                    trial[5] = None;
                }
                let dps = if g.usable && !g.excluded {
                    target
                        .and_then(|m| {
                            combat::calculate_configured(
                                member,
                                &trial,
                                style,
                                m,
                                comparison_spell(s, member, &trial, m),
                                s.saved.options.raid_level,
                                &s.saved.options.combat,
                            )
                            .ok()
                        })
                        .map(|r| r.dps)
                } else {
                    None
                };
                (g, dps)
            })
            .collect::<Vec<_>>();
        choices.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        html.push_str(&format!("<tr><th>{}</th><td>", planner::SLOTS[slot]));
        if choices.is_empty() {
            html.push_str("<span class=\"tier-empty\">No owned equipment for this slot</span>");
        }
        let more = choices.len().saturating_sub(6);
        for (index, (g, dps)) in choices.into_iter().enumerate() {
            if index == 6 {
                html.push_str(&format!(
                    "<details><summary>{more} more owned choices</summary>"
                ));
            }
            let disabled = if g.usable && !g.excluded {
                ""
            } else {
                "disabled"
            };
            html.push_str(&format!(r#"<button class="tier-item {}" data-action="pick-gear:{style}:{slot}:{}" {disabled}>{}<span>{}<small>{}</small></span></button>"#,if set[slot]==Some(g.id){"tier-selected"}else{""},g.id,image(g.id),esc(&planner::item_name(g.id)),dps.map(|d|format!("{d:.3} DPS")).unwrap_or_else(||if g.excluded{"Excluded".into()}else if !g.usable{"Skill requirements unconfirmed / unmet".into()}else{"DPS not supported".into()})));
        }
        if more > 0 {
            html.push_str("</details>");
        }
        html.push_str("</td></tr>");
    }
    html.push_str("</tbody></table><p>Armour and accessories use the full owned-equipment catalog. Weapons use reviewed attack behaviours. Your bank itself is never included in a share link.</p></section>");
    html
}

fn planned_style(s: &State, p: &Plan, style: &str) -> Vec<Option<i32>> {
    if style == planner::primary_style(&s.saved.options) {
        return p.equipment.clone();
    }
    p.phases
        .iter()
        .find(|phase| {
            phase.style == style
                || (style == "Magic" && s.saved.options.boss == Boss::Toa && phase.name == "Akkha")
        })
        .map(|phase| phase.equipment.clone())
        .unwrap_or_else(|| planner::equipment_for_style(&s.saved.snapshot, &s.saved.options, style))
}
fn switch_controls(s: &State, p: &Plan) -> String {
    if s.shared_setup.is_some() {
        return String::new();
    }
    let mut html = format!(
        r#"<section class="flow-controls" aria-label="Switching preferences"><label>Switch limit<input id="max-switches" type="number" min="0" max="11" step="1" value="{}" placeholder="Any" title="Maximum changed slots per switch; leave empty for no limit" aria-describedby="switch-usage"></label><details id="switch-preferences" {}><summary>Gear options</summary><div class="switch-preferences-fields"><label>Gear slots in bag<input id="switch-slots" type="number" min="0" max="20" value="{}"></label><label>Prioritize<select id="focus-style">"#,
        s.saved
            .options
            .max_switches
            .map(|v| v.to_string())
            .unwrap_or_default(),
        if s.switch_preferences_open {
            "open"
        } else {
            ""
        },
        s.saved.options.switch_slots
    );
    for (value, label) in [
        ("Balanced", "Balance the encounter"),
        ("Magic", "Keep my full magic set"),
        ("Ranged", "Keep my full ranged set"),
        ("Melee", "Keep my full melee set"),
        ("Stab", "Keep my full stab set"),
        ("Slash", "Keep my full slash set"),
        ("Crush", "Keep my full crush set"),
    ] {
        html.push_str(&format!(
            r#"<option value="{value}" {}>{label}</option>"#,
            if s.saved.options.focus_style == value {
                "selected"
            } else {
                ""
            }
        ));
    }
    html.push_str("</select></label></div>");
    html.push_str("<details><summary>Advanced · custom weights</summary>");
    html.push_str(&ranking_controls(s, "plan-rank"));
    html.push_str("</details>");
    let usage = if let Some(report) = &p.optimization {
        if let Some(limit) = s.saved.options.max_switches {
            format!(
                "Uses {} of {limit} allowed {} per switch.",
                report.max_changes,
                if limit == 1 { "change" } else { "changes" }
            )
        } else {
            format!(
                "Uses {} {} per switch.",
                report.max_changes,
                if report.max_changes == 1 {
                    "change"
                } else {
                    "changes"
                }
            )
        }
    } else if s.saved.options.max_switches.is_some() {
        "No setup verified within this limit. Review Preparation.".into()
    } else {
        "Maximum changed slots per switch.".into()
    };
    let usage_html =
        format!("<p id=\"switch-usage\" class=\"switch-usage\" role=\"status\">{usage}</p>");
    html.push_str(&slayer_tasks(s));
    let recommended = crate::recommendation::enabled(&s.saved.options);
    html.push_str(&format!("<label class=\"void-preference\"><input id=\"allow-void\" type=\"checkbox\" {}> Use Void armour <small>Normal and Elite Void · turn off to compare your other gear.</small></label>", if s.saved.options.exclude_void { "" } else { "checked" }));
    html.push_str(&format!(r#"<div class="recommendation-policy"><label>Gear approach<select id="recommendation-mode"><option value="recommended" {}>Recommended</option><option value="custom" {}>Custom weights</option></select></label><details><summary>Why this gear?</summary><p>{}</p>"#,if recommended{"selected"}else{""},if recommended{""}else{"selected"},esc(crate::recommendation::summary(&s.saved.options))));
    if recommended {
        if let Some(profile) = crate::guidance::profile(&s.saved.options) {
            for reason in &profile.rationale {
                html.push_str(&format!("<p>{}</p>", esc(reason)));
            }
            for url in &profile.source_urls {
                html.push_str(&format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">Wiki guide ↗</a>",
                    esc(url)
                ));
            }
            if s.saved.options.boss == Boss::Custom && s.saved.options.targets.len() == 1 {
                html.push_str(&button(
                    "compare-owned-styles",
                    "Compare owned styles",
                    "phase",
                ));
            }
            html.push_str("<p>Supported attack windows only. Movement, specials and full fight survival remain partial.</p>");
        } else {
            html.push_str("<p>Uses supported combat calculations and reviewed encounter roles where available.</p>");
        }
    }
    html.push_str("</details></div></details>");
    html.push_str(&usage_html);
    html.push_str("</section>");
    html
}

fn efficient_panel(s: &State, _plan: &Plan) -> String {
    if !s.compact_open {
        return "<details id=\"compact-setup\" class=\"panel efficient-panel\" aria-label=\"Compact setup recommendation\"><summary><h3>Save bag space</h3></summary></details>".into();
    }
    ensure_efficient(s);
    let limits = crate::efficient::Tolerances::from_options(&s.saved.options);
    let mut html = format!(
        "<details id=\"compact-setup\" class=\"panel efficient-panel\" aria-label=\"Compact setup recommendation\" {}><summary><h3>Save bag space</h3></summary><div class=\"efficient-body\">",
        if s.compact_open { "open" } else { "" }
    );
    html.push_str(&format!("<details id=\"packing-limits\" class=\"packing-limits\" {}><summary>Allowances <span>{}% DPS · {}% damage · {}% prayer</span></summary><div class=\"tradeoff-limits\">",if s.packing_limits_open {"open"}else{""},limits.dps_loss,limits.damage_taken,limits.prayer_drain));
    for (id, label, value) in [
        ("switch-tolerance", "DPS loss", limits.dps_loss),
        (
            "damage-tolerance",
            "Extra damage taken",
            limits.damage_taken,
        ),
        (
            "prayer-tolerance",
            "Extra prayer drain",
            limits.prayer_drain,
        ),
    ] {
        html.push_str(&format!("<label class=\"tolerance-control\">{label}<span><input id=\"{id}\" type=\"number\" min=\"0\" max=\"{}\" step=\"1\" inputmode=\"numeric\" value=\"{value}\" aria-describedby=\"tolerance-help\">%</span></label>",crate::efficient::MAX_TOLERANCE_PERCENT));
    }
    html.push_str("</div><p id=\"tolerance-help\" class=\"small-note\">Effective encounter DPS. Prayer drain is points per minute; longer fights can use more prayer overall.</p></details>");
    let Some(result) = efficient_result(s) else {
        html.push_str("<p role=\"status\">Checking switch limits…</p></div></details>");
        return html;
    };
    let Some((selected, best)) = result.compact.zip(result.best) else {
        html.push_str(&format!(
            "<p>{}</p></div></details>",
            esc(result
                .reason
                .as_deref()
                .unwrap_or("No comparison available."))
        ));
        return html;
    };
    let chosen = &result.candidates[selected];
    let benchmark = &result.candidates[best];
    let a = chosen.totals.as_ref().unwrap();
    let b = benchmark.totals.as_ref().unwrap();
    let loss = (1.0 - b.seconds / a.seconds) * 100.0;
    let dps_change = if loss.abs() < 0.05 {
        "Same DPS".into()
    } else {
        format!("{loss:.1}% less DPS")
    };
    let saving =
        benchmark.budget.max_inventory_space as i32 - chosen.budget.max_inventory_space as i32;
    let prayer = a.prayer_points - b.prayer_points;
    let prayer_cost = if prayer.abs() < 0.05 {
        "same total prayer use".into()
    } else {
        format!(
            "{:.1} {} prayer points total",
            prayer.abs(),
            if prayer > 0.0 { "more" } else { "fewer" }
        )
    };
    html.push_str(&format!("<p class=\"efficient-choice\">{} switch item{} · up to {} change{}</p><p>{dps_change}; {prayer_cost}.</p><p class=\"small-note\">Peak bag {}/28 · {}{} compared with fastest.</p>",chosen.budget.carried_gear,if chosen.budget.carried_gear==1 {""}else{"s"},chosen.budget.max_pair,if chosen.budget.max_pair==1 {""}else{"s"},chosen.budget.max_inventory_space,saving.abs(),if saving>=0 {" fewer slots"}else{" more slots"}));
    html.push_str(&format!(
        "<p class=\"small-note\">Prayer drain: {:.2}/min ({:+.2} vs fastest).</p>",
        a.prayer_per_minute,
        a.prayer_per_minute - b.prayer_per_minute
    ));
    if !chosen.omissions.is_empty() {
        html.push_str(&format!(
            "<p class=\"efficient-unknown\">Partial recommendation: {}</p>",
            esc(&chosen.omissions[0])
        ));
    }
    if a.incoming_damage.is_none() || b.incoming_damage.is_none() {
        html.push_str("<p class=\"efficient-unknown\">Incoming damage remains unverified.</p>");
    }
    let edited_inventory = !s.saved.options.inventory_overrides.is_empty()
        || !s.saved.options.inventory_quantities.is_empty();
    if edited_inventory {
        html.push_str("<p class=\"small-note\">This suggestion uses automatic packing. Applying replaces your manual inventory slots.</p>");
    }
    let using = s.saved.options.max_switches == Some(chosen.budget.cap)
        && s.saved.options.inventory_overrides.is_empty()
        && s.saved.options.inventory_quantities.is_empty();
    html.push_str(&format!("<div class=\"efficient-actions\"><button class=\"phase\" data-action=\"efficient-apply\" {}>{}</button>{}</div>",if using {"disabled"}else{""},if using {"Using compact setup"}else if edited_inventory {"Repack & use compact setup"}else{"Use compact setup"},button("efficient-compare","Compare with fastest","text-button")));
    html.push_str(&format!("<details><summary>How we choose</summary><p>Limits: {limits}. DPS loss uses effective encounter DPS, not a percentage increase in attack time. Damage checks include known incoming rates and total exposure where modelled. Prayer checks use drain rates, not total points consumed.</p><p>Tests caps 0–11 with the same bank, gear pins, combat settings and supply counts. Keeps identical workload coverage and your switch ceiling. Prioritizes peak bag space, then carried gear and actual changes. Applying restores automatic inventory packing. This bounded search does not prove a global BIS.</p><p>Fastest tested: {}-change cap · {} gear pieces · {:.1}s · {:.1} prayer points. Suggested cap: {}.</p></details></div></details>",benchmark.budget.cap,benchmark.budget.carried_gear,b.seconds,b.prayer_points,chosen.budget.cap));
    html
}

fn signed_defence(delta: &BTreeMap<String, i32>) -> String {
    let label = |key: &str| match key {
        "stab" => "stab",
        "slash" => "slash",
        "crush" => "crush",
        "magic" => "magic",
        "ranged" => "ranged",
        _ => "defence",
    };
    let mut values: Vec<_> = delta.iter().filter(|(_, value)| **value != 0).collect();
    values.sort_by_key(|(_, value)| std::cmp::Reverse(value.abs()));
    let positive = values.iter().copied().find(|(_, value)| **value > 0);
    let negative = values.iter().copied().find(|(_, value)| **value < 0);
    let chosen = if let (Some(positive), Some(negative)) = (positive, negative) {
        vec![positive, negative]
    } else {
        values.into_iter().take(2).collect()
    };
    chosen
        .into_iter()
        .map(|(key, value)| format!("{value:+} {} def", label(key)))
        .collect::<Vec<_>>()
        .join(" · ")
}
fn budget_stat(
    label: &str,
    a: Option<f64>,
    b: Option<f64>,
    digits: usize,
    lower: bool,
    percent: bool,
) -> String {
    let number = |value: Option<f64>| {
        value
            .map(|v| format!("{v:.digits$}"))
            .unwrap_or_else(|| "—".into())
    };
    let (delta, class) = match a.zip(b) {
        Some((a, b)) if (b - a).abs() < 0.5 * 10_f64.powi(-(digits as i32)) => {
            ("No change".into(), "same")
        }
        Some((a, b)) => {
            let difference = b - a;
            let mut text = format!("{difference:+.digits$}");
            if percent && a.abs() > 1e-9 {
                text.push_str(&format!(" ({:+.2}%)", difference / a * 100.0));
            }
            (
                text,
                if (difference < 0.0) == lower {
                    "gain"
                } else {
                    "loss"
                },
            )
        }
        None => ("Unverified".into(), "unknown"),
    };
    format!(
        "<div class=\"budget-stat\"><span>{label}</span><strong>{} <span aria-hidden=\"true\">→</span> {}</strong><small class=\"{class}\">{delta}</small></div>",
        number(a),
        number(b)
    )
}
fn encounter_overview(comparison: &crate::tradeoffs::Comparison) -> String {
    let summary = crate::encounter_summary::compare(comparison);
    let partial = !summary.complete();
    let mut html = format!(
        "<section class=\"encounter-overview\" aria-label=\"Overall encounter comparison\"><h3>Overall encounter{}</h3><p class=\"small-note\">Attack-time estimate · {} of {} windows{}</p>",
        if partial { " · partial" } else { "" },
        summary.covered_windows,
        summary.total_windows,
        if partial {
            " · missing coverage below"
        } else {
            ""
        }
    );
    let a = summary.left.as_ref();
    let b = summary.right.as_ref();
    if let Some((a, b)) = a.zip(b) {
        let seconds = b.seconds - a.seconds;
        let prayer = b.prayer_points - a.prayer_points;
        let speed = if seconds.abs() < 0.05 {
            "same attack time".into()
        } else {
            format!(
                "{:.1}s {} ({:.1}%)",
                seconds.abs(),
                if seconds < 0.0 { "faster" } else { "slower" },
                seconds.abs() / a.seconds * 100.0
            )
        };
        let cost = if prayer.abs() < 0.05 {
            "same prayer use".into()
        } else {
            format!(
                "{:.1} {} prayer points",
                prayer.abs(),
                if prayer < 0.0 { "fewer" } else { "more" }
            )
        };
        html.push_str(&format!(
            "<p class=\"encounter-verdict\">At {}: {speed}; {cost}{}.</p>",
            comparison.right.cap,
            if partial {
                " across covered windows"
            } else {
                ""
            }
        ));
    }
    html.push_str(&format!(
        "<div class=\"budget-metrics\">{}{}{}{}</div>",
        budget_stat(
            "Effective DPS",
            a.map(|v| v.effective_dps),
            b.map(|v| v.effective_dps),
            3,
            false,
            true
        ),
        budget_stat(
            "Attack time · seconds",
            a.map(|v| v.seconds),
            b.map(|v| v.seconds),
            1,
            true,
            true
        ),
        budget_stat(
            "Prayer used · points",
            a.map(|v| v.prayer_points),
            b.map(|v| v.prayer_points),
            1,
            true,
            false
        ),
        budget_stat(
            "Prayer / minute",
            a.map(|v| v.prayer_per_minute),
            b.map(|v| v.prayer_per_minute),
            2,
            true,
            false
        )
    ));
    let range = |v: Option<&crate::encounter_summary::Totals>| {
        v.and_then(|v| v.incoming_damage)
            .map(|(min, max)| {
                if (max - min).abs() < 0.05 {
                    format!("{min:.1}")
                } else {
                    format!("{min:.1}–{max:.1}")
                }
            })
            .unwrap_or_else(|| "—".into())
    };
    let incoming_known = a
        .and_then(|v| v.incoming_damage)
        .zip(b.and_then(|v| v.incoming_damage))
        .is_some();
    html.push_str(&format!("<div class=\"budget-stat encounter-incoming\"><span>Normal damage over attack time</span><strong>{} → {}</strong><small>{}</small></div>",range(a),range(b),if incoming_known {"Scenario bounds, not a predicted attack mix. Specials excluded."} else {"Unverified: at least one incoming attack scenario is unknown. Specials excluded."}));
    for note in summary
        .assumptions
        .iter()
        .filter(|n| n.contains("one declared probe workload"))
    {
        html.push_str(&format!("<p class=\"small-note\">{}</p>", esc(note)));
    }
    html.push_str("<p class=\"small-note\">Combat time only. Movement, downtime and specials are excluded.</p>");
    if partial {
        html.push_str(
            "<details class=\"encounter-omissions\"><summary>Missing coverage</summary><ul>",
        );
        for note in &summary.omissions {
            html.push_str(&format!("<li>{}</li>", esc(note)));
        }
        html.push_str("</ul></details>");
    }
    html.push_str("<details><summary>Overall defence bonuses</summary><p class=\"small-note\">Equipment bonuses weighted by time in each set. These are not damage-reduction percentages.</p><table class=\"bonus-table\"><thead><tr><th>Type</th>");
    html.push_str(&format!(
        "<th>{}</th><th>{}</th><th>Change</th></tr></thead><tbody>",
        comparison.left.cap, comparison.right.cap
    ));
    for kind in ["stab", "slash", "crush", "magic", "ranged"] {
        let av = a.and_then(|v| v.defence.get(kind)).copied();
        let bv = b.and_then(|v| v.defence.get(kind)).copied();
        let number = |v: Option<f64>| v.map(|v| format!("{v:+.1}")).unwrap_or_else(|| "—".into());
        html.push_str(&format!(
            "<tr><th>{kind}</th><td>{}</td><td>{}</td><td>{}</td></tr>",
            number(av),
            number(bv),
            number(av.zip(bv).map(|(a, b)| b - a))
        ));
    }
    html.push_str(
        "</tbody></table></details><details><summary>How the total is calculated</summary><ul>",
    );
    for note in &summary.assumptions {
        html.push_str(&format!("<li>{}</li>", esc(note)));
    }
    html.push_str("</ul><div class=\"comparison-scroll\"><table class=\"bonus-table\"><thead><tr><th>Window</th><th>HP</th><th>Seconds ");
    html.push_str(&format!(
        "{} → {}</th></tr></thead><tbody>",
        comparison.left.cap, comparison.right.cap
    ));
    for row in &summary.workloads {
        html.push_str(&format!(
            "<tr><th>{}</th><td>{:.1}</td><td>{:.1} → {:.1}</td></tr>",
            esc(&row.name),
            row.hp,
            row.left_seconds,
            row.right_seconds
        ));
    }
    html.push_str("</tbody></table></div></details></section>");
    html
}

fn budget_comparison(s: &State) -> String {
    let [left_cap, right_cap] = s.saved.budget_caps;
    let mut html = format!(
        "<details id=\"budget-comparison\" class=\"panel budget-comparison\" {}><summary>Compare switches · {left_cap} vs {right_cap}</summary>",
        if s.budget_comparison_open { "open" } else { "" }
    );
    if !s.budget_comparison_open {
        html.push_str("</details>");
        return html;
    }
    let comparison =
        crate::tradeoffs::compare(&s.saved.snapshot, &s.saved.options, left_cap, right_cap);
    html.push_str("<div class=\"budget-body\"><div class=\"budget-cap-fields\">");
    for (id, label, cap) in [
        ("left", "From cap", left_cap),
        ("right", "To cap", right_cap),
    ] {
        html.push_str(&format!("<label>{label}<input id=\"budget-cap-{id}\" type=\"number\" min=\"0\" max=\"11\" value=\"{cap}\"></label>"));
    }
    html.push_str("</div><p class=\"small-note\">Choose any two caps, 0–11. Same account, prayers and supplies; each comparison repacks the bag. Applying a cap keeps your manual inventory edits; review any resulting packing conflicts.</p><div class=\"budget-sides\">");
    for (key, side) in [("left", &comparison.left), ("right", &comparison.right)] {
        html.push_str(&format!("<div class=\"budget-side\"><h3>{}-item cap</h3><dl><div><dt>Switch pieces</dt><dd>{}</dd></div><div><dt>Largest switch</dt><dd>{}</dd></div><div><dt>Peak bag</dt><dd>{}/28</dd></div></dl><button data-action=\"budget-apply:{key}\" class=\"phase\" {}>Use {}</button>",side.cap,side.carried_gear,side.max_pair,side.max_inventory_space,if side.feasible{""}else{"disabled"},side.cap));
        if !side.feasible {
            html.push_str("<p class=\"budget-blocked\">No setup fits this cap.</p>");
        }
        if !side.prep_blockers.is_empty() {
            html.push_str("<details><summary>Preparation gaps</summary><ul>");
            for item in &side.prep_blockers {
                html.push_str(&format!("<li>{}</li>", esc(item)));
            }
            html.push_str("</ul></details>");
        }
        html.push_str("</div>");
    }
    html.push_str("</div>");
    if comparison.left.feasible && comparison.right.feasible {
        let extra = comparison.right.max_inventory_space as i32
            - comparison.left.max_inventory_space as i32;
        html.push_str(&format!(
            "<p class=\"budget-space\">{extra:+} peak bag slots at the {}-item cap.</p>",
            right_cap
        ));
        if !comparison.added_gear.is_empty() || !comparison.removed_gear.is_empty() {
            html.push_str("<details class=\"budget-gear\"><summary>Gear that changes</summary>");
            for (label, ids) in [
                ("Added", &comparison.added_gear),
                ("Removed", &comparison.removed_gear),
            ] {
                if !ids.is_empty() {
                    html.push_str(&format!(
                        "<p><strong>{label}:</strong> {}.</p>",
                        ids.iter()
                            .map(|id| format!("{} {}", image(*id), esc(&planner::item_name(*id))))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            html.push_str("</details>");
        }
    }
    html.push_str(&encounter_overview(&comparison));
    html.push_str(&format!(
        "<details id=\"budget-phases\" {}><summary>Phase details</summary>",
        if s.budget_phases_open { "open" } else { "" }
    ));
    let mut keys = Vec::<(i32, String, String)>::new();
    for window in comparison
        .left
        .windows
        .iter()
        .chain(&comparison.right.windows)
    {
        if !keys
            .iter()
            .any(|(id, style, _)| *id == window.target_id && style == &window.style)
        {
            keys.push((window.target_id, window.style.clone(), window.name.clone()));
        }
    }
    let key_value = |id: i32, style: &str| format!("{id}:{style}");
    let default_key = keys
        .iter()
        .max_by(|(aid, astyle, _), (bid, bstyle, _)| {
            let score = |id: i32, style: &str| {
                let a = comparison
                    .left
                    .windows
                    .iter()
                    .find(|w| w.target_id == id && w.style == style);
                let b = comparison
                    .right
                    .windows
                    .iter()
                    .find(|w| w.target_id == id && w.style == style);
                a.zip(b)
                    .map(|(a, b)| {
                        (b.metrics.dps.unwrap_or(0.0) - a.metrics.dps.unwrap_or(0.0)).abs() * 1000.0
                            + (b.metrics.prayer_per_minute - a.metrics.prayer_per_minute).abs()
                    })
                    .unwrap_or(0.0)
            };
            score(*aid, astyle).total_cmp(&score(*bid, bstyle))
        })
        .map(|(id, style, _)| key_value(*id, style));
    let selected = s
        .budget_window
        .as_ref()
        .filter(|value| {
            keys.iter()
                .any(|(id, style, _)| key_value(*id, style) == **value)
        })
        .cloned()
        .or(default_key);
    if let Some(selected) = selected {
        html.push_str("<label class=\"budget-window\">Attack window<select id=\"budget-window\">");
        for (id, style, name) in &keys {
            let value = key_value(*id, style);
            html.push_str(&format!(
                "<option value=\"{}\" {}>{} · {}</option>",
                esc(&value),
                if value == selected { "selected" } else { "" },
                esc(name),
                esc(style)
            ));
        }
        html.push_str("</select></label>");
        let find = |side: &crate::tradeoffs::Budget| {
            side.windows
                .iter()
                .find(|w| key_value(w.target_id, &w.style) == selected)
                .map(|w| w.metrics.clone())
        };
        let a = find(&comparison.left);
        let b = find(&comparison.right);
        let ad = a.as_ref().and_then(|m| m.dps);
        let bd = b.as_ref().and_then(|m| m.dps);
        let ap = a
            .as_ref()
            .filter(|m| !m.defence.is_empty())
            .map(|m| m.prayer_per_minute);
        let bp = b
            .as_ref()
            .filter(|m| !m.defence.is_empty())
            .map(|m| m.prayer_per_minute);
        html.push_str(&format!(
            "<div class=\"budget-metrics\">{}{}</div>",
            budget_stat("Outgoing DPS", ad, bd, 3, false, true),
            budget_stat("Prayer / minute", ap, bp, 2, true, false)
        ));
        if ad.is_none() || bd.is_none() {
            for reason in a
                .iter()
                .chain(b.iter())
                .filter_map(|m| m.dps_reason.as_deref())
                .collect::<std::collections::BTreeSet<_>>()
            {
                html.push_str(&format!("<p class=\"small-note\">{}</p>", esc(reason)));
            }
        }
        let attacks = a
            .iter()
            .chain(b.iter())
            .flat_map(|m| m.incoming.iter().map(|v| v.attack.clone()))
            .collect::<std::collections::BTreeSet<_>>();
        if !attacks.is_empty() {
            let attack = s
                .budget_incoming
                .as_ref()
                .filter(|v| attacks.contains(*v))
                .cloned()
                .unwrap_or_else(|| attacks.iter().next().unwrap().clone());
            html.push_str("<label>Incoming scenario<select id=\"budget-incoming\">");
            for name in attacks {
                html.push_str(&format!(
                    "<option {}>{}</option>",
                    if name == attack { "selected" } else { "" },
                    esc(&name)
                ));
            }
            html.push_str("</select></label>");
            let incoming = |m: &Option<crate::tradeoffs::Metrics>| {
                m.as_ref()
                    .and_then(|m| m.incoming.iter().find(|v| v.attack == attack))
                    .cloned()
            };
            let ai = incoming(&a);
            let bi = incoming(&b);
            html.push_str(&budget_stat(
                "Incoming DPS",
                ai.as_ref().and_then(|v| v.dps),
                bi.as_ref().and_then(|v| v.dps),
                3,
                true,
                true,
            ));
            let reasons = ai
                .iter()
                .chain(bi.iter())
                .filter_map(|v| v.reason.as_deref())
                .collect::<std::collections::BTreeSet<_>>();
            for reason in reasons {
                html.push_str(&format!("<p class=\"small-note\">{}</p>", esc(reason)));
            }
        }
        let mut delta = BTreeMap::new();
        for key in ["stab", "slash", "crush", "magic", "ranged"] {
            if let Some((av, bv)) = a
                .as_ref()
                .and_then(|m| m.defence.get(key))
                .zip(b.as_ref().and_then(|m| m.defence.get(key)))
            {
                delta.insert(key.to_string(), bv - av);
            }
        }
        let changes = signed_defence(&delta);
        html.push_str(&format!("<details class=\"budget-defence\"><summary>Defence bonuses{}</summary><table class=\"bonus-table\"><thead><tr><th>Type</th><th>{left_cap}</th><th>{right_cap}</th><th>Change</th></tr></thead><tbody>",if changes.is_empty(){String::new()}else{format!(" · {}",esc(&changes))}));
        for (key, label) in [
            ("stab", "Stab"),
            ("slash", "Slash"),
            ("crush", "Crush"),
            ("magic", "Magic"),
            ("ranged", "Ranged"),
        ] {
            let value = |m: &Option<crate::tradeoffs::Metrics>| {
                m.as_ref()
                    .and_then(|m| m.defence.get(key))
                    .map(|v| format!("{v:+}"))
                    .unwrap_or_else(|| "—".into())
            };
            html.push_str(&format!(
                "<tr><th>{label}</th><td>{}</td><td>{}</td><td>{}</td></tr>",
                value(&a),
                value(&b),
                delta
                    .get(key)
                    .map(|v| format!("{v:+}"))
                    .unwrap_or_else(|| "—".into())
            ));
        }
        html.push_str("</tbody></table><p class=\"small-note\">Equipment bonuses. Damage taken uses the selected incoming scenario.</p></details>");
        html.push_str("<details><summary>All attack windows</summary><div class=\"comparison-scroll\"><table class=\"bonus-table\"><thead><tr><th>Window</th><th>DPS</th><th>Prayer / min</th></tr></thead><tbody>");
        for (id, style, name) in &keys {
            let a = comparison
                .left
                .windows
                .iter()
                .find(|w| w.target_id == *id && w.style == *style);
            let b = comparison
                .right
                .windows
                .iter()
                .find(|w| w.target_id == *id && w.style == *style);
            let dps = |w: Option<&crate::tradeoffs::Window>| {
                w.and_then(|w| w.metrics.dps)
                    .map(|v| format!("{v:.3}"))
                    .unwrap_or_else(|| "—".into())
            };
            let pray = |w: Option<&crate::tradeoffs::Window>| {
                w.filter(|w| !w.metrics.defence.is_empty())
                    .map(|w| format!("{:.2}", w.metrics.prayer_per_minute))
                    .unwrap_or_else(|| "—".into())
            };
            html.push_str(&format!(
                "<tr><th>{} · {}</th><td>{} → {}</td><td>{} → {}</td></tr>",
                esc(name),
                esc(style),
                dps(a),
                dps(b),
                pray(a),
                pray(b)
            ));
        }
        html.push_str("</tbody></table></div></details>");
    }
    html.push_str("</details><details><summary>Coverage & assumptions</summary><ul>");
    for note in comparison
        .notes
        .iter()
        .chain(&comparison.left.notes)
        .chain(&comparison.right.notes)
        .collect::<std::collections::BTreeSet<_>>()
    {
        html.push_str(&format!("<li>{}</li>", esc(note)));
    }
    html.push_str("</ul></details></div></details>");
    html
}

fn switch_panel(s: &State, p: &Plan) -> String {
    if s.shared_setup.is_some()
        || s.budget_comparison_open
        || p.optimization.is_none()
            && s.saved.options.max_switches.is_none()
            && encounter_targets(s).len() <= 1
    {
        return String::new();
    }
    let mut html = String::from(
        "<section class=\"panel switch-panel\"><div class=\"panel-heading\"><h2>Current setup details</h2></div><div class=\"combat-body\"><details><summary>Phase priorities</summary><p>1 is normal. Increase to favour a phase; 0 gives it the lowest priority.</p>",
    );
    let mut priority_ids = std::collections::BTreeSet::new();
    for id in encounter_targets(s)
        .into_iter()
        .filter(|id| priority_ids.insert(*id))
    {
        if let Some(target) = planner::monster(id) {
            html.push_str(&format!(r#"<label>{}<input id="priority-{id}" type="number" min="0" max="10" value="{}"></label>"#,esc(&target.name),s.saved.options.phase_priorities.get(&id).copied().unwrap_or(1)));
        }
    }
    html.push_str("</details>");
    if let Some(report) = &p.optimization {
        html.push_str(&format!("<p><strong>At most {} changed slots · {} carried style-switch items</strong></p><div class=\"comparison-scroll\"><table class=\"bonus-table\"><thead><tr><th>Attack window</th><th>Outgoing DPS<br>before → after</th><th>Incoming scenario DPS<br>before → after</th><th>Prayer / min<br>before → after</th></tr></thead><tbody>",report.max_changes,report.gear_slots));
        for row in &report.comparisons {
            html.push_str(&format!(
                "<tr><th>{} ({})</th><td>{} → {}</td><td>{} → {}</td><td>{:.1} → {:.1}</td></tr>",
                esc(&row.name),
                esc(&row.style),
                row.before.map(|d| format!("{d:.3}")).unwrap_or("—".into()),
                row.after.map(|d| format!("{d:.3}")).unwrap_or("—".into()),
                row.incoming_before
                    .map(|d| format!("{d:.3}"))
                    .unwrap_or("—".into()),
                row.incoming_after
                    .map(|d| format!("{d:.3}"))
                    .unwrap_or("—".into()),
                row.prayer_before,
                row.prayer_after,
            ));
        }
        html.push_str(&format!(
            "</tbody></table></div><p class=\"small-note\">Incoming values use the first recorded attack scenario, not a weighted mix of all attacks. — means unverified or unavailable, not zero.</p><details><summary>Search coverage</summary><p>{}</p></details>",
            esc(&report.note)
        ));
    } else if s.saved.options.max_switches.is_some() {
        html.push_str("<p class=\"combat-unavailable\">No feasible budget setup found. Review preparation blockers below.</p>");
    }
    html.push_str("</div></section>");
    html
}

fn ranking_controls(s: &State, prefix: &str) -> String {
    let preferences = s.ranking_draft.as_ref().unwrap_or(&s.saved.options.ranking);
    let mut html = String::from(
        "<fieldset class=\"ranking-controls\"><legend>Gear ranking</legend><div class=\"ranking-weights\">",
    );
    for ((key, label), value) in [
        ("dps", "DPS"),
        ("defence", "Defence"),
        ("prayer", "Prayer"),
        ("weight", "Lighter gear"),
    ]
    .into_iter()
    .zip(preferences.weights.values())
    {
        html.push_str(&format!("<label for=\"{prefix}-{key}\">{label}<input id=\"{prefix}-{key}\" type=\"number\" min=\"0\" max=\"100\" step=\"0.01\" value=\"{value}\" aria-label=\"{label} importance\"></label>"));
    }
    html.push_str(&format!("</div><label class=\"ranking-defence\">Defence measure<select id=\"{prefix}-defence-type\">"));
    for kind in crate::ranking::Defence::ALL {
        html.push_str(&format!(
            "<option value=\"{}\" {}>{}</option>",
            kind.key(),
            if kind == preferences.defence {
                "selected"
            } else {
                ""
            },
            kind.label()
        ));
    }
    html.push_str("</select></label><p>Weights, not percentages. 0 ignores a stat.</p><div class=\"ranking-presets\">");
    for (key, label) in [
        ("damage", "DPS first"),
        ("prayer", "DPS + prayer"),
        ("defence", "DPS + defence"),
        ("equal", "All three equal"),
    ] {
        html.push_str(&button(&format!("rank-preset:{key}"), label, "phase"));
    }
    html.push_str(&format!(
        "</div><p id=\"{prefix}-status\">{}</p>{}</fieldset>",
        if s.ranking_draft.is_some() {
            "Apply to update gear."
        } else {
            ""
        },
        button("rank-apply", "Apply", "phase")
    ));
    if !s.saved.options.gear_overrides.is_empty() {
        html.push_str(&format!(
            "<p>Manual gear choices stay fixed. {}</p>",
            button("rank-reset", "Reset item choices", "phase")
        ));
    }
    html
}

fn update_gear_ranking(id: &str, value: &str) {
    STATE.with(|state| {
        let mut s = state.borrow_mut();
        let mut preferences = s
            .ranking_draft
            .clone()
            .unwrap_or_else(|| s.saved.options.ranking.clone());
        let key = id
            .strip_prefix("plan-rank-")
            .or_else(|| id.strip_prefix("rank-"))
            .unwrap_or("");
        if key == "defence-type" {
            if let Some(kind) = crate::ranking::Defence::ALL
                .into_iter()
                .find(|kind| kind.key() == value)
            {
                preferences.defence = kind;
            }
        } else if let Ok(value) = value.parse::<f64>() {
            preferences.weights.set(key, value);
        } else {
            return;
        }
        s.ranking_draft = Some(preferences);
        if let Some(results) = document().get_element_by_id("gear-pick-results") {
            results.set_inner_html(&gear_choices(&s));
        }
        for prefix in ["rank", "plan-rank"] {
            if let Some(note) = document().get_element_by_id(&format!("{prefix}-status")) {
                note.set_text_content(Some("Apply to update gear."));
            }
        }
    });
}

fn picker_priorities(s: &State) -> String {
    format!(
        "<details class=\"picker-priorities\"><summary>Gear priorities</summary>{}</details>",
        ranking_controls(s, "rank")
    )
}

fn gear_picker(s: &State) -> String {
    let Some(slot) = s.edit_equipment else {
        return String::new();
    };
    let style = comparison_style(s);
    format!(
        r#"<div class="picker-backdrop"><section class="gear-picker" role="dialog" aria-modal="true" aria-labelledby="gear-picker-title"><div class="panel-heading"><h2 id="gear-picker-title">{} · owned equipment</h2>{}</div><div class="picker-controls"><label>Search your items<input id="gear-search" type="search" value="{}" placeholder="Search owned items…" autocomplete="off"></label></div>{}<p class="picker-note">{style} vs {} · setup totals</p><div class="picker-actions">{} {}</div><div class="picker-scroll"><div id="gear-pick-results" class="gear-pick-results" aria-live="polite">{}</div><details class="secondary-section"><summary>Upgrade reference</summary><div class="bis-scale">{}</div></details></div></section></div>"#,
        planner::SLOTS[slot],
        button("close-picker", "Close ×", "phase"),
        esc(&s.gear_search),
        picker_priorities(s),
        esc(comparison_monster(s)
            .map(|m| m.name.as_str())
            .unwrap_or("selected encounter")),
        button(&format!("auto-gear:{slot}"), "Equip top result", "phase"),
        button(
            &format!("pick-gear:{style}:{slot}:0"),
            "Leave empty",
            "phase"
        ),
        gear_choices(s),
        bis_scale(s)
    )
}

struct GearChoice {
    gear: &'static planner::Gear,
    usable: bool,
    metrics: crate::ranking::Metrics,
    score: Option<f64>,
    reason: Option<String>,
}

fn ranked_gear(s: &State) -> (Vec<GearChoice>, Vec<&'static str>) {
    let Some(slot) = s.edit_equipment else {
        return (vec![], vec![]);
    };
    let Some(member) = s.saved.snapshot.selected() else {
        return (vec![], vec![]);
    };
    let style = comparison_style(s);
    let p = s.plan();
    let set = planned_style(s, &p, style);
    let owned = s.saved.snapshot.available(false);
    // Rank the entire usable pool. Searching must not change scores or relative order.
    let mut items: Vec<_> = planner::equipment()
        .iter()
        .filter(|g| g.slot == planner::SLOT_KEYS[slot] && owned.get(&g.id).is_some_and(|n| *n > 0))
        .map(|g| {
            let compatible = slot != 13
                || set[3].and_then(crate::rules::weapon).is_none_or(|rule| {
                    rule.family.style != "Ranged"
                        || rule.family.ammo.is_empty()
                        || rule.family.ammo.contains(&g.id)
                });
            let usable = compatible
                && planner::can_use(g.id, member)
                && !s.saved.options.item_excluded(g.id)
                && (slot != 13
                    || owned.get(&g.id).copied().unwrap_or(0) >= s.saved.options.combat.ammo_shots);
            let mut trial = set.clone();
            trial[slot] = Some(g.id);
            if slot == 3 && g.two_handed {
                trial[5] = None;
            }
            let trial_style = if slot == 3
                && s.saved.options.boss == Boss::Custom
                && s.saved.options.targets.len() == 1
            {
                crate::rules::weapon(g.id)
                    .filter(|rule| !rule.permits(style))
                    .map(|rule| rule.family.style.as_str())
                    .unwrap_or(style)
            } else {
                style
            };
            if slot == 3
                && trial_style == "Ranged"
                && !crate::rules::ammo_ids(g.id).contains(&trial[13].unwrap_or(0))
            {
                trial[13] = crate::rules::ammo_ids(g.id)
                    .iter()
                    .copied()
                    .filter(|id| {
                        owned.get(id).copied().unwrap_or(0) >= s.saved.options.combat.ammo_shots
                            && planner::can_use(*id, member)
                    })
                    .max_by(|a, b| {
                        let evaluate = |id| {
                            let mut set = trial.clone();
                            set[13] = Some(id);
                            comparison_monster(s)
                                .and_then(|target| {
                                    combat::calculate_configured(
                                        member,
                                        &set,
                                        trial_style,
                                        target,
                                        "",
                                        s.saved.options.raid_level,
                                        &s.saved.options.combat,
                                    )
                                    .ok()
                                })
                                .map(|r| r.dps)
                        };
                        evaluate(*a)
                            .partial_cmp(&evaluate(*b))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
            }
            let calculation = if usable {
                comparison_monster(s).map(|target| {
                    combat::calculate_configured(
                        member,
                        &trial,
                        trial_style,
                        target,
                        comparison_spell(s, member, &trial, target),
                        s.saved.options.raid_level,
                        &s.saved.options.combat,
                    )
                })
            } else {
                None
            };
            let dps = calculation
                .as_ref()
                .and_then(|r| r.as_ref().ok())
                .map(|r| r.dps);
            let reason = if !compatible {
                Some("Not compatible with the equipped weapon".into())
            } else {
                calculation.and_then(Result::err)
            };
            let known_bonuses = trial
                .iter()
                .flatten()
                .all(|id| combat::stats(*id).is_some());
            let bonuses = combat::bonuses(&trial);
            let defence = known_bonuses.then(|| {
                if s.ranking_draft
                    .as_ref()
                    .unwrap_or(&s.saved.options.ranking)
                    .defence
                    == crate::ranking::Defence::Total
                {
                    bonuses.defensive.values().sum::<i32>() as f64
                } else {
                    bonuses
                        .defensive
                        .get(
                            s.ranking_draft
                                .as_ref()
                                .unwrap_or(&s.saved.options.ranking)
                                .defence
                                .key(),
                        )
                        .copied()
                        .unwrap_or(0) as f64
                }
            });
            GearChoice {
                gear: g,
                usable,
                metrics: crate::ranking::Metrics {
                    dps,
                    defence,
                    prayer: known_bonuses.then(|| crate::prayer::bonus(&trial) as f64),
                    weight: crate::ranking::equipment_weight(&trial),
                },
                score: None,
                reason,
            }
        })
        .collect();
    let metrics: Vec<_> = items
        .iter()
        .filter(|row| row.usable)
        .map(|row| row.metrics)
        .collect();
    let ranking = crate::ranking::rank(
        &metrics,
        &s.ranking_draft
            .as_ref()
            .unwrap_or(&s.saved.options.ranking)
            .weights,
    );
    let mut scores = ranking.scores.into_iter();
    for row in &mut items {
        if row.usable {
            row.score = scores.next().flatten();
        }
    }
    items.sort_by(|a, b| {
        b.usable
            .cmp(&a.usable)
            .then_with(|| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.gear.name.cmp(&b.gear.name))
            .then_with(|| a.gear.id.cmp(&b.gear.id))
    });
    (items, ranking.unavailable)
}

fn gear_choices(s: &State) -> String {
    let Some(slot) = s.edit_equipment else {
        return String::new();
    };
    let style = comparison_style(s);
    let p = s.plan();
    let set = planned_style(s, &p, style);
    let owned = s.saved.snapshot.available(false);
    let query = s.gear_search.to_lowercase();
    let (items, unavailable) = ranked_gear(s);
    let items: Vec<_> = items
        .into_iter()
        .filter(|row| {
            query.is_empty()
                || planner::item_name(row.gear.id)
                    .to_lowercase()
                    .contains(&query)
        })
        .collect();
    let mut html = format!(
        "<p class=\"picker-note\">{} owned items · combined score / 100</p>",
        items.len()
    );
    if s.ranking_draft
        .as_ref()
        .unwrap_or(&s.saved.options.ranking)
        .weights
        .values()
        .iter()
        .all(|v| *v == 0.0)
    {
        html.push_str(
            "<p class=\"ranking-notice\">Set at least one importance above 0 to rank gear.</p>",
        );
    } else if !unavailable.is_empty() {
        html.push_str(&format!("<p class=\"ranking-notice\">{} unavailable for these items; omitted from the score.</p>", esc(&unavailable.join(", "))));
    }
    html.push_str("<details class=\"ranking-help\"><summary>How scores work</summary><p>Each stat is scaled across all usable owned items in this slot. Search does not change scores. Figures describe the resulting setup. Missing enabled stats rank below complete results. Defence uses bonuses, not a full fight simulation.</p></details>");
    if items.is_empty() {
        html.push_str("<p>No owned items match this search.</p>");
    }
    let metric = |value: Option<f64>, decimals: usize| {
        value
            .map(|n| format!("{n:.decimals$}"))
            .unwrap_or_else(|| "?".into())
    };
    for row in items {
        let g = row.gear;
        let state = if !row.usable {
            "Excluded or skill/ammo requirements unmet"
        } else if set[slot] == Some(g.id) {
            "Equipped"
        } else {
            ""
        };
        let state = if state.is_empty() {
            format!("Owned × {}", owned[&g.id])
        } else {
            format!("{state} · owned × {}", owned[&g.id])
        };
        let state = if let Some(reason) = &row.reason {
            format!("{state} · {reason}")
        } else {
            state
        };
        let score = row
            .score
            .map(|n| format!("{n:.1}"))
            .unwrap_or_else(|| "—".into());
        html.push_str(&format!(r#"<button class="gear-pick {}" data-action="pick-gear:{style}:{slot}:{}" {}>{}<span><strong>{}</strong><small>{} DPS · {} defence · {} prayer · {} kg</small><small>{state}</small></span><b class="ranking-score" aria-label="Combined score {score} out of 100">{score}<small>/ 100</small></b></button>"#,
            if set[slot] == Some(g.id) { "selected" } else { "" }, g.id, if row.usable { "" } else { "disabled" }, image(g.id), esc(&planner::item_name(g.id)),
            metric(row.metrics.dps, 3), metric(row.metrics.defence, 0), metric(row.metrics.prayer, 0), metric(row.metrics.weight, 3)));
    }
    html
}

fn bis_scale(s: &State) -> String {
    let Some(slot) = s.edit_equipment else {
        return String::new();
    };
    let style = comparison_style(s);
    let p = s.plan();
    let set = planned_style(s, &p, style);
    let owned = s.saved.snapshot.available(false);
    let Some(member) = s.saved.snapshot.selected() else {
        return String::new();
    };
    let choices = reference::tiers(crate::rules::base_style(style), slot);
    if choices.is_empty() {
        return String::new();
    }
    let mut html = String::from(
        "<h3>BIS & upgrade scale</h3><p class=\"picker-note\">Reviewed main-game progression examples. Exact order depends on your target, attack style and other gear; this is not a global BIS proof. Unowned items can become upgrade goals.</p><div class=\"bis-items\">",
    );
    for (rank, g) in choices.into_iter().enumerate() {
        let have = owned.contains_key(&g.id);
        let mut trial = set.clone();
        trial[slot] = Some(g.id);
        if slot == 3 && g.two_handed {
            trial[5] = None;
        }
        let dps = comparison_monster(s)
            .and_then(|m| {
                combat::calculate_configured(
                    member,
                    &trial,
                    style,
                    m,
                    comparison_spell(s, member, &trial, m),
                    s.saved.options.raid_level,
                    &s.saved.options.combat,
                )
                .ok()
            })
            .map(|r| format!("{:.3} DPS", r.dps))
            .unwrap_or_else(|| "Effect not modelled".into());
        let eligible = planner::can_use(g.id, member);
        html.push_str(&format!(r#"<article class="bis-item {}"><small>Tier {}</small>{}<strong>{}</strong><span>{}</span><small>{}</small>{}</article>"#,if have{"owned"}else{""},rank+1,image(g.id),esc(&planner::item_name(g.id)),esc(&dps),if have{"Owned"}else{"Not owned"},if have&&eligible{button(&format!("pick-gear:{style}:{slot}:{}",g.id),"Equip","phase")}else if !have{button(&format!("goal-item:{}",g.id),"Track upgrade","phase")}else{"<span>Check skill requirements</span>".into()}));
    }
    html.push_str("</div>");
    if slot == 3 && crate::rules::base_style(style) == "Melee" {
        html.push_str(&keris_family(s, &set));
    }
    html
}

fn keris_family(s: &State, set: &Vec<Option<i32>>) -> String {
    let Some(member) = s.saved.snapshot.selected() else {
        return String::new();
    };
    let owned = s.saved.snapshot.available(false);
    let Some(target) = planner::monster(11719) else {
        return String::new();
    };
    let mut html = String::from(
        r#"<section class="keris-family"><h3>Keris variants</h3><p>Full-HP Kephri comparison, using your other equipment. A plain partisan never counts as an upgraded one.</p><table class="bonus-table"><thead><tr><th>Exact weapon</th><th>Ownership</th><th>DPS</th><th>Effect</th></tr></thead><tbody>"#,
    );
    for (id, effect) in reference::KERIS_VARIANTS {
        let have = owned.contains_key(&id);
        let mut trial = set.clone();
        trial[3] = Some(id);
        let dps = combat::calculate_configured(
            member,
            &trial,
            "Melee",
            target,
            "",
            s.saved.options.raid_level,
            &s.saved.options.combat,
        )
        .ok()
        .map(|r| format!("{:.3}", r.dps))
        .unwrap_or("—".into());
        html.push_str(&format!(
            "<tr><th>{} {}</th><td>{}</td><td>{dps}</td><td>{}</td></tr>",
            image(id),
            esc(&planner::item_name(id)),
            if have { "Owned" } else { "Not owned" },
            esc(effect)
        ));
    }
    html.push_str("</tbody></table></section>");
    html
}

fn wilderness_controls(s: &State) -> String {
    let mut html = String::from(
        "<section class=\"wilderness-controls\" aria-label=\"Wilderness filters\"><div class=\"wilderness-location\" role=\"group\" aria-label=\"Boss location\">",
    );
    for (key, label) in [
        ("all", "All locations"),
        ("safe", "Avoid Wilderness"),
        ("wild", "Wilderness only"),
    ] {
        html.push_str(&format!("<button class=\"encounter-filter {}\" data-action=\"wilderness-location:{key}\" aria-pressed=\"{}\">{label}</button>",if s.wilderness_location==key{"active"}else{""},s.wilderness_location==key));
    }
    html.push_str(&format!("</div><button class=\"encounter-filter scaredy-toggle {}\" data-action=\"wilderness-mode\" aria-pressed=\"{}\">Scaredy cat · 3 valuable items</button>",if s.scaredy_cat{"active"}else{""},s.scaredy_cat));
    if s.scaredy_cat {
        html.push_str("<div class=\"wilderness-cap\" role=\"group\" aria-label=\"Disposable gear price ceiling\"><span>Cheap gear per piece / ammo stack:</span>");
        for cap in crate::wilderness::CAPS {
            html.push_str(&format!("<button class=\"encounter-filter {}\" data-action=\"wilderness-cap:{cap}\" aria-pressed=\"{}\">≤{}k GP</button>",if s.filler_cap==cap{"active"}else{""},s.filler_cap==cap,cap/1000));
        }
        html.push_str("</div><p>Wilderness cards recalculate with up to 3 valuable gear pieces plus owned cheap fillers. Open a card to see the exact kit and full-gear comparison. Selecting it keeps that kit in the planner, where the full packed death-risk report includes supplies and charges.</p><small>Unskulled + PK Skull Prevention assumed. Confirm Items Kept on Death; supplies, charges and loot add risk.</small>");
    }
    html.push_str("</section>");
    html
}
fn wilderness_banner(s: &State) -> String {
    let Some(kit) = &s.saved.options.wilderness else {
        return String::new();
    };
    format!(
        "<section class=\"wilderness-plan-banner\"><strong>Scaredy cat kit active · up to 3 valuable gear pieces</strong><p>Gear is locked to your screened kit; cheap fillers ≤{} GP per piece/stack. Changing quantities rechecks the carried gear limit.</p><details><summary>Protection assumptions</summary><p>{}</p></details>{}</section>",
        kit.filler_cap,
        esc(crate::wilderness::CAUTION),
        button("wilderness-off", "Return to full gear", "text-button")
    )
}

fn death_risk_panel(s: &State, plan: &Plan) -> String {
    let exposed = selected_encounter(&s.saved.options).is_some_and(|entry| {
        crate::wilderness::exposure(&entry.name) != crate::wilderness::Exposure::None
    });
    if s.saved.options.wilderness.is_none() && !exposed {
        return String::new();
    }
    let defaults = crate::death_risk::Settings::default();
    let settings = if s.shared_setup.is_some() {
        &defaults
    } else {
        &s.saved.death_risk
    };
    crate::death_risk::render(plan, settings, s.risk_open, s.shared_setup.is_none())
}
