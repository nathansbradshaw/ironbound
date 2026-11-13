use crate::{
    account::{self, SKILLS, Snapshot},
    combat, export,
    planner::{self, Boss, Options, Plan},
    reference, share,
};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap};
use wasm_bindgen::{JsCast, prelude::*};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Element, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

const STORAGE: &str = "ironbound.account.v1";
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u32,
    snapshot: Snapshot,
    options: Options,
    pins: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    last_setup_fragment: Option<String>,
}
struct State {
    saved: Saved,
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
    building_group: bool,
    raid_rooms_open: bool,
    switch_preferences_open: bool,
    ranking_draft: Option<crate::ranking::Preferences>,
}
impl State {
    fn initial() -> Self {
        let restored = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(STORAGE).ok().flatten())
            .and_then(|s| serde_json::from_str::<Saved>(&s).ok())
            .filter(|s| {
                s.version == 1
                    && s.snapshot.selected().is_some()
                    && Snapshot::parse(
                        &serde_json::to_string(&s.snapshot).unwrap_or_default(),
                        s.snapshot.fetched_at.clone(),
                    )
                    .is_ok()
            });
        let mut saved = restored.unwrap_or(Saved {
            version: 1,
            snapshot: account::demo(),
            options: Options::default(),
            pins: BTreeMap::new(),
            last_setup_fragment: None,
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
            encounter_filter: "Popular".into(),
            building_group: false,
            raid_rooms_open: false,
            switch_preferences_open: false,
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
        if self.demo_return.is_some() {
            return;
        }
        let mut stored = self.saved.clone();
        if self.shared_setup.is_none() {
            let base = format!(
                "{}{}",
                window().location().origin().unwrap_or_default(),
                window().location().pathname().unwrap_or_default()
            );
            let hash = share::Setup::from_plan(&self.plan(), &self.saved.options)
                .fragment()
                .unwrap_or_default();
            stored.last_setup_fragment = Some(hash.clone());
            let _ = window().history().and_then(|h| {
                h.replace_state_with_url(&JsValue::NULL, "", Some(&format!("{base}{hash}")))
            });
        }
        if let Some(storage) = window().local_storage().ok().flatten() {
            if storage
                .set_item(STORAGE, &serde_json::to_string(&stored).unwrap_or_default())
                .is_err()
            {
                web_sys::console::warn_1(
                    &"Browser storage unavailable; download a backup before leaving.".into(),
                );
            }
        }
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
    // Every client downloads the same public icon pack inside Wasm; no owned-item URLs leave the page.
    static ICONS: std::sync::OnceLock<BTreeMap<i32, String>> = std::sync::OnceLock::new();
    let icons = ICONS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/item-icons-pack.json"))
            .expect("public icon pack")
    });
    match icons.get(&id) {
        Some(data) => format!(
            r#"<img src="data:image/png;base64,{data}" alt="" width="32" height="32" loading="lazy">"#
        ),
        None => "<span class=\"unknown-item\" aria-hidden=\"true\">?</span>".into(),
    }
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

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    render();
    let click = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        let target = e
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|e| e.closest("[data-action]").ok().flatten());
        if let Some(el) = target {
            if let Some(action) = el.get_attribute("data-action") {
                let return_slot = STATE.with(|s| s.borrow().edit_equipment);
                let return_inventory = STATE.with(|s| s.borrow().edit_inventory);
                act(&action);
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
            }
        }
    });
    document().add_event_listener_with_callback("input", search.as_ref().unchecked_ref())?;
    search.forget();
    let disclosure = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if let Some(el) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) {
            if el.id() == "raid-room-picker" {
                STATE.with(|state| state.borrow_mut().raid_rooms_open = el.has_attribute("open"));
            } else if el.id() == "switch-preferences" {
                STATE.with(|state| {
                    state.borrow_mut().switch_preferences_open = el.has_attribute("open")
                });
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
        // Draft form fields must survive blur: they are committed by their action button.
        if ![
            "max-switches",
            "switch-slots",
            "focus-style",
            "compare-target",
            "incoming-attack",
            "protection",
            "offensive-prayer",
            "preserve",
            "mark-darkness",
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
        {
            return;
        }
        STATE.with(|state|{let mut s=state.borrow_mut();if s.shared_setup.is_some() && !id.starts_with("skill-"){s.notice("Choose Rebuild using my items before editing this shared setup.",true);return;}match id.as_str(){
            "inventory-filter"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.inventory_filter=sel.value();}},
            _ if id.starts_with("priority-")=>{if let(Ok(target),Ok(weight))=(id[9..].parse::<i32>(),input(&id).parse::<u32>()){if encounter_targets(&s).contains(&target){s.saved.options.phase_priorities.insert(target,weight.min(10));}}},
            "incoming-attack"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.incoming_attack=Some(sel.value());}},
            "protection"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){let value=sel.value();if ["None","Melee","Ranged","Magic"].contains(&value.as_str()){s.saved.options.combat.protection=value;}}},
            "offensive-prayer"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){let value=sel.value();if crate::prayer::prayers().iter().any(|p|p.name==value){s.saved.options.combat.offensive=value;}}},
            "preserve"=>s.saved.options.combat.preserve=el.dyn_ref::<HtmlInputElement>().is_some_and(|e|e.checked()),
            "mark-darkness"=>s.saved.options.combat.mark_darkness=el.dyn_ref::<HtmlInputElement>().is_some_and(|e|e.checked()),
            "safety"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.saved.options.combat.safety=sel.value().parse::<u32>().unwrap_or(1).min(3);}},
            "prayer-weight"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.saved.options.combat.prayer_weight=sel.value().parse::<u32>().unwrap_or(0).min(3);}},
            "max-switches"=>{if let Ok(control)=el.dyn_into::<HtmlInputElement>(){let value=control.value();let limit=if value.trim().is_empty(){None}else if let Ok(n)=value.parse::<usize>(){Some(n.min(11))}else{return;};s.saved.options.max_switches=limit;s.saved.options.fewer_switches=false;s.saved.options.inventory_overrides.clear();s.saved.options.inventory_quantities.clear();}},
            "switch-slots"=>{s.saved.options.switch_slots=input("switch-slots").parse::<usize>().unwrap_or(12).min(20);},
            "focus-style"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){let value=sel.value();if value=="Balanced"||crate::rules::STYLES.contains(&value.as_str()){s.saved.options.focus_style=value;}}},
            "compare-target"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.compare_target=sel.value().parse().ok();s.incoming_attack=None;if s.saved.options.boss==Boss::Custom {if let Some(target)=s.saved.options.targets.iter().find(|t|Some(t.id)==s.compare_target){s.edit_style=Some(target.style.clone());}}}},
            _ if id.starts_with("quest-")=>{if let (Ok(i),Ok(sel))=(id[6..].parse::<usize>(),el.dyn_into::<HtmlSelectElement>()){if let Some(quest)=planner::GEAR_QUESTS.get(i){let selected=s.saved.snapshot.selected.clone();if let Some(m)=s.saved.snapshot.members.iter_mut().find(|m|m.name==selected){match sel.value().as_str(){"yes"=>{m.quest_overrides.insert((*quest).into(),2);},"no"=>{m.quest_overrides.insert((*quest).into(),1);},_=>{m.quest_overrides.remove(*quest);}}}}}},
            "member"=>{if let Ok(select)=el.dyn_into::<HtmlSelectElement>(){s.saved.snapshot.selected=select.value();s.saved.options.access_override=None;s.saved.options.started_barrows=None;s.phase=None;}},
            "shared"=>s.saved.options.shared=el.dyn_ref::<HtmlInputElement>().is_some_and(|e|e.checked()),
            "fewer"=>s.saved.options.fewer_switches=el.dyn_ref::<HtmlInputElement>().is_some_and(|e|e.checked()),
            "food"=>{s.saved.options.food=input("food").parse::<usize>().unwrap_or(8).min(20);s.saved.options.inventory_overrides.clear();s.saved.options.inventory_quantities.clear();},
            "potions"=>{s.saved.options.prayer_potions=input("potions").parse::<usize>().unwrap_or(3).min(12);s.saved.options.inventory_overrides.clear();s.saved.options.inventory_quantities.clear();},
            "raid-level"=>s.saved.options.raid_level=input("raid-level").parse::<u32>().unwrap_or(0).min(150),
            "access"=>{if let Ok(select)=el.dyn_into::<HtmlSelectElement>(){s.saved.options.access_override=match select.value().as_str(){"yes"=>Some(true),"no"=>Some(false),_=>None};}},
            "started"=>{if let Ok(select)=el.dyn_into::<HtmlSelectElement>(){s.saved.options.started_barrows=match select.value().as_str(){"yes"=>Some(true),"no"=>Some(false),_=>None};}},
            _ if id.starts_with("gear-")=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){if let Ok(value)=sel.value().parse::<i32>(){s.saved.options.gear_overrides.insert(id[5..].into(),value);s.saved.options.inventory_overrides.clear();}}},
            _ if id.starts_with("skill-")=>{if let (Ok(index),Ok(level))=(id[6..].parse::<usize>(),input(&id).parse::<u32>()){if index<24&&(1..=99).contains(&level){let selected=s.saved.snapshot.selected.clone();if let Some(m)=s.saved.snapshot.members.iter_mut().find(|m|m.name==selected){m.level_overrides.insert(SKILLS[index].into(),level);s.notice("Skill correction saved locally. Public skill lookup replaces local corrections.",false);}}}},
            _=>return,
        }s.export_text=None;s.persist();});
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
    if let Some(filter) = action.strip_prefix("encounter-filter:") {
        if ["Popular", "Bosses", "Raids", "Groups"].contains(&filter) {
            STATE.with(|state| {
                let mut s = state.borrow_mut();
                s.encounter_filter = filter.into();
                s.encounter_query.clear();
                if let Some(input) = document()
                    .get_element_by_id("encounter-search")
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                {
                    input.set_value("");
                }
                refresh_encounter_browser(&s);
            });
        }
        return;
    }
    match action {
        "clear-encounters" => {
            STATE.with(|state| {
                let mut s = state.borrow_mut();
                s.encounter_query.clear();
                s.encounter_filter = "Popular".into();
                if let Some(input) = document()
                    .get_element_by_id("encounter-search")
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                {
                    input.set_value("");
                }
                refresh_encounter_browser(&s);
            });
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
                                "Setup link copied. Only encounter, equipped items and inventory are shared."
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
        if let Some(tab) = action.strip_prefix("tab:") {
            s.tab = tab.into();
            s.edit_equipment = None;
            s.edit_inventory = None;
        } else if let Some(view) = action.strip_prefix("view:") {
            if ["loadout", "combat", "prepare"].contains(&view) {
                s.view = view.into();
                s.edit_equipment = None;
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
            s.saved.options.inventory_overrides.clear();
            s.saved.options.inventory_quantities.clear();
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
                s.saved.options.inventory_overrides.clear();
                s.saved.options.inventory_quantities.clear();
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
                                && s.saved.snapshot.available(false).contains_key(&id)
                                && s.saved
                                    .snapshot
                                    .selected()
                                    .is_some_and(|m| planner::can_use(id, m))))
                    {
                        let mut selected_style = parts[0].to_string();
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
                        s.saved.options.inventory_overrides.clear();
                        s.saved.options.inventory_quantities.clear();
                        s.notice(
                            "Owned equipment selected; switches and supplies rechecked.",
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
        } else if let Some(style) = action.strip_prefix("edit-style:") {
            if crate::rules::STYLES.contains(&style) {
                s.edit_style = Some(style.into());
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
            s.saved.options = Options::default();
            s.saved.pins = BTreeMap::new();
            s.shared_setup = None;
            s.tab = "bossing".into();
            s.phase = None;
            s.edit_style = None;
            s.compare_target = None;
            s.notice("Example preview. Your saved account is unchanged.", false);
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
    if let Ok(mut saved) = serde_json::from_str::<Saved>(raw) {
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

fn render() {
    STATE.with(|state|{let s=state.borrow();let snapshot=&s.saved.snapshot;let options=&s.saved.options;let plan=s.plan();
    let demo=snapshot.source=="Demo";
    let nav=|(key,label,icon):(&str,&str,i32)|format!("<button class=\"nav-item {}\" data-action=\"tab:{key}\">{}<span>{label}</span>{}</button>",if s.tab==key{"active"}else{""},image(icon),if key=="goals"{format!("<span class=\"nav-count\">{}</span>",s.pins().len())}else{String::new()});
    let mut html=format!("<div class=\"app-shell\"><aside class=\"sidebar\"><a class=\"brand\" href=\"#\" aria-label=\"Ironbound home\"><span class=\"brand-mark\">{}</span><span>IRONBOUND<small>IRONMAN BOSSING & GEAR</small></span></a><div class=\"sidebar-rule\"></div><div class=\"nav-label\">YOUR ADVENTURE</div><nav aria-label=\"Main navigation\">{}{}</nav><div class=\"sidebar-note\"><span class=\"tiny-rune\">✦</span><p>Ironman bossing planner</p></div><div class=\"sidebar-bottom\"><div class=\"local-indicator\"><span></span>Saved on this device</div><small>Independent OSRS fan project.<br>Game assets © Jagex.</small></div></aside><main class=\"main\"><header class=\"topbar\"><span class=\"breadcrumb\">Your journal <span>/</span> {}</span><div class=\"account-pill\">{}<span>{}<small>{}</small></span></div></header>",image(2347),nav(("bossing","Bossing",4151)),nav(("account","Your account",1712)),match s.tab.as_str(){"goals"=>"Progression goals","account"=>"Your account",_=>"Bossing"},image(10828),esc(&snapshot.selected),if demo{"Example Ironman"}else{"Imported account"});
    if demo{html.push_str(&format!("<div class=\"demo-banner\"><p>{}</p>{}</div>","Example bank",if s.demo_return.is_some(){button("restore-account","Return to your account","text-button")}else{button("tab:account","Import your bank","button gold")}));}
    if !s.message.is_empty(){html.push_str(&format!("<div role=\"status\" class=\"notice {}\">{}</div>",if s.error{"error"}else{""},esc(&s.message)));}
    html.push_str("<div class=\"page-content\">");
    if s.tab=="account"{html.push_str(&account_page(&s));}
    else if s.shared_setup.is_none()&&!planner::missing_combat_levels(snapshot).is_empty(){html.push_str(&format!("<section class=\"page-heading level-gate\"><h1>Add your combat levels</h1><p>Your bank is loaded. Combat levels are needed before a setup can be recommended.</p>{}</section>",button("tab:account","Add levels or look up your character","button gold")));}
    else{
        html.push_str(&encounter_detail(&s,&plan));
        html.push_str("<div class=\"encounter-actions\">");
        html.push_str(&format!("<details class=\"encounter-picker\" {}><summary>Choose boss or raid</summary>",if s.building_group||(options.boss==Boss::Custom&&options.targets.is_empty()){"open"}else{""}));
        html.push_str(&encounter_catalog(&s));
        html.push_str(&format!("<details class=\"custom-builder\" {}><summary>Build a custom group</summary>",if s.building_group{"open"}else{""}));
        html.push_str(&custom_group(&s));
        html.push_str(&button("custom-done","Done","button ghost"));
        html.push_str("</details>");
        html.push_str("</details>");
        html.push_str(&raid_rooms(&s));
        html.push_str("</div>");
        if s.shared_setup.is_some(){html.push_str(&format!("<p class=\"notice\">Viewing a shared setup. Your saved bank is unchanged. {}</p>",button("use-bank","Rebuild using my items","text-button")));}
        if s.shared_setup.is_none(){html.push_str(&switch_controls(&s));}
        html.push_str("<nav class=\"planner-tabs\" aria-label=\"Planner sections\">");
        for (view,label) in [("loadout","Gear"),("combat","Combat"),("prepare","Preparation")] {let disabled=if view=="combat"&&s.shared_setup.is_some(){"disabled title=\"Rebuild using your items to calculate combat stats\""}else{""};html.push_str(&format!("<button class=\"workspace-tab {}\" data-action=\"view:{view}\" aria-pressed=\"{}\" {disabled}>{label}{}</button>",if s.view==view{"active"}else{""},s.view==view,if view=="prepare"&&!plan.missing.is_empty(){format!(" <span class=\"issue-count\">{}</span>",plan.missing.len())}else{String::new()}));}
        html.push_str("</nav><section class=\"workspace-view\">");
        match s.view.as_str(){
            "combat"=>{html.push_str("<div class=\"analysis-layout\">");html.push_str(&combat_panel(&s,&plan));html.push_str(&switch_panel(&s,&plan));html.push_str("</div>");},
            "prepare"=>{html.push_str("<div class=\"prepare-layout\">");html.push_str(&checks_panel(&plan));if !plan.missing.is_empty(){html.push_str("<section class=\"panel\"><div class=\"panel-heading\"><h2>Missing trip items</h2></div><ul class=\"missing-list\">");for missing in &plan.missing{html.push_str(&format!("<li>{}</li>",esc(missing)));}html.push_str("</ul></section>");}if s.shared_setup.is_none(){html.push_str(&next_goal(&s));}html.push_str("</div>");},
            _=>{html.push_str("<div class=\"loadout-layout\">");html.push_str(&loadout_panel(&s,&plan));html.push_str(&inventory_panel(&s,&plan));html.push_str("</div>");}
        }
        html.push_str("</section><div class=\"plan-actions\">");html.push_str(&export_panel(&s,&plan));html.push_str("</div>");
        if s.view=="loadout" && s.shared_setup.is_none(){html.push_str("<details class=\"secondary-section\"><summary>All owned gear</summary>");html.push_str(&tier_table(&s));html.push_str("</details>");}
    }
    html.push_str("</div><footer><span>Ironbound — an unofficial OSRS fan site.</span><a href=\"https://oldschool.runescape.wiki/\" target=\"_blank\" rel=\"noreferrer\">OSRS Wiki ↗</a><a href=\"./assets/osrs/NOTICE.txt\" target=\"_blank\" rel=\"noreferrer\">Asset credits</a><span>Starter catalog · 9 Nov 2026</span></footer></main></div>");
    if s.edit_equipment.is_some(){html.push_str(&gear_picker(&s));}
    if let Some(index)=s.edit_inventory{html.push_str(&inventory_editor(&s,index));}
    if let Some(el)=document().get_element_by_id("app"){el.set_inner_html(&html);}
    if s.edit_equipment.is_some(){if let Ok(Some(main))=document().query_selector("main"){let _=main.set_attribute("inert","");}if let Some(search)=document().get_element_by_id("gear-search").and_then(|e|e.dyn_into::<web_sys::HtmlElement>().ok()){let _=search.focus();}}
    if s.edit_inventory.is_some(){if let Ok(Some(main))=document().query_selector("main"){let _=main.set_attribute("inert","");}if let Some(search)=document().get_element_by_id("inventory-search").and_then(|e|e.dyn_into::<web_sys::HtmlElement>().ok()){let _=search.focus();}}
});
}

fn encounter_detail(s: &State, p: &Plan) -> String {
    let boss = s.saved.options.boss;
    let guide = selected_encounter(&s.saved.options)
        .map(|entry| {
            format!(
                "https://oldschool.runescape.wiki/w/{}/Strategies",
                entry.name.replace(' ', "_")
            )
        })
        .unwrap_or_else(|| boss.source().into());
    let mut html = format!(
        "<section class=\"encounter-detail planner-summary\"><h1>{}</h1><div class=\"detail-facts\"><span>Gear: {} bag slots</span><a href=\"{}\" target=\"_blank\" rel=\"noreferrer\">Wiki guide ↗</a></div>",
        encounter_label(&s.saved.options),
        p.switches,
        esc(&guide)
    );
    let coverage = if s.shared_setup.is_some() {
        "Shared packing list · levels not included"
    } else if boss == Boss::Toa
        || selected_encounter(&s.saved.options)
            .is_some_and(|e| e.coverage().contains("DPS partial"))
    {
        "Gear plan · some damage calculations unavailable"
    } else if boss == Boss::Custom && selected_encounter(&s.saved.options).is_none() {
        "Custom gear plan · mechanics need review"
    } else {
        ""
    };
    if !coverage.is_empty() {
        html.push_str(&format!("<p class=\"plan-coverage\">{coverage}</p>"));
    }
    if boss == Boss::Toa {
        html.push_str(&format!("<label class=\"raid-control\">Raid level <input id=\"raid-level\" type=\"number\" min=\"0\" max=\"150\" value=\"{}\"><span>Mechanic-specific invocation planning is outside this starter model.</span></label>",s.saved.options.raid_level));
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
        "<div class=\"item-slot {class} {}\" title=\"{}\" aria-label=\"{}\">{}{}<span class=\"slot-caption\">{}</span></div>",
        if id.is_some() { "filled" } else { "empty" },
        esc(&title),
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
fn loadout_panel(s: &State, p: &Plan) -> String {
    let current = s.phase.and_then(|i| p.phases.get(i));
    let set = current.map(|p| &p.equipment).unwrap_or(&p.equipment);
    let mut html = format!(
        "<section class=\"panel loadout-panel\"><div class=\"panel-heading\"><h2>{} {}</h2>{}</div><div class=\"loadout-content\"><div class=\"equipment-paper\"><div class=\"paper-header\">{}</div><div class=\"equipment-grid\">",
        image(12954),
        if current.is_some() {
            "Phase preview"
        } else {
            "Equipment"
        },
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
        html.push_str(&format!(r#"<button class="equipment-edit {class}" data-action="edit-equipment:{slot_index}" data-tooltip="{}" aria-label="Change {}">{}</button>"#,esc(&tooltip),esc(&tooltip),slot(set[slot_index],planner::SLOTS[slot_index],"",None)));
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
            "<div class=\"spell-line\">{}<span>Selected spell<strong>{}</strong></span></div>",
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
    html.push_str("</details></div><div class=\"phase-tabs\">");
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
    html.push_str("</div>");
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
        html.push_str(&format!("<button class=\"inventory-edit\" data-action=\"edit-inventory:{i}\" aria-label=\"Inventory slot {}\" {}>",i+1,if phase.is_some() { "disabled" } else { "" }));
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
    html.push_str(&format!("<details class=\"supply-settings\"><summary>Adjust supplies</summary><label>Food<input id=\"food\" type=\"number\" min=\"0\" max=\"20\" value=\"{}\"></label><label>Prayer / restores<input id=\"potions\" type=\"number\" min=\"0\" max=\"12\" value=\"{}\"></label><div class=\"small-note\">{} slots free</div></details></div>",s.saved.options.food,s.saved.options.prayer_potions,28usize.saturating_sub(occupied)));
    if !p.missing.is_empty() {
        html.push_str("<div class=\"missing-list\"><strong>Before you set out</strong><ul>");
        for missing in &p.missing {
            html.push_str(&format!("<li>{}</li>", esc(missing)));
        }
        html.push_str("</ul></div>");
    }
    html.push_str("<details class=\"assumptions\"><summary>Trip notes</summary><ul>");
    for w in &p.warnings {
        html.push_str(&format!("<li>{}</li>", esc(w)));
    }
    html.push_str(&format!("<li>{} unique item IDs observed; {} match the reviewed gear priorities. Other items remain in your snapshot.</li></ul></details></section>",p.owned,p.supported));
    html
}
fn checks_panel(p: &Plan) -> String {
    let mut html = format!(
        "<section class=\"panel readiness\"><div class=\"panel-heading\"><h2>Preparation check</h2><span class=\"mini-star\">✦</span></div><div class=\"readiness-status\">{}</div><div class=\"check-list\">",
        badge(
            p.status(),
            if p.status() == "Meets starter guidelines" {
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
fn encounter_catalog(s: &State) -> String {
    let mut html = format!(
        r#"<section class="encounter-browser"><h2>Choose an encounter</h2><label><span class="slot-caption">Search encounters</span><input id="encounter-search" type="search" placeholder="Search bosses or raids" value="{}" autocomplete="off"></label><nav class="encounter-filters" aria-label="Encounter categories">"#,
        esc(&s.encounter_query)
    );
    for filter in ["Popular", "Bosses", "Raids", "Groups"] {
        html.push_str(&format!(r#"<button id="enc-filter-{filter}" class="encounter-filter {}" data-action="encounter-filter:{filter}" aria-pressed="{}">{filter}</button>"#,if s.encounter_filter==filter{"active"}else{""},s.encounter_filter==filter));
    }
    html.push_str("</nav><div id=\"encounter-results\">");
    html.push_str(&encounter_results(s));
    html.push_str("</div></section>");
    html
}
fn encounter_results(s: &State) -> String {
    let query = s.encounter_query.trim().to_lowercase();
    let popular = &crate::encounters::browser_catalog().popular;
    let mut entries = crate::encounters::catalog()
        .iter()
        .enumerate()
        .filter(|(_, e)| {
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
                && if query.is_empty() && s.encounter_filter == "Popular" {
                    popular.contains(&e.name)
                } else {
                    s.encounter_filter == "Popular" || e.category() == s.encounter_filter
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
    let mut html = String::new();
    for (index, e) in entries {
        let label = if e.name == "Barrows Chests" {
            "Barrows"
        } else {
            &e.name
        };
        let coverage = if e.targets.is_empty() {
            "Not supported yet"
        } else if e.coverage().contains("DPS partial") {
            "Gear plan · partial DPS"
        } else {
            "Attack estimates"
        };
        html.push_str(&format!(r#"<button class="encounter-choice" data-action="choose-encounter:{index}" {}><span><strong>{}</strong><small>{coverage}</small></span><span aria-hidden="true">→</span></button>"#,if e.targets.is_empty(){"disabled"}else{""},esc(label)));
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
    if let Some(results) = document().get_element_by_id("encounter-results") {
        results.set_inner_html(&encounter_results(s));
    }
    for filter in ["Popular", "Bosses", "Raids", "Groups"] {
        if let Some(el) = document().get_element_by_id(&format!("enc-filter-{filter}")) {
            el.set_class_name(if s.encounter_filter == filter {
                "encounter-filter active"
            } else {
                "encounter-filter"
            });
            let _ = el.set_attribute(
                "aria-pressed",
                if s.encounter_filter == filter {
                    "true"
                } else {
                    "false"
                },
            );
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
        .map(|(slot, _)| if slot == 13 { 100 } else { 1 })
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
    match s.saved.options.boss {
        Boss::Barrows => vec![1673, 1674, 1676, 1677, 1675, 1672],
        Boss::Toa => vec![11778, 11719, 11730, 11789],
        Boss::Custom => s.saved.options.targets.iter().map(|t| t.id).collect(),
    }
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
    let mut html = String::from(
        "<section class=\"sustain-panel\"><h3>Damage taken & prayer</h3><div class=\"sustain-controls\">",
    );
    html.push_str("<label>Protection prayer<select id=\"protection\">");
    for value in ["None", "Melee", "Ranged", "Magic"] {
        html.push_str(&format!(
            "<option {}>{value}</option>",
            if value == settings.protection {
                "selected"
            } else {
                ""
            }
        ));
    }
    html.push_str("</select></label><label>Stat prayer<select id=\"offensive-prayer\">");
    for prayer in crate::prayer::prayers() {
        html.push_str(&format!(
            "<option {} {}>{}</option>",
            if prayer.name == settings.offensive {
                "selected"
            } else {
                ""
            },
            if crate::prayer::eligible(member, prayer) {
                ""
            } else {
                "disabled"
            },
            esc(&prayer.name)
        ));
    }
    html.push_str(&format!(
        "</select></label><label><input id=\"preserve\" type=\"checkbox\" {}> Preserve</label>",
        if settings.preserve { "checked" } else { "" }
    ));
    let choices = crate::defence::attacks(target);
    if !choices.is_empty() {
        html.push_str("<label>Incoming attack scenario<select id=\"incoming-attack\">");
        for choice in &choices {
            html.push_str(&format!(
                "<option {}>{}</option>",
                if incoming_choice(s, target).as_deref() == Some(choice) {
                    "selected"
                } else {
                    ""
                },
                esc(choice)
            ));
        }
        html.push_str("</select></label>");
    }
    html.push_str("</div>");
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
                    &p.spell,
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
    for candidate in crate::rules::STYLES {
        html.push_str(&format!("<button class=\"phase {}\" data-action=\"edit-style:{candidate}\" aria-pressed=\"{}\">{candidate}</button>",if candidate == style { "active" } else { "" },candidate == style));
    }
    html.push_str(r#"<label>Against<select id="compare-target">"#);
    let mut priority_ids = std::collections::BTreeSet::new();
    for id in encounter_targets(s)
        .into_iter()
        .filter(|id| priority_ids.insert(*id))
    {
        if let Some(m) = planner::monsters().iter().find(|m| m.id == id) {
            html.push_str(&format!(
                r#"<option value="{id}" {}>{}</option>"#,
                if comparison_monster(s).is_some_and(|m| m.id == id) {
                    "selected"
                } else {
                    ""
                },
                esc(&m.name)
            ));
        }
    }
    html.push_str("</select></label>");
    if let (Some(m), Some(target)) = (s.saved.snapshot.selected(), comparison_monster(s)) {
        if crate::guidance::profile(&s.saved.options)
            .is_some_and(|profile| profile.recommended_mark_darkness)
        {
            html.push_str(&format!("<label><input id=\"mark-darkness\" type=\"checkbox\" {}> Mark of Darkness active</label>", if s.saved.options.combat.mark_darkness { "checked" } else { "" }));
        }
        match combat::calculate_configured(m,&set,style,target,&p.spell,s.saved.options.raid_level,&s.saved.options.combat){Ok(r)=>html.push_str(&format!(r#"<dl class="combat-values"><div><dt>Outgoing DPS</dt><dd>{:.3}</dd></div><div><dt>Hit chance</dt><dd>{:.1}%</dd></div><div><dt>Max hit</dt><dd>{}</dd></div><div><dt>Attack speed</dt><dd>{} ticks</dd></div></dl><p>{} attacks · {:.1} seconds per attack</p>"#,r.dps,r.accuracy*100.0,r.max_hit,r.attack_ticks,esc(&r.attack_type),r.attack_ticks as f64*0.6)),Err(e)=>html.push_str(&format!("<p class=\"combat-unavailable\">DPS unavailable: {}</p>",esc(&e)))}
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
    html.push_str("</tbody></table></details><details><summary>DPS assumptions</summary><p class=\"small-note\">Unboosted levels with the selected stat prayer; no specials or recoil. ToA assumes solo, path level 0, raid levels 0–150. Bolt effects use full target HP without Kandarin diary bonuses. Outgoing DPS covers uninterrupted attacks while vulnerable. Incoming scenarios are separate and their coverage notes apply. Movement, special-phase uptime, curses and eating are not a full encounter simulation. Unsupported mechanics show no DPS.</p></details></div></section>");
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
    let spell = planner::plan(&s.saved.snapshot, &s.saved.options).spell;
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
                                &spell,
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
fn switch_controls(s: &State) -> String {
    if s.shared_setup.is_some() {
        return String::new();
    }
    let mut html = format!(
        r#"<section class="flow-controls" aria-label="Switching preferences"><label>Switch limit<input id="max-switches" type="number" min="0" max="11" step="1" value="{}" placeholder="No limit" title="Leave empty for no limit"></label><details id="switch-preferences" {}><summary>Options</summary><div class="switch-preferences-fields"><label>Gear slots in bag<input id="switch-slots" type="number" min="0" max="20" value="{}"></label><label>Prioritize<select id="focus-style">"#,
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
    html.push_str("</details>");
    let recommended = crate::recommendation::enabled(&s.saved.options);
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
    html.push_str("</details></div></section>");
    html
}

fn switch_panel(s: &State, p: &Plan) -> String {
    if s.shared_setup.is_some() {
        return String::new();
    }
    let mut html = String::from(
        "<section class=\"panel switch-panel\"><div class=\"panel-heading\"><h2>Switch comparison</h2></div><div class=\"combat-body\"><details><summary>Phase priorities</summary><p>1 is normal. Increase to favour a phase; 0 gives it the lowest priority.</p>",
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
                && !s.saved.options.excluded.contains(&g.id)
                && (slot != 13 || owned.get(&g.id).copied().unwrap_or(0) >= 100);
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
                        owned.get(id).copied().unwrap_or(0) >= 100 && planner::can_use(*id, member)
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
                        crate::rules::spell_name(member, &trial, &owned),
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
                    &p.spell,
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
