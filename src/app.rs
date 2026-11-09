use crate::{
    account::{self, SKILLS, Snapshot},
    export,
    planner::{self, Boss, Options, Plan},
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
}
struct State {
    saved: Saved,
    tab: String,
    phase: Option<usize>,
    message: String,
    error: bool,
    busy: bool,
    export_text: Option<String>,
}
impl State {
    fn initial() -> Self {
        let restored = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item(STORAGE).ok().flatten())
            .and_then(|s| serde_json::from_str::<Saved>(&s).ok())
            .filter(|s| {
                s.version == 1
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
        });
        saved.options.food = saved.options.food.min(20);
        saved.options.prayer_potions = saved.options.prayer_potions.min(12);
        saved.options.raid_level = saved.options.raid_level.min(150);
        Self {
            saved,
            tab: "bossing".into(),
            phase: None,
            message: String::new(),
            error: false,
            busy: false,
            export_text: None,
        }
    }
    fn persist(&self) {
        if let Some(storage) = window().local_storage().ok().flatten() {
            if storage
                .set_item(
                    STORAGE,
                    &serde_json::to_string(&self.saved).unwrap_or_default(),
                )
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
            .get(&self.saved.snapshot.selected)
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
    format!(
        "<img src=\"./assets/items/{id}.png\" alt=\"\" width=\"32\" height=\"32\" loading=\"lazy\">"
    )
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
        STATE.with(|state|{let mut s=state.borrow_mut();match id.as_str(){
            "member"=>{if let Ok(select)=el.dyn_into::<HtmlSelectElement>(){s.saved.snapshot.selected=select.value();s.saved.options.access_override=None;s.saved.options.started_barrows=None;s.phase=None;}},
            "shared"=>s.saved.options.shared=el.dyn_ref::<HtmlInputElement>().is_some_and(|e|e.checked()),
            "fewer"=>s.saved.options.fewer_switches=el.dyn_ref::<HtmlInputElement>().is_some_and(|e|e.checked()),
            "food"=>s.saved.options.food=input("food").parse::<usize>().unwrap_or(8).min(20),
            "potions"=>s.saved.options.prayer_potions=input("potions").parse::<usize>().unwrap_or(3).min(12),
            "raid-level"=>s.saved.options.raid_level=input("raid-level").parse::<u32>().unwrap_or(0).min(150),
            "access"=>{if let Ok(select)=el.dyn_into::<HtmlSelectElement>(){s.saved.options.access_override=match select.value().as_str(){"yes"=>Some(true),"no"=>Some(false),_=>None};}},
            "started"=>{if let Ok(select)=el.dyn_into::<HtmlSelectElement>(){s.saved.options.started_barrows=match select.value().as_str(){"yes"=>Some(true),"no"=>Some(false),_=>None};}},
            _ if id.starts_with("skill-")=>{if let (Ok(index),Ok(level))=(id[6..].parse::<usize>(),input(&id).parse::<u32>()){if index<24&&(1..=99).contains(&level){let selected=s.saved.snapshot.selected.clone();if let Some(m)=s.saved.snapshot.members.iter_mut().find(|m|m.name==selected){let skills=m.skills.get_or_insert(vec![0;24]);skills.resize(24,0);skills[index]=account::xp_for_level(level);s.notice("Skill correction saved locally. Refreshing GroupIron replaces imported skills.",false);}}}},
            _=>return,
        }s.export_text=None;s.persist();});
        render();
    });
    document().add_event_listener_with_callback("change", change.as_ref().unchecked_ref())?;
    change.forget();
    Ok(())
}

fn notice(message: &str, error: bool) {
    STATE.with(|s| s.borrow_mut().notice(message, error));
    render();
}
fn act(action: &str) {
    match action {
        "connect" => {
            let group = input("group-name");
            let token = input("group-token");
            if group.trim().is_empty() || token.trim().is_empty() {
                notice("Enter your GroupIron group name and group token.", true);
                return;
            }
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.busy = true;
                s.notice("Reading your GroupIron snapshot…", false);
            });
            render();
            spawn_local(connect(group, token));
            return;
        }
        "import" => {
            import_raw(&text_input("import-data"));
            return;
        }
        "copy" => {
            let content = STATE.with(|s| {
                let s = s.borrow();
                let p = planner::plan(&s.saved.snapshot, &s.saved.options);
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
        "download-setup" => {
            let result = STATE.with(|s| {
                let s = s.borrow();
                export::inventory_setup(
                    &planner::plan(&s.saved.snapshot, &s.saved.options),
                    &s.saved.options,
                    &s.saved.snapshot.selected,
                )
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
            if !window().confirm_with_message("Remove the saved account and goals from this browser? Your GroupIron account will not be changed.").unwrap_or(false){return;}
            if let Some(storage) = window().local_storage().ok().flatten() {
                let _ = storage.remove_item(STORAGE);
            }
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
        } else if let Some(boss) = action.strip_prefix("boss:") {
            s.saved.options.boss = if boss == "toa" {
                Boss::Toa
            } else {
                Boss::Barrows
            };
            s.saved.options.access_override = None;
            s.saved.options.started_barrows = None;
            s.phase = None;
        } else if let Some(i) = action.strip_prefix("phase:") {
            s.phase = i.parse().ok();
        } else if action == "starting" {
            s.phase = None;
        } else if action == "demo" {
            s.saved.snapshot = account::demo();
            s.saved.options = Options::default();
            s.tab = "bossing".into();
            s.phase = None;
            s.notice(
                "Example account loaded. Your GroupIron account has not been changed.",
                false,
            );
        } else if let Some(id) = action.strip_prefix("pin:") {
            let selected = s.saved.snapshot.selected.clone();
            let pins = s.saved.pins.entry(selected).or_default();
            if pins.iter().any(|v| v == id) {
                pins.retain(|v| v != id);
            } else if pins.len() < 3 {
                pins.push(id.into());
            } else {
                s.notice(
                    "Keep up to three active goals. Unpin one before adding another.",
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

async fn connect(group: String, token: String) {
    let result=async {
        let path=group.trim().bytes().map(|b|if b.is_ascii_alphanumeric()||b==b'-'||b==b'_' {char::from(b).to_string()}else{format!("%{b:02X}")}).collect::<String>();
        let init=web_sys::RequestInit::new();init.set_method("GET");init.set_mode(web_sys::RequestMode::Cors);init.set_redirect(web_sys::RequestRedirect::Error);
        let request=web_sys::Request::new_with_str_and_init(&format!("https://groupiron.men/api/group/{path}/get-group-data?from_time=1970-01-01T00%3A00%3A00.000Z"),&init).map_err(|_|"Could not prepare the GroupIron request.")?;
        request.headers().set("Authorization",token.trim()).map_err(|_|"The token contains unsupported characters.")?;
        let response=JsFuture::from(window().fetch_with_request(&request)).await.map_err(|_|"GroupIron could not be reached. Check connectivity and browser access; your saved account is unchanged.")?.dyn_into::<web_sys::Response>().map_err(|_|"Unexpected GroupIron response.")?;
        if !response.ok(){return Err(if response.status()==401||response.status()==400{"GroupIron rejected the group name or token. Check both and try again."}else{"GroupIron returned an error. Your saved account is unchanged."});}
        let raw=JsFuture::from(response.text().map_err(|_|"Could not read the response.")?).await.map_err(|_|"Could not read the response.")?.as_string().ok_or("Unexpected response body.")?;
        Ok(raw)
    }.await;
    // Credentials exist only in this request's memory. They are never stored or exported.
    STATE.with(|s| s.borrow_mut().busy = false);
    match result {
        Ok(raw) => import_raw(&raw),
        Err(e) => notice(e, true),
    }
}

fn import_raw(raw: &str) {
    let now = js_sys::Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default();
    if let Ok(mut saved) = serde_json::from_str::<Saved>(raw) {
        if saved.version != 1 {
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
                "Account imported. Review access, skills and your packing list.",
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
    STATE.with(|state|{let s=state.borrow();let snapshot=&s.saved.snapshot;let options=&s.saved.options;let plan=planner::plan(snapshot,options);let m=snapshot.selected();
    let demo=snapshot.source=="Demo";
    let nav=|(key,label,icon):(&str,&str,i32)|format!("<button class=\"nav-item {}\" data-action=\"tab:{key}\">{}<span>{label}</span>{}</button>",if s.tab==key{"active"}else{""},image(icon),if key=="goals"{format!("<span class=\"nav-count\">{}</span>",s.pins().len())}else{String::new()});
    let mut html=format!("<div class=\"app-shell\"><aside class=\"sidebar\"><a class=\"brand\" href=\"#\" aria-label=\"Ironbound home\"><span class=\"brand-mark\">{}</span><span>IRONBOUND<small>THE IRONMAN'S JOURNAL</small></span></a><div class=\"sidebar-rule\"></div><div class=\"nav-label\">YOUR ADVENTURE</div><nav aria-label=\"Main navigation\">{}{}{}</nav><div class=\"sidebar-note\"><span class=\"tiny-rune\">✦</span><p>Every item earned.<br>Every adventure your own.</p></div><div class=\"sidebar-bottom\"><div class=\"local-indicator\"><span></span>Saved on this device</div><small>Independent OSRS fan project.<br>Game assets © Jagex.</small></div></aside><main class=\"main\"><header class=\"topbar\"><span class=\"breadcrumb\">Your journal <span>/</span> {}</span><div class=\"account-pill\">{}<span>{}<small>{}</small></span></div></header>",image(2347),nav(("bossing","Bossing",4151)),nav(("goals","Progression goals",6570)),nav(("account","Your account",1712)),match s.tab.as_str(){"goals"=>"Progression goals","account"=>"Your account",_=>"Bossing"},image(10828),esc(&snapshot.selected),if demo{"Example Ironman"}else{"Imported account"});
    if demo{html.push_str(&format!("<div class=\"demo-banner\"><span>✦</span><p>You're exploring a sample account. <strong>Bring your own bank to the adventure.</strong></p>{}</div>",button("tab:account","Connect GroupIron <span aria-hidden=\"true\">→</span>","text-button")));}
    if !s.message.is_empty(){html.push_str(&format!("<div role=\"status\" class=\"notice {}\">{}</div>",if s.error{"error"}else{""},esc(&s.message)));}
    html.push_str("<div class=\"page-content\">");
    if s.tab=="account"{html.push_str(&account_page(&s));}
    else if s.tab=="goals"{html.push_str(&goals_page(&s));}
    else{
        html.push_str("<div class=\"page-heading\"><div><div class=\"eyebrow\">PREPARE FOR YOUR NEXT CHALLENGE</div><h1>Your bank. Your next adventure.</h1><p>Find your footing, pack your gear, and make every grind count.</p></div><span class=\"journal-stamp\">IRONMAN<br><b>FIELD NOTES</b></span></div>");
        if let Some(m)=m{html.push_str("<div class=\"skill-strip\">");for (skill,icon) in [("Attack",1215),("Strength",4587),("Defence",12954),("Ranged",9185),("Magic",1381),("Prayer",2434)]{html.push_str(&format!("<div class=\"skill-stat\">{}<span><small>{skill}</small><b>{}</b></span></div>",image(icon),m.level(skill).map(|l|l.to_string()).unwrap_or("?".into())));}html.push_str(&format!("<div class=\"skill-total\"><small>TOTAL LEVEL</small><b>{}</b></div></div>",m.total_level().map(|v|v.to_string()).unwrap_or("?".into())));}
        html.push_str("<div class=\"section-line\"><h2>Choose your encounter</h2><span>2 supported adventures</span></div><div class=\"encounter-grid\">");
        for (boss,icon,kicker,description) in [(Boss::Barrows,4708,"BOSS ENCOUNTER","Six brothers. One chest. Your first steps into PvM."),(Boss::Toa,25979,"RAID · STARTER PLAN","One shared inventory, from the paths to the Wardens.")]{let mut opt=options.clone();opt.boss=boss;if boss!=options.boss{opt.access_override=None;opt.started_barrows=None;}let p=planner::plan(snapshot,&opt);html.push_str(&format!("<button class=\"encounter {}\" data-action=\"boss:{}\"><div class=\"encounter-icon\">{}</div><div><small>{kicker}</small><h3>{}</h3><p>{description}</p>{}</div><span class=\"select-dot\"></span></button>",if boss==options.boss{"selected"}else{""},boss.key(),image(icon),boss.name(),badge(p.status(),if p.status()=="Meets starter guidelines"{"green"}else{"amber"})));}
        html.push_str("</div><div class=\"workspace-grid\"><div class=\"primary-column\">");
        html.push_str(&encounter_detail(&s,&plan));html.push_str(&loadout_panel(&s,&plan));html.push_str(&inventory_panel(&s,&plan));
        html.push_str("</div><aside class=\"right-column\">");html.push_str(&checks_panel(&plan));html.push_str(&export_panel(&s,&plan));html.push_str(&next_goal(&s));html.push_str("</aside></div>");
    }
    html.push_str("</div><footer><span>Made for the self-sufficient.</span><a href=\"https://oldschool.runescape.wiki/\" target=\"_blank\" rel=\"noreferrer\">OSRS Wiki ↗</a><a href=\"https://groupiron.men/\" target=\"_blank\" rel=\"noreferrer\">GroupIron ↗</a><span>Starter catalog · 9 Nov 2026</span></footer></main></div>");
    if let Some(el)=document().get_element_by_id("app"){el.set_inner_html(&html);}
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
        boss.name(),
        if boss == Boss::Barrows {
            "A magic-first setup, with a ranged switch for Ahrim."
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
        badge("FROM YOUR BANK", "neutral"),
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
        html.push_str(&slot(
            set[slot_index],
            planner::SLOTS[slot_index],
            class,
            None,
        ));
    }
    html.push_str("</div><small>Hover or focus gear names for details</small></div><div class=\"loadout-notes\"><h3>Made from what you have.</h3><p>Reviewed starter gear priorities, matched against your bank and levels.</p>");
    if !p.spell.is_empty() {
        html.push_str(&format!(
            "<div class=\"spell-line\">{}<span>Selected spell<strong>{}</strong></span></div>",
            image(556),
            esc(&p.spell)
        ));
    }
    html.push_str(&format!("<label class=\"toggle-row\"><input id=\"fewer\" type=\"checkbox\" {}><span>Fewer switches<small>Share accessories between styles</small></span></label><label class=\"toggle-row\"><input id=\"shared\" type=\"checkbox\" {}><span>Include group storage<small>One shared-bank observation, counted once</small></span></label>",if s.saved.options.fewer_switches{"checked"}else{""},if s.saved.options.shared{"checked"}else{""}));
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
    html.push_str("</div></details></div></div><div class=\"phase-tabs\">");
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
        html.push_str(&slot(
            item.map(|p| p.id),
            &format!("Slot {}", i + 1),
            "inventory-item",
            item.map(|p| p.quantity),
        ));
    }
    html.push_str(&format!("</div><div class=\"supply-settings\"><h3>Leave room for the journey.</h3><p>Adjust your starting supplies. Quantities are a trip template, not a prediction of what you'll consume.</p><label>Food slots<input id=\"food\" type=\"number\" min=\"0\" max=\"20\" value=\"{}\"></label><label>Prayer / restore bottles<input id=\"potions\" type=\"number\" min=\"0\" max=\"12\" value=\"{}\"></label><div class=\"small-note\">◈ {} slots free at the start</div></div></div>",s.saved.options.food,s.saved.options.prayer_potions,28usize.saturating_sub(p.inventory.len())));
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
    html.push_str(&format!("</div><p class=\"panel-footnote\">Preparation guidelines don't predict a successful kill. {} to confirm access.</p></section>",button("tab:account","Open your account","text-button")));
    html
}
fn export_panel(s: &State, p: &Plan) -> String {
    let disabled = if p.exportable() { "" } else { "disabled" };
    let mut html = format!(
        "<section class=\"export-card\"><div class=\"eyebrow\">READY WHEN YOU ARE</div><h2>Take your plan to RuneLite.</h2><p>Import your equipment and packing list into Inventory Setups.</p><button data-action=\"copy\" class=\"button gold full\" {disabled}>Copy Inventory Setup <span>↗</span></button><button data-action=\"download-setup\" class=\"button ghost full\" {disabled}>Download setup JSON</button><small>{}</small><details><summary>How to import</summary><ol><li>Enable Inventory Setups and Bank Tags in RuneLite.</li><li>Choose the green import button → Import setup.</li><li>Paste the copied text and confirm.</li></ol><p>You withdraw and equip items yourself.</p></details>",
        if p.exportable() {
            "Check staff charges and teleport access before your trip."
        } else {
            "Resolve missing items or preparation blockers to export."
        }
    );
    if let Some(text) = &s.export_text {
        html.push_str(&format!("<label class=\"export-label\">Setup text<textarea readonly rows=\"5\">{}</textarea></label>",esc(text)));
    }
    html.push_str("</section>");
    html
}
fn next_goal(s: &State) -> String {
    let Some(m) = s.saved.snapshot.selected() else {
        return String::new();
    };
    let goals = planner::goals(m, s.saved.options.boss);
    let pins = s.pins();
    let goal = goals
        .iter()
        .find(|g| pins.iter().any(|p| p == g.id) && !g.complete)
        .or_else(|| goals.iter().find(|g| !g.complete));
    let Some(g) = goal else {
        return "<section class=\"panel goal-teaser\"><h3>Starter milestones reached</h3><p>Review your encounter supplies and unlocks.</p></section>".into();
    };
    format!(
        "<section class=\"panel goal-teaser\"><div class=\"eyebrow\">A LITTLE CLOSER</div><h3>{}</h3><p>{}</p><div class=\"progress-track\"><span style=\"width:{}%\"></span></div><small>{}</small>{}</section>",
        g.title,
        esc(g.reason),
        g.progress,
        esc(&g.next),
        button("tab:goals", "Explore progression goals →", "text-button")
    )
}
fn goals_page(s: &State) -> String {
    let mut html="<div class=\"page-heading\"><div><div class=\"eyebrow\">ONE STEP AT A TIME</div><h1>Give the grind a purpose.</h1><p>Small, useful milestones on the road to your next adventure.</p></div></div><div class=\"goals-intro panel\"><span>✦</span><p>Pin up to three goals. Your first pinned goal is your primary focus. Progress follows the selected account's skills and imported quests.</p></div><div class=\"goal-grid\">".to_string();
    if let Some(m) = s.saved.snapshot.selected() {
        for g in planner::goals(m, s.saved.options.boss) {
            let pinned = s.pins().iter().position(|p| p == g.id);
            html.push_str(&format!("<article class=\"panel goal-card\"><div class=\"goal-card-top\">{}<span class=\"goal-symbol\">{}</span></div><h2>{}</h2><p>{}</p><div class=\"progress-track\"><span style=\"width:{}%\"></span></div><div class=\"goal-next\"><small>NEXT STEP</small><strong>{}</strong></div><div class=\"goal-actions\">{}<a href=\"{}\" target=\"_blank\" rel=\"noreferrer\">Wiki guide ↗</a></div></article>",badge(if g.complete{"Milestone reached"}else if pinned==Some(0){"Primary goal"}else if pinned.is_some(){"Pinned goal"}else{"Suggested milestone"},if g.complete{"green"}else{"neutral"}),if g.complete{"✓"}else{"✦"},g.title,esc(g.reason),g.progress,esc(&g.next),button(&format!("pin:{}",g.id),if pinned.is_some(){"Unpin goal"}else{"Pin goal"},if pinned.is_some(){"button ghost"}else{"button gold"}),g.link));
        }
    }
    html.push_str("</div><p class=\"muted\">These are reviewed starter milestones, not a universal Ironman route. Item drops and unobserved unlocks require separate confirmation.</p>");
    html
}
fn access_select(id: &str, value: Option<bool>) -> String {
    format!(
        "<select id=\"{id}\"><option value=\"auto\" {}>Use imported state</option><option value=\"yes\" {}>Yes, confirmed</option><option value=\"no\" {}>Not yet</option></select>",
        if value.is_none() { "selected" } else { "" },
        if value == Some(true) { "selected" } else { "" },
        if value == Some(false) { "selected" } else { "" }
    )
}
fn account_page(s: &State) -> String {
    let mut html = format!(
        "<div class=\"page-heading\"><div><div class=\"eyebrow\">THE ADVENTURER BEHIND THE GEAR</div><h1>Your account, your browser.</h1><p>Connect GroupIron or import a snapshot. Your bank stays on this device.</p></div>{}</div><div class=\"account-grid\"><section class=\"panel account-form\"><div class=\"panel-heading\"><h2>Connect GroupIron</h2>{}</div><div class=\"connection-warning\"><strong>Direct browser sync is currently blocked.</strong><p>GroupIron is not allowing this site's browser origin. Use the Rust snapshot importer below, then choose the downloaded JSON file.</p></div><details class=\"import-helper\"><summary>Get a snapshot with the Rust importer</summary><p>From the Ironbound source directory, run:</p><code>cargo run --locked --bin site -- import-group 'Your Group'</code><p>Enter your token at the hidden prompt. Import <code>.local/groupiron.json</code> here. Your token is never saved.</p></details><p>Retry direct sync after GroupIron enables browser access:</p><label>Group name<input id=\"group-name\" autocomplete=\"off\" maxlength=\"80\" placeholder=\"Your group name\"></label><label>Group token<input id=\"group-token\" type=\"password\" autocomplete=\"off\" maxlength=\"300\" placeholder=\"Paste your group token\"></label><button class=\"button gold full\" data-action=\"connect\" {}>{}</button><small>The token is used for this request only. It is never saved in your account or backups. Reconnect to refresh.</small><a href=\"https://groupiron.men/\" target=\"_blank\" rel=\"noreferrer\">Open GroupIron ↗</a></section><section class=\"panel account-form\"><div class=\"panel-heading\"><h2>Import a snapshot</h2></div><p>Paste a raw GroupIron member-array response or restore an Ironbound backup.</p><label>GroupIron JSON or account backup<textarea id=\"import-data\" rows=\"6\" placeholder=\"Paste your GroupIron JSON here\"></textarea></label>{}<label class=\"file-label\">Or choose a JSON file<input id=\"import-file\" type=\"file\" accept=\"application/json,.json\"></label></section></div>",
        button("backup", "↓ Download account backup", "button ghost"),
        badge("READ ONLY", "green"),
        if s.busy { "disabled" } else { "" },
        if s.busy {
            "Connecting…"
        } else {
            "Read my GroupIron account →"
        },
        button("import", "Import account", "button ghost full")
    );
    html.push_str("<section class=\"panel account-summary\"><div class=\"panel-heading\"><h2>Account snapshot</h2>");
    html.push_str(&badge(&s.saved.snapshot.source, "neutral"));
    html.push_str(
        "</div><div class=\"snapshot-fields\"><label>Selected member<select id=\"member\">",
    );
    for m in &s.saved.snapshot.members {
        if m.name != "@SHARED" {
            html.push_str(&format!(
                "<option value=\"{}\" {}>{}</option>",
                esc(&m.name),
                if m.name == s.saved.snapshot.selected {
                    "selected"
                } else {
                    ""
                },
                esc(&m.name)
            ));
        }
    }
    html.push_str("</select></label>");
    if let Some(m) = s.saved.snapshot.selected() {
        html.push_str(&format!("<div><small>RETRIEVED</small><strong>{}</strong></div><div><small>GROUPIRON LAST UPDATE</small><strong>{}</strong></div><div><small>OBSERVED ITEM TYPES</small><strong>{}</strong></div></div><p class=\"small-note\">GroupIron's member timestamp is not proof that the bank was opened recently. Reopen your bank in RuneLite before reconnecting.</p>",esc(&s.saved.snapshot.fetched_at),esc(m.last_updated.as_deref().unwrap_or("Unknown")),planner::raw_item_count(m)));
        html.push_str(&format!(
            "<div class=\"access-grid\"><label>{} complete{}</label>",
            if s.saved.options.boss == Boss::Barrows {
                "Priest in Peril"
            } else {
                "Beneath Cursed Sands"
            },
            access_select("access", s.saved.options.access_override)
        ));
        if s.saved.options.boss == Boss::Barrows {
            html.push_str(&format!(
                "<label>His Faithful Servants started{}</label>",
                access_select("started", s.saved.options.started_barrows)
            ));
        }
        html.push_str("</div><details class=\"skill-corrections\"><summary>Skills & local corrections</summary><p>Imported values are XP, converted to levels. Editing a level saves a local correction; reconnecting replaces it.</p><div class=\"skills-grid\">");
        for (i, skill) in SKILLS.iter().enumerate() {
            html.push_str(&format!("<label>{skill}<input id=\"skill-{i}\" type=\"number\" min=\"1\" max=\"99\" value=\"{}\" placeholder=\"?\"></label>",m.level(skill).map(|v|v.to_string()).unwrap_or_default()));
        }
        html.push_str("</div></details>");
    }
    html.push_str(&format!("</section><div class=\"account-tools\">{}{}<span>Backups contain account data and goals, never your GroupIron token.</span></div>",button("demo","Explore sample account","text-button"),button("forget","Forget saved account","text-button danger")));
    html
}
