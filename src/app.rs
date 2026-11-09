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
    tab: String,
    phase: Option<usize>,
    message: String,
    error: bool,
    busy: bool,
    export_text: Option<String>,
    shared_setup: Option<share::Setup>,
    edit_inventory: Option<usize>,
    edit_style: Option<String>,
    compare_target: Option<i32>,
    edit_equipment: Option<usize>,
    gear_search: String,
    gear_sort: String,
    incoming_attack: Option<String>,
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
            shared_setup,
            edit_inventory: None,
            edit_style: None,
            compare_target: None,
            edit_equipment: None,
            gear_search: String::new(),
            gear_sort: "dps".into(),
            incoming_attack: None,
            saved,
            tab: "bossing".into(),
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
            .get(&format!(
                "{}:{}",
                self.saved.snapshot.selected,
                self.saved.options.boss.key()
            ))
            .cloned()
            .unwrap_or_default()
    }
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
                act(&action);
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
            if el.id() == "gear-search" {
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
    let escape = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        if js_sys::Reflect::get(e.as_ref(), &JsValue::from_str("key"))
            .ok()
            .and_then(|v| v.as_string())
            .as_deref()
            == Some("Escape")
        {
            STATE.with(|s| s.borrow_mut().edit_equipment = None);
            render();
        }
    });
    document().add_event_listener_with_callback("keydown", escape.as_ref().unchecked_ref())?;
    escape.forget();
    let change = Closure::<dyn FnMut(web_sys::Event)>::new(|e: web_sys::Event| {
        let Some(el) = e.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let id = el.id();
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
        // Draft form fields must survive blur: they are committed by their action button.
        if ![
            "gear-sort",
            "max-switches",
            "switch-slots",
            "focus-style",
            "compare-target",
            "incoming-attack",
            "protection",
            "offensive-prayer",
            "preserve",
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
            "inventory-item",
            "inventory-quantity",
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
            _ if id.starts_with("priority-")=>{if let(Ok(target),Ok(weight))=(id[9..].parse::<i32>(),input(&id).parse::<u32>()){if encounter_targets(&s).contains(&target){s.saved.options.phase_priorities.insert(target,weight.min(10));}}},
            "gear-sort"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.gear_sort=sel.value();}},
            "incoming-attack"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.incoming_attack=Some(sel.value());}},
            "protection"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){let value=sel.value();if ["None","Melee","Ranged","Magic"].contains(&value.as_str()){s.saved.options.combat.protection=value;}}},
            "offensive-prayer"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){let value=sel.value();if crate::prayer::prayers().iter().any(|p|p.name==value){s.saved.options.combat.offensive=value;}}},
            "preserve"=>s.saved.options.combat.preserve=el.dyn_ref::<HtmlInputElement>().is_some_and(|e|e.checked()),
            "safety"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.saved.options.combat.safety=sel.value().parse::<u32>().unwrap_or(1).min(3);}},
            "prayer-weight"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.saved.options.combat.prayer_weight=sel.value().parse::<u32>().unwrap_or(0).min(3);}},
            "max-switches"=>{if let Ok(sel)=el.dyn_into::<HtmlSelectElement>(){s.saved.options.max_switches=sel.value().parse::<usize>().ok().map(|n|n.min(11));s.saved.options.fewer_switches=false;s.saved.options.inventory_overrides.clear();s.saved.options.inventory_quantities.clear();}},
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
            "inventory-quantity"=>{if let Some(index)=s.edit_inventory{if let Ok(q)=input("inventory-quantity").parse::<u32>(){s.saved.options.inventory_quantities.insert(index,q);}}},
            "inventory-item"=>{if let (Some(index),Ok(sel))=(s.edit_inventory,el.dyn_into::<HtmlSelectElement>()){if let Ok(value)=sel.value().parse::<i32>(){if value>0{s.saved.options.inventory_overrides.insert(index,value);s.edit_inventory=None;}}}},
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
fn act(action: &str) {
    match action {
        "load-levels" => {
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
                    let _ = window()
                        .history()
                        .and_then(|h| h.replace_state_with_url(&JsValue::NULL, "", Some(&url)));
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
        } else if action == "load-encounter" {
            let name = input("encounter-search");
            if let Some(entry) = crate::encounters::catalog()
                .iter()
                .find(|e| e.name.eq_ignore_ascii_case(name.trim()))
            {
                if entry.targets.is_empty() {
                    s.notice(&entry.note, true);
                } else {
                    s.saved.options.boss = match entry.name.as_str() {
                        "Barrows Chests" => Boss::Barrows,
                        "Tombs of Amascut" => Boss::Toa,
                        _ => Boss::Custom,
                    };
                    s.saved.options.targets = entry.targets.clone();
                    s.saved.options.gear_overrides.clear();
                    s.saved.options.inventory_overrides.clear();
                    s.saved.options.inventory_quantities.clear();
                    s.shared_setup = None;
                    s.phase = None;
                    s.edit_style = None;
                    s.compare_target = None;
                    s.edit_equipment = None;
                    s.notice(&entry.note, false);
                }
            } else {
                s.notice("Choose a listed boss, raid or encounter.", true);
            }
        } else if let Some(id) = action
            .strip_prefix("goal-item:")
            .and_then(|v| v.parse::<i32>().ok())
        {
            if planner::gear(id).is_some() {
                let key = format!(
                    "{}:{}",
                    s.saved.snapshot.selected,
                    s.saved.options.boss.key()
                );
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
            s.saved
                .options
                .gear_overrides
                .remove(&format!("{style}:{slot}"));
            s.edit_equipment = None;
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
                        let key = format!(
                            "{}:{slot}",
                            if s.saved.options.fewer_switches
                                && [0, 1, 2, 9, 10, 12].contains(&slot)
                            {
                                planner::primary_style(&s.saved.options)
                            } else {
                                parts[0]
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
            s.edit_inventory = i.parse().ok();
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
            s.saved.snapshot = account::demo();
            s.saved.options = Options::default();
            s.tab = "bossing".into();
            s.phase = None;
            s.notice("Example account loaded locally.", false);
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
            let selected = format!(
                "{}:{}",
                s.saved.snapshot.selected,
                s.saved.options.boss.key()
            );
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
                STATE.with(|state|{let mut s=state.borrow_mut();
                if s.saved.snapshot.source!="Demo" {if let Some(old)=s.saved.snapshot.selected(){snapshot.members[0].skills=old.skills.clone();snapshot.members[0].level_overrides=old.level_overrides.clone();snapshot.members[0].last_updated=old.last_updated.clone();snapshot.members[0].name=old.name.clone();snapshot.selected=old.name.clone();}}
                s.saved.snapshot=snapshot;s.shared_setup=None;s.saved.options.access_override=None;s.saved.options.started_barrows=None;s.saved.options.gear_overrides.clear();s.tab="bossing".into();s.phase=None;s.persist();s.notice("Bank imported on this device. Enter or look up your levels to build a setup.",false);
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
            s.saved = saved;
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
            s.saved.snapshot = snapshot;
            s.saved.options = Options::default();
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
    STATE.with(|state|{let s=state.borrow();let snapshot=&s.saved.snapshot;let options=&s.saved.options;let plan=s.plan();let m=snapshot.selected();
    let demo=snapshot.source=="Demo";
    let nav=|(key,label,icon):(&str,&str,i32)|format!("<button class=\"nav-item {}\" data-action=\"tab:{key}\">{}<span>{label}</span>{}</button>",if s.tab==key{"active"}else{""},image(icon),if key=="goals"{format!("<span class=\"nav-count\">{}</span>",s.pins().len())}else{String::new()});
    let mut html=format!("<div class=\"app-shell\"><aside class=\"sidebar\"><a class=\"brand\" href=\"#\" aria-label=\"Ironbound home\"><span class=\"brand-mark\">{}</span><span>IRONBOUND<small>IRONMAN BOSSING & GEAR</small></span></a><div class=\"sidebar-rule\"></div><div class=\"nav-label\">YOUR ADVENTURE</div><nav aria-label=\"Main navigation\">{}{}</nav><div class=\"sidebar-note\"><span class=\"tiny-rune\">✦</span><p>Ironman bossing planner</p></div><div class=\"sidebar-bottom\"><div class=\"local-indicator\"><span></span>Saved on this device</div><small>Independent OSRS fan project.<br>Game assets © Jagex.</small></div></aside><main class=\"main\"><header class=\"topbar\"><span class=\"breadcrumb\">Your journal <span>/</span> {}</span><div class=\"account-pill\">{}<span>{}<small>{}</small></span></div></header>",image(2347),nav(("bossing","Bossing",4151)),nav(("account","Your account",1712)),match s.tab.as_str(){"goals"=>"Progression goals","account"=>"Your account",_=>"Bossing"},image(10828),esc(&snapshot.selected),if demo{"Example Ironman"}else{"Imported account"});
    if demo{html.push_str(&format!("<div class=\"demo-banner\"><span>✦</span><p>You're exploring a sample account. <strong>Bring your own bank to the adventure.</strong></p>{}</div>",button("tab:account","Import your bank <span aria-hidden=\"true\">→</span>","text-button")));}
    if !s.message.is_empty(){html.push_str(&format!("<div role=\"status\" class=\"notice {}\">{}</div>",if s.error{"error"}else{""},esc(&s.message)));}
    html.push_str("<div class=\"page-content\">");
    if s.tab=="account"{html.push_str(&account_page(&s));}
    else{
        html.push_str("<div class=\"page-heading\"><div><div class=\"eyebrow\">PREPARE FOR YOUR NEXT CHALLENGE</div><h1>Ironman Bossing</h1><p>Choose a boss to find a setup from your own bank.</p></div><span class=\"journal-stamp\">IRONMAN<br><b>FIELD NOTES</b></span></div>");
        if let Some(m)=m{html.push_str("<div class=\"skill-strip\">");for (skill,icon) in [("Attack",1215),("Strength",4587),("Defence",12954),("Ranged",9185),("Magic",1381),("Prayer",2434)]{html.push_str(&format!("<div class=\"skill-stat\">{}<span><small>{skill}</small><b>{}</b></span></div>",image(icon),m.level(skill).map(|l|l.to_string()).unwrap_or("?".into())));}html.push_str("</div>");}
        html.push_str("<div class=\"section-line\"><h2>Choose your encounter</h2><span>Presets or your own group</span></div><div class=\"encounter-grid\">");
        for (boss,icon,kicker,description) in [(Boss::Barrows,4708,"BOSS ENCOUNTER","Equipment and supplies for the six brothers."),(Boss::Toa,25979,"RAID · STARTER PLAN","Starter equipment for raid levels 0–150.")]{let mut opt=options.clone();opt.boss=boss;if boss!=options.boss{opt.access_override=None;opt.started_barrows=None;}let p=planner::plan(snapshot,&opt);html.push_str(&format!("<button class=\"encounter {}\" data-action=\"boss:{}\"><div class=\"encounter-icon\">{}</div><div><small>{kicker}</small><h3>{}</h3><p>{description}</p>{}</div><span class=\"select-dot\"></span></button>",if boss==options.boss{"selected"}else{""},boss.key(),image(icon),boss.name(),badge(p.status(),if p.status()=="Meets starter guidelines"{"green"}else{"amber"})));}
        html.push_str("</div>");
        html.push_str(&encounter_catalog());
        html.push_str(&button("boss:custom","Build a custom encounter group","button ghost"));
        if options.boss==Boss::Custom {html.push_str(&custom_group(&s));}
        if s.shared_setup.is_some(){html.push_str(&format!("<p class=\"notice\">Viewing a shared setup. Your saved bank is unchanged. {}</p>",button("use-bank","Rebuild using my items","text-button")));}
        html.push_str("<div class=\"workspace-grid\"><div class=\"primary-column\">");
        html.push_str(&encounter_detail(&s,&plan));html.push_str(&combat_panel(&s,&plan));html.push_str(&switch_panel(&s,&plan));html.push_str(&loadout_panel(&s,&plan));html.push_str(&inventory_panel(&s,&plan));
        html.push_str("</div><aside class=\"right-column\">");html.push_str(&checks_panel(&plan));html.push_str(&export_panel(&s,&plan));if s.shared_setup.is_none(){html.push_str(&next_goal(&s));}html.push_str("</aside></div>");if s.shared_setup.is_none(){html.push_str(&tier_table(&s));}
    }
    html.push_str("</div><footer><span>Ironbound — an unofficial OSRS fan site.</span><a href=\"https://oldschool.runescape.wiki/\" target=\"_blank\" rel=\"noreferrer\">OSRS Wiki ↗</a><span>Starter catalog · 9 Nov 2026</span></footer></main></div>");
    if s.edit_equipment.is_some(){html.push_str(&gear_picker(&s));}
    if let Some(el)=document().get_element_by_id("app"){el.set_inner_html(&html);}
    if s.edit_equipment.is_some(){if let Some(search)=document().get_element_by_id("gear-search").and_then(|e|e.dyn_into::<web_sys::HtmlElement>().ok()){let _=search.focus();}}
});
}

fn encounter_detail(s: &State, p: &Plan) -> String {
    let boss = s.saved.options.boss;
    let helmets = if boss == Boss::Barrows {
        [4716, 4724, 4732, 4745, 4753, 4708]
            .into_iter()
            .map(image)
            .collect::<String>()
    } else {
        [25979, 9185, 11905, 1215]
            .into_iter()
            .map(image)
            .collect::<String>()
    };
    let mut html = format!(
        "<section class=\"encounter-detail\"><div class=\"encounter-detail-top\"><div><div class=\"eyebrow\">YOUR ENCOUNTER PLAN</div><h2>{}</h2><p>{}</p></div><div class=\"brothers\">{helmets}</div></div><div class=\"detail-facts\"><span>✦ {} gear items matched</span><span>◈ {} carried switches</span><a href=\"{}\" target=\"_blank\" rel=\"noreferrer\">Strategy guide ↗</a></div>",
        encounter_label(&s.saved.options),
        if boss == Boss::Barrows {
            "A magic-first setup, with a ranged switch for Ahrim."
        } else if boss == Boss::Custom {
            "One shared inventory for your selected monsters and styles."
        } else {
            "A starter packing plan for raid levels 0–150."
        },
        p.supported,
        p.switches,
        boss.source()
    );
    if boss == Boss::Toa {
        html.push_str(&format!("<label class=\"raid-control\">Raid level <input id=\"raid-level\" type=\"number\" min=\"0\" max=\"150\" value=\"{}\"><span>Mechanic-specific invocation planning is outside this starter model.</span></label>",s.saved.options.raid_level));
    }
    html.push_str("</section>");
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
        "<section class=\"panel loadout-panel\"><div class=\"panel-heading\"><h2>{} Your equipment</h2>{}</div><div class=\"loadout-content\"><div class=\"equipment-paper\"><div class=\"paper-header\">{}</div><div class=\"equipment-grid\">",
        image(12954),
        badge(
            if s.shared_setup.is_some() {
                "SHARED SETUP"
            } else {
                "FROM YOUR BANK"
            },
            "neutral"
        ),
        current
            .map(|p| esc(&p.name))
            .unwrap_or("Starting equipment".into())
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
    html.push_str("</div><small>Hover over an item for its name</small></div><div class=\"loadout-notes\"><h3>Suggested equipment</h3><p>Reviewed starter gear priorities, matched against your bank and levels. Compare owned alternatives in the equipment tiers below.</p>");
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
    html.push_str(&button(
        "casual",
        "Casual preset: 4-item switches",
        "button ghost full",
    ));
    html.push_str(
        "<details><summary>Matched equipment & exclusions</summary><div class=\"gear-list\">",
    );
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
    html.push_str("</div></div><div class=\"phase-tabs\">");
    html.push_str(&button(
        "starting",
        "Starting gear",
        if s.phase.is_none() {
            "phase active"
        } else {
            "phase"
        },
    ));
    for (i, phase) in p.phases.iter().enumerate() {
        let label = if s.saved.options.boss == Boss::Barrows {
            if i == 0 { "Five brothers" } else { "Ahrim" }
        } else {
            &phase.name
        };
        html.push_str(&button(
            &format!("phase:{i}"),
            &esc(label),
            if s.phase == Some(i) {
                "phase active"
            } else {
                "phase"
            },
        ));
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
    let mut html = format!(
        "<section class=\"panel\"><div class=\"panel-heading\"><h2>{} Your packing list</h2><span class=\"slot-count {}\">{} / 28 slots</span></div><div class=\"packing-body\"><div class=\"inventory-grid\">",
        image(385),
        if p.inventory.len() > 28 { "over" } else { "" },
        p.inventory.len()
    );
    for i in 0..28 {
        let item = p.inventory.get(i);
        html.push_str(&format!("<button class=\"inventory-edit\" data-action=\"edit-inventory:{i}\" aria-label=\"Change inventory slot {}\">",i+1));
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
    html.push_str(&format!("</div><div class=\"supply-settings\"><h3>Supplies</h3><p>Adjust your starting supplies. Quantities are a trip template, not a prediction of what you'll consume.</p><label>Food slots<input id=\"food\" type=\"number\" min=\"0\" max=\"20\" value=\"{}\"></label><label>Prayer / restore bottles<input id=\"potions\" type=\"number\" min=\"0\" max=\"12\" value=\"{}\"></label><div class=\"small-note\">◈ {} slots free at the start</div></div></div>",s.saved.options.food,s.saved.options.prayer_potions,28usize.saturating_sub(p.inventory.len())));
    if let Some(i) = s.edit_inventory {
        if s.shared_setup.is_none() {
            html.push_str(&inventory_editor(s, i));
        }
    }
    if !p.missing.is_empty() {
        html.push_str("<div class=\"missing-list\"><strong>Before you set out</strong><ul>");
        for missing in &p.missing {
            html.push_str(&format!("<li>{}</li>", esc(missing)));
        }
        html.push_str("</ul></div>");
    }
    html.push_str(
        "<details class=\"assumptions\"><summary>Trip assumptions & catalog coverage</summary><ul>",
    );
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
        "<section class=\"export-card\"><div class=\"eyebrow\">READY WHEN YOU ARE</div><h2>RuneLite export</h2><p>Import your equipment and packing list into Inventory Setups.</p><button data-action=\"copy\" class=\"button gold full\" {disabled}>Copy Inventory Setup <span>↗</span></button><button data-action=\"download-setup\" class=\"button ghost full\" {disabled}>Download setup JSON</button><small>{}</small><details><summary>How to import</summary><ol><li>Enable Inventory Setups and Bank Tags in RuneLite.</li><li>Choose the green import button → Import setup.</li><li>Paste the copied text and confirm.</li></ol><p>You withdraw and equip items yourself.</p></details>",
        if p.exportable() {
            "Check staff charges and supplies before your trip."
        } else {
            "Resolve missing items or preparation blockers to export."
        }
    );
    html.push_str(&button("bank-tags", "Copy Bank Tags", "button ghost full"));
    html.push_str(&button("share", "Copy setup link", "button ghost full"));
    if let Some(text) = &s.export_text {
        html.push_str(&format!("<label class=\"export-label\">Setup text<textarea readonly rows=\"5\">{}</textarea></label>",esc(text)));
    }
    html.push_str("</section>");
    html
}
fn next_goal(s: &State) -> String {
    if s.saved.options.boss == Boss::Custom {
        return String::new();
    }
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
        r#"<div class="page-heading"><div><h1>Your Ironman</h1><p>Import your RuneLite bank. Everything in the bank stays on this device.</p></div></div><div class="account-grid"><section class="panel account-form"><h2>1. Import bank</h2><p>In RuneLite Bank Memory, export your bank as a tab-separated table. Paste it or choose the file below.</p><label>Bank table or private backup<textarea id="import-data" rows="6" placeholder="Item id    Item name    Item quantity"></textarea></label>{}<label>Choose bank file<input id="import-file" type="file" accept=".txt,.tsv,.json,text/plain,application/json"></label><p>Bank data is never put in a share link. Backups contain private account data.</p></section><section class="panel account-form"><h2>2. Combat levels</h2><label>RuneScape name (optional)<input id="player-name" maxlength="12" value="{}" autocomplete="off"></label><button class="button ghost" data-action="load-levels" {}>Look up public levels</button><p>Lookup sends only the name to Wise Old Man. Its public snapshot may be stale. Or enter your levels below.</p><small>Skill snapshot: {}</small><div class="skills-grid">"#,
        button("import", "Import bank", "button gold full"),
        esc(if s.saved.snapshot.source == "Demo" {
            ""
        } else {
            &s.saved.snapshot.selected
        }),
        if s.busy { "disabled" } else { "" },
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
        button("demo", "Explore example", "text-button"),
        button("forget", "Forget saved account", "text-button danger")
    ));
    html
}
fn selected_encounter(options: &Options) -> Option<&'static crate::encounters::Entry> {
    if options.boss != Boss::Custom {
        return None;
    }
    crate::encounters::catalog()
        .iter()
        .filter(|e| !e.targets.is_empty() && e.targets == options.targets)
        .min_by_key(|e| !crate::encounters::groups().iter().any(|g| g.name == e.name))
}
fn encounter_label(options: &Options) -> &str {
    selected_encounter(options)
        .map(|e| e.name.as_str())
        .unwrap_or(options.boss.name())
}
fn encounter_catalog() -> String {
    let mut html = String::from(
        r#"<section class="panel account-form"><h3>Boss, raid or encounter</h3><p>Browse the full boss catalog, or build a group from the monster database. Coverage notes identify packing templates and missing mechanics.</p><label>Encounter<input id="encounter-search" list="encounters" placeholder="Search bosses and raids…"></label><datalist id="encounters">"#,
    );
    for e in crate::encounters::catalog() {
        html.push_str(&format!(
            r#"<option value="{}">{}</option>"#,
            esc(&e.name),
            e.coverage()
        ));
    }
    html.push_str("</datalist>");
    html.push_str(&button("load-encounter", "Load encounter", "button ghost"));
    html.push_str("</section>");
    html
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
    let mut html = format!(
        "<label class=\"account-form\">Change inventory slot {}<select id=\"inventory-item\"><option value=\"-1\">Choose an owned item…</option>",
        index + 1
    );
    if let Some(m) = s.saved.snapshot.selected() {
        for (id, q) in s.saved.snapshot.available(false) {
            html.push_str(&format!(
                "<option value=\"{id}\">{} × {q}</option>",
                esc(&m
                    .item_names
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| planner::item_name(id)))
            ));
        }
    }
    let quantity = s
        .plan()
        .inventory
        .get(index)
        .map(|i| i.quantity)
        .unwrap_or(1);
    html.push_str(&format!("</select></label><label class=\"account-form\">Stack quantity<input id=\"inventory-quantity\" type=\"number\" min=\"1\" max=\"2147483647\" value=\"{quantity}\"></label><p class=\"small-note\">Stacks may contain multiple runes or ammunition. Other items occupy one slot each. Required trip items must remain carried.</p>"));
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
            html.push_str(&format!("<dl class=\"combat-values\"><div><dt>Incoming DPS · scenario</dt><dd>{:.3}</dd></div><div><dt>Enemy hit chance</dt><dd>{:.1}%</dd></div><div><dt>Boss healing / second</dt><dd>{}</dd></div></dl><p class=\"small-note\">{}</p>",r.damage_per_second,r.hit_chance*100.0,healing,esc(&r.note)));
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
                        html.push_str("<p class=\"combat-unavailable\">Boss healing meets or exceeds outgoing DPS in this normal-attack scenario. Try more defence, more damage, or improve the mechanics.</p>");
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
    html.push_str(&format!("<table class=\"bonus-table\"><tbody><tr><th>Prayer bonus</th><td>{bonus:+}</td></tr><tr><th>Continuous drain</th><td>{drain:.2} points / minute</td></tr><tr><th>Carried prayer supply</th><td>{}</td></tr></tbody></table><p class=\"small-note\">Supply estimate assumes full starting prayer and this equipment throughout. No flicking, regeneration, wrench bonuses or encounter-specific prayer drains. Incoming DPS covers the selected attack scenario; unmodelled chip damage and specials are not zero.</p>",esc(&duration)));
    html.push_str("<details><summary>Weigh damage, defence & longer trips</summary><label>Defence weighting<select id=\"safety\">");
    for (value, label) in [
        (0, "Damage first"),
        (1, "Balanced"),
        (2, "Safer"),
        (3, "Prioritize safety"),
    ] {
        html.push_str(&format!(
            "<option value=\"{value}\" {}>{label}</option>",
            if settings.safety == value {
                "selected"
            } else {
                ""
            }
        ));
    }
    html.push_str(
        "</select></label><label>Prayer efficiency weighting<select id=\"prayer-weight\">",
    );
    for (value, label) in [
        (0, "No preference"),
        (1, "Some preference"),
        (2, "Stronger preference"),
        (3, "Long trips"),
    ] {
        html.push_str(&format!(
            "<option value=\"{value}\" {}>{label}</option>",
            if settings.prayer_weight == value {
                "selected"
            } else {
                ""
            }
        ));
    }
    html.push_str("</select></label><p class=\"small-note\">Weights guide a bounded search, not a guaranteed trip length. Defence affects automatic ranking only for verified encounter profiles. Blood Moon healing reduces net progress even at zero defence weight. Compare gear using Encounter balance, DPS, defence or prayer bonus in the slot picker.</p></details></section>");
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
    for style in crate::rules::STYLES {
        html.push_str(&button(&format!("edit-style:{style}"), style, "phase"));
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
        match combat::calculate_configured(m,&set,style,target,&p.spell,s.saved.options.raid_level,&s.saved.options.combat){Ok(r)=>html.push_str(&format!(r#"<dl class="combat-values"><div><dt>Outgoing DPS</dt><dd>{:.3}</dd></div><div><dt>Hit chance</dt><dd>{:.1}%</dd></div><div><dt>Max hit</dt><dd>{}</dd></div><div><dt>Attack speed</dt><dd>{} ticks</dd></div></dl><p>{} attacks · {:.1} seconds per attack</p>"#,r.dps,r.accuracy*100.0,r.max_hit,r.attack_ticks,esc(&r.attack_type),r.attack_ticks as f64*0.6)),Err(e)=>html.push_str(&format!("<p class=\"combat-unavailable\">DPS unavailable: {}</p>",esc(&e)))}
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
fn switch_panel(s: &State, p: &Plan) -> String {
    if s.shared_setup.is_some() {
        return String::new();
    }
    let mut html = String::from(
        r#"<section class="panel switch-panel"><div class="panel-heading"><h2>Make switching comfortable</h2></div><div class="combat-body"><label>Maximum items changed per switch<select id="max-switches"><option value="unlimited">Unrestricted</option>"#,
    );
    for n in 0..=11 {
        html.push_str(&format!(
            r#"<option value="{n}" {}>{n} items</option>"#,
            if s.saved.options.max_switches == Some(n) {
                "selected"
            } else {
                ""
            }
        ));
    }
    html.push_str(&format!(r#"</select></label><label>Inventory slots for style-switch gear<input id="switch-slots" type="number" min="0" max="20" value="{}"></label><label>Priority<select id="focus-style">"#,s.saved.options.switch_slots));
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
    html.push_str("</select></label><p>Checks every pair of style sets, including shield displacement. Supplies, tools and optional spec weapons still need their own inventory space.</p><details><summary>Prioritize individual phases</summary><p>1 is normal; higher values favour that target. 0 gives it the lowest damage priority while retaining its required attack style.</p>");
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

fn gear_picker(s: &State) -> String {
    let Some(slot) = s.edit_equipment else {
        return String::new();
    };
    let style = comparison_style(s);
    let sort_options = [
        ("dps", "DPS"),
        ("overall", "Encounter balance"),
        ("defence", "Defence: total bonuses"),
        ("magic-def", "Defence: magic"),
        ("ranged-def", "Defence: ranged"),
        ("prayer", "Prayer bonus"),
    ]
    .into_iter()
    .map(|(key, label)| {
        format!(
            "<option value=\"{key}\" {}>{label}</option>",
            if s.gear_sort == key { "selected" } else { "" }
        )
    })
    .collect::<String>();
    format!(
        r#"<div class="picker-backdrop"><section class="gear-picker" role="dialog" aria-modal="true" aria-labelledby="gear-picker-title"><div class="panel-heading"><h2 id="gear-picker-title">{} · owned equipment</h2>{}</div><div class="picker-controls"><label>Search your items<input id="gear-search" type="search" value="{}" placeholder="Search owned items…" autocomplete="off"></label><label>Order by<select id="gear-sort">{sort_options}</select></label></div><p class="picker-note">{style} against {}. Comparisons hold your other equipment fixed. Encounter balance uses your defence and prayer weights; incoming values cover the selected attack scenario.</p><div class="picker-actions">{} {}</div><div class="picker-scroll"><div class="bis-scale">{}</div><h3 class="owned-heading">Your owned items</h3><div id="gear-pick-results" class="gear-pick-results">{}</div></div></section></div>"#,
        planner::SLOTS[slot],
        button("close-picker", "Close ×", "phase"),
        esc(&s.gear_search),
        esc(&comparison_monster(s)
            .map(|m| m.name.as_str())
            .unwrap_or("selected encounter")),
        button(&format!("auto-gear:{slot}"), "Use recommended", "phase"),
        button(
            &format!("pick-gear:{style}:{slot}:0"),
            "Leave empty",
            "phase"
        ),
        bis_scale(s),
        gear_choices(s)
    )
}
fn gear_choices(s: &State) -> String {
    let Some(slot) = s.edit_equipment else {
        return String::new();
    };
    let style = comparison_style(s);
    let p = s.plan();
    let set = planned_style(s, &p, style);
    let Some(member) = s.saved.snapshot.selected() else {
        return String::new();
    };
    let owned = s.saved.snapshot.available(false);
    let query = s.gear_search.to_lowercase();
    let mut items = planner::equipment()
        .iter()
        .filter(|g| {
            g.slot == planner::SLOT_KEYS[slot]
                && owned.contains_key(&g.id)
                && (query.is_empty() || planner::item_name(g.id).to_lowercase().contains(&query))
        })
        .map(|g| {
            let usable = planner::can_use(g.id, member)
                && (slot != 13 || owned.get(&g.id).copied().unwrap_or(0) >= 100);
            let mut trial = set.clone();
            trial[slot] = Some(g.id);
            if slot == 3 && g.two_handed {
                trial[5] = None;
            }
            let dps = if usable {
                comparison_monster(s)
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
                    .map(|r| r.dps)
            } else {
                None
            };
            let own_defence = combat::stats(g.id).map(|g| &g.defensive);
            let score = own_defence
                .map(|map| match s.gear_sort.as_str() {
                    "magic-def" => map.get("magic").copied().unwrap_or(0),
                    "ranged-def" => map.get("ranged").copied().unwrap_or(0),
                    _ => map.values().sum(),
                })
                .unwrap_or(0);
            let overall = dps.and_then(|damage| {
                comparison_monster(s).map(|target| {
                    crate::defence::score(member, &trial, target, damage, &s.saved.options.combat)
                })
            });
            let incoming = comparison_monster(s)
                .and_then(|target| {
                    incoming_choice(s, target).and_then(|attack| {
                        crate::defence::estimate(
                            member,
                            &trial,
                            target,
                            &attack,
                            &s.saved.options.combat,
                        )
                        .ok()
                    })
                })
                .map(|r| r.damage_per_second);
            let prayer_bonus = crate::prayer::bonus(&trial);
            let drain = crate::prayer::drain_per_minute(prayer_bonus, &s.saved.options.combat);
            (
                g,
                usable,
                dps,
                score,
                overall,
                incoming,
                drain,
                prayer_bonus,
            )
        })
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        if s.gear_sort == "dps" {
            b.2.partial_cmp(&a.2)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.name.cmp(&b.0.name))
        } else if s.gear_sort == "overall" {
            b.4.partial_cmp(&a.4)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.name.cmp(&b.0.name))
        } else if s.gear_sort == "prayer" {
            b.7.cmp(&a.7)
                .then_with(|| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
        } else {
            b.3.cmp(&a.3).then_with(|| a.0.name.cmp(&b.0.name))
        }
    });
    let mut html = format!("<p class=\"picker-note\">{} owned items</p>", items.len());
    if items.is_empty() {
        html.push_str("<p>No owned items match this search.</p>");
    }
    for (g, usable, dps, defence, _overall, incoming, drain, prayer_bonus) in items {
        html.push_str(&format!(r#"<button class="gear-pick {}" data-action="pick-gear:{style}:{slot}:{}" {}>{}<span><strong>{}</strong><small>{} · {defence:+} defence</small><small>Incoming {} · {prayer_bonus:+} prayer · {drain:.1} points/min</small><small>{}</small></span><b>× {}</b></button>"#,if set[slot]==Some(g.id){"selected"}else{""},g.id,if usable{""}else{"disabled"},image(g.id),esc(&planner::item_name(g.id)),dps.map(|v|format!("{v:.3} DPS")).unwrap_or_else(||"DPS unavailable".into()),incoming.map(|v|format!("{v:.3} DPS")).unwrap_or_else(||"unverified".into()),if usable{"Click to equip"}else{"Skill requirements unconfirmed or unmet"},owned[&g.id]));
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
