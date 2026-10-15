use family_core::{new_recovery_key, FamilyCore};
use gtk::{gio, glib, prelude::*};
use serde_json::{json, Value};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpokenCaptionCue {
    start_ms: u64,
    end_ms: u64,
    text: String,
}

impl SpokenCaptionCue {
    fn is_active_at(&self, milliseconds: u64) -> bool {
        self.start_ms <= milliseconds && milliseconds < self.end_ms
    }
}

fn spoken_caption_cues(response: &Value) -> Vec<SpokenCaptionCue> {
    let mut cues: Vec<_> = response["cues"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|cue| {
            let start_ms = cue["start_ms"].as_u64()?;
            let end_ms = cue["end_ms"].as_u64()?;
            let text = cue["text"].as_str()?.trim();
            (end_ms > start_ms
                && end_ms.saturating_sub(start_ms) <= 120_000
                && !text.is_empty()
                && text.chars().count() <= 1_024)
                .then(|| SpokenCaptionCue {
                    start_ms,
                    end_ms,
                    text: text.to_owned(),
                })
        })
        .take(15_000)
        .collect();
    cues.sort_by(|left, right| {
        left.start_ms
            .cmp(&right.start_ms)
            .then(left.end_ms.cmp(&right.end_ms))
            .then(left.text.cmp(&right.text))
    });
    cues.dedup();
    cues
}

struct Session {
    core: Arc<FamilyCore>,
    recovery_key: String,
    snapshot: Value,
    room_id: String,
    last_video_prompt: Option<(String, String)>,
    storage_feedback: Option<(String, String)>,
    section: String,
    library_search: String,
    library_year: String,
    library_moment: String,
    moment_river_year: String,
    library_people: HashSet<String>,
    library_match_all_people: bool,
    library_filters_dirty: bool,
    refreshing_library_filters: Rc<Cell<bool>>,
    year_filter: Option<glib::WeakRef<gtk::ComboBoxText>>,
    moment_filter: Option<glib::WeakRef<gtk::ComboBoxText>>,
    people_filter: Option<glib::WeakRef<gtk::MenuButton>>,
    people_filter_list: Option<glib::WeakRef<gtk::Box>>,
    people_match_all: Option<glib::WeakRef<gtk::CheckButton>>,
    library_list: Option<glib::WeakRef<gtk::ListBox>>,
    selecting_assets: bool,
    selected_assets: HashSet<String>,
    rooms_widget: Option<glib::WeakRef<gtk::DropDown>>,
    restore_backup_button: Option<glib::WeakRef<gtk::Button>>,
    restore_recovery_entry: Option<glib::WeakRef<gtk::PasswordEntry>>,
    storage_gateway_entry: Option<glib::WeakRef<gtk::Entry>>,
    storage_gateway_connect: Option<glib::WeakRef<gtk::Button>>,
    parent_window: Option<glib::WeakRef<gtk::Window>>,
    watch_scanning: Rc<Cell<bool>>,
}

struct Screening {
    core: Arc<FamilyCore>,
    films: Vec<Value>,
    title: String,
    index: usize,
    generation: u64,
    video: gtk::Video,
    caption_label: gtk::Label,
    caption_panel: gtk::Box,
    caption_toggle: gtk::ToggleButton,
    caption_cues: Vec<SpokenCaptionCue>,
    heading: gtk::Label,
    temporary_file: Option<PathBuf>,
    start_at_ms: u64,
}

fn preview_root() -> Result<PathBuf, String> {
    let root = glib::user_cache_dir().join("family-room-preview");
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    Ok(root)
}

fn playlist_films(room: &Value, item: &Value) -> Result<Vec<Value>, String> {
    let ids = item["story_ids"]
        .as_array()
        .ok_or("Playlist has no films")?;
    if ids.is_empty() {
        return Err("Add a film before playing this playlist".into());
    }
    ids.iter()
        .map(|id| {
            let story = room["stories"]
                .as_array()
                .and_then(|stories| {
                    stories
                        .iter()
                        .find(|story| story["id"] == *id && story["private_draft"] == false)
                })
                .ok_or_else(|| {
                    format!("Film {} is no longer published", id.as_str().unwrap_or("?"))
                })?;
            let asset_id = story["published_asset"]
                .as_str()
                .ok_or("Published film has no movie")?;
            room["assets"]
                .as_array()
                .and_then(|assets| {
                    assets
                        .iter()
                        .find(|asset| asset["id"] == asset_id && asset["deleted"] != true)
                })
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "Original for film {} is unavailable",
                        id.as_str().unwrap_or("?")
                    )
                })
        })
        .collect()
}

async fn materialize_movie(core: Arc<FamilyCore>, asset: &Value) -> Result<PathBuf, String> {
    let root = preview_root()?;
    let filename = asset["filename"].as_str().unwrap_or("movie.mp4");
    let safe = std::path::Path::new(filename)
        .file_name()
        .ok_or("Invalid filename")?;
    let path = root.join(format!(
        "{}-{}-{}",
        asset["id"].as_str().unwrap_or("movie"),
        uuid::Uuid::new_v4(),
        safe.to_string_lossy()
    ));
    if asset["local"] != true {
        command(
            core.clone(),
            json!({"action":"fetch","room_id":asset["room_id"],"asset_id":asset["id"]}),
        )
        .await?;
    }
    if let Err(error) = command(core, json!({"action":"materialize","room_id":asset["room_id"],"asset_id":asset["id"],"destination":path})).await {
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    Ok(path)
}

fn play_film(screening: Rc<RefCell<Screening>>, index: usize) {
    let (core, asset, generation, seek_at_ms) = {
        let mut state = screening.borrow_mut();
        if index >= state.films.len() {
            return;
        }
        state.generation += 1;
        if let Some(stream) = state.video.media_stream() {
            stream.pause();
        }
        state.index = index;
        let seek_at_ms = if index == 0 { state.start_at_ms } else { 0 };
        state.start_at_ms = 0;
        state.heading.set_label(&format!(
            "{} · {} of {} · Loading…",
            state.title,
            index + 1,
            state.films.len()
        ));
        (
            state.core.clone(),
            state.films[index].clone(),
            state.generation,
            seek_at_ms,
        )
    };
    glib::spawn_future_local(async move {
        let result = materialize_movie(core.clone(), &asset).await;
        let caption_cues = if result.is_ok() {
            command(
                core,
                json!({
                    "action":"export_video_transcript",
                    "room_id":asset["room_id"],
                    "asset_id":asset["id"]
                }),
            )
            .await
            .map(|response| spoken_caption_cues(&response))
            .unwrap_or_default()
        } else {
            Vec::new()
        };
        let mut state = screening.borrow_mut();
        if state.generation != generation {
            if let Ok(path) = result {
                let _ = std::fs::remove_file(path);
            }
            return;
        }
        state.caption_cues = caption_cues;
        state
            .caption_toggle
            .set_sensitive(!state.caption_cues.is_empty());
        state.caption_toggle.set_active(true);
        state.caption_panel.set_visible(false);
        match result {
            Ok(path) => {
                let stream = gtk::MediaFile::for_filename(&path);
                let weak = Rc::downgrade(&screening);
                stream.connect_ended_notify(move |stream| {
                    if !stream.is_ended() {
                        return;
                    }
                    if let Some(screening) = weak.upgrade() {
                        let next = {
                            let state = screening.borrow();
                            (state.generation == generation).then_some(state.index + 1)
                        };
                        if let Some(next) = next {
                            let count = screening.borrow().films.len();
                            if next < count {
                                play_film(screening, next);
                            } else {
                                screening.borrow().heading.set_label("Playlist finished");
                            }
                        }
                    }
                });
                if seek_at_ms > 0 {
                    let target = seek_at_ms.saturating_mul(1_000) as i64;
                    stream.connect_prepared_notify(move |stream| {
                        if stream.is_prepared() {
                            stream.seek(target);
                        }
                    });
                }
                let video = state.video.clone();
                let old = state.temporary_file.replace(path);
                state.heading.set_label(&format!(
                    "{} · {} of {}",
                    state.title,
                    index + 1,
                    state.films.len()
                ));
                drop(state);
                video.set_media_stream(Some(&stream));
                if let Some(old) = old {
                    let _ = std::fs::remove_file(old);
                }
                stream.play();
            }
            Err(error) => state
                .heading
                .set_label(&format!("Could not play film {}: {error}", index + 1)),
        }
    });
}

fn open_playlist(parent: &gtk::Window, core: Arc<FamilyCore>, title: &str, films: Vec<Value>) {
    open_screening(parent, core, title, films, 0);
}

fn open_screening(
    parent: &gtk::Window,
    core: Arc<FamilyCore>,
    title: &str,
    films: Vec<Value>,
    start_at_ms: u64,
) {
    let window = gtk::Window::builder()
        .title(title)
        .transient_for(parent)
        .default_width(860)
        .default_height(570)
        .build();
    let layout = gtk::Box::new(gtk::Orientation::Vertical, 12);
    layout.set_margin_top(16);
    layout.set_margin_bottom(16);
    layout.set_margin_start(16);
    layout.set_margin_end(16);
    let heading = gtk::Label::new(Some(title));
    heading.add_css_class("heading");
    let video = gtk::Video::for_media_stream(None::<&gtk::MediaStream>);
    video.set_hexpand(true);
    video.set_vexpand(true);
    let video_overlay = gtk::Overlay::new();
    video_overlay.set_child(Some(&video));
    let caption_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    caption_panel.add_css_class("card");
    caption_panel.set_halign(gtk::Align::Center);
    caption_panel.set_valign(gtk::Align::End);
    caption_panel.set_margin_start(20);
    caption_panel.set_margin_end(20);
    caption_panel.set_margin_bottom(18);
    let caption_label = gtk::Label::new(None);
    caption_label.set_wrap(true);
    caption_label.set_justify(gtk::Justification::Center);
    caption_label.set_max_width_chars(72);
    caption_label.add_css_class("title-4");
    caption_panel.append(&caption_label);
    caption_panel.set_visible(false);
    video_overlay.add_overlay(&caption_panel);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let previous = gtk::Button::with_label("Previous film");
    let next = gtk::Button::with_label("Next film");
    let caption_toggle = gtk::ToggleButton::with_label("Spoken captions");
    caption_toggle.set_active(true);
    caption_toggle.set_sensitive(false);
    let toggle_panel = caption_panel.clone();
    caption_toggle.connect_toggled(move |toggle| {
        if !toggle.is_active() {
            toggle_panel.set_visible(false);
        }
    });
    controls.append(&previous);
    controls.append(&next);
    controls.append(&caption_toggle);
    layout.append(&heading);
    layout.append(&video_overlay);
    layout.append(&controls);
    window.set_child(Some(&layout));
    let screening = Rc::new(RefCell::new(Screening {
        core,
        films,
        title: title.into(),
        index: 0,
        generation: 0,
        video,
        caption_label,
        caption_panel,
        caption_toggle,
        caption_cues: Vec::new(),
        heading,
        temporary_file: None,
        start_at_ms,
    }));
    let weak = Rc::downgrade(&screening);
    glib::timeout_add_local(Duration::from_millis(200), move || {
        let Some(screening) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let state = screening.borrow();
        let cue = if state.caption_toggle.is_active() {
            state.video.media_stream().and_then(|stream| {
                let milliseconds = u64::try_from(stream.timestamp().max(0) / 1_000_000).ok()?;
                state
                    .caption_cues
                    .iter()
                    .find(|cue| cue.is_active_at(milliseconds))
                    .cloned()
            })
        } else {
            None
        };
        if let Some(cue) = cue {
            state.caption_label.set_text(&cue.text);
            state.caption_panel.set_visible(true);
        } else {
            state.caption_panel.set_visible(false);
        }
        glib::ControlFlow::Continue
    });
    let weak = Rc::downgrade(&screening);
    previous.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            let index = state.borrow().index;
            if index > 0 {
                play_film(state, index - 1);
            }
        }
    });
    let weak = Rc::downgrade(&screening);
    next.connect_clicked(move |_| {
        if let Some(state) = weak.upgrade() {
            let (index, count) = {
                let current = state.borrow();
                (current.index, current.films.len())
            };
            if index + 1 < count {
                play_film(state, index + 1);
            }
        }
    });
    let cleanup = screening.clone();
    window.connect_close_request(move |_| {
        let mut state = cleanup.borrow_mut();
        state.generation += 1;
        let video = state.video.clone();
        let old = state.temporary_file.take();
        drop(state);
        video.set_media_stream(None::<&gtk::MediaStream>);
        if let Some(path) = old {
            let _ = std::fs::remove_file(path);
        }
        glib::Propagation::Proceed
    });
    window.present();
    play_film(screening, 0);
}
fn open() -> Result<(Arc<FamilyCore>, String), Box<dyn std::error::Error>> {
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(glib::user_data_dir()))
        .join("family-room");
    std::fs::create_dir_all(&root)?;
    let entry = keyring::Entry::new("com.familyroom.vault", "recovery")?;
    let key = match entry.get_password() {
        Ok(key) => key,
        Err(keyring::Error::NoEntry) => {
            if root.join("catalog.fr").exists() {
                return Err("Recover your vault key before opening the existing library".into());
            }
            let key = new_recovery_key();
            entry.set_password(&key)?;
            key
        }
        Err(error) => return Err(error.into()),
    };
    let core = FamilyCore::open(root.to_string_lossy().into(), key.clone())?;
    Ok((core, key))
}

fn can_restore_backup(snapshot: &Value) -> bool {
    snapshot["rooms"]
        .as_array()
        .is_some_and(|rooms| rooms.is_empty())
        && snapshot["pending_invitations"]
            .as_array()
            .is_some_and(|invitations| invitations.is_empty())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RoomSetupStatus {
    can_manage_storage: bool,
    has_preferred_storage: bool,
    recovery_key_available: bool,
    has_first_contribution: bool,
    has_active_folder_watch: bool,
}

fn room_setup_status(room: &Value, snapshot: &Value, recovery_key: &str) -> RoomSetupStatus {
    let room_id = room["id"].as_str();
    let can_manage_storage = matches!(room["role"].as_str(), Some("owner" | "admin"));
    let has_preferred_storage = snapshot["sources"].as_array().is_some_and(|sources| {
        sources.iter().any(|source| {
            source["room_id"].as_str() == room_id
                && source["preferred"].as_bool() == Some(true)
                && source["paused"].as_bool() != Some(true)
        })
    });
    let has_first_contribution = room["assets"].as_array().is_some_and(|assets| {
        assets.iter().any(|asset| {
            asset["deleted"].as_bool() != Some(true)
                && matches!(asset["kind"].as_str(), Some("photo" | "video"))
        })
    });
    let has_active_folder_watch = snapshot["watches"].as_array().is_some_and(|watches| {
        watches.iter().any(|watch| {
            watch["room_id"].as_str() == room_id
                && watch["paused"].as_bool() != Some(true)
                && watch["path"]
                    .as_str()
                    .is_some_and(|path| !path.starts_with("photos://"))
        })
    });

    RoomSetupStatus {
        can_manage_storage,
        has_preferred_storage,
        recovery_key_available: !recovery_key.trim().is_empty(),
        has_first_contribution,
        has_active_folder_watch,
    }
}

async fn command(core: Arc<FamilyCore>, value: Value) -> Result<Value, String> {
    let network_state = if matches!(
        value["action"].as_str(),
        Some("sync_all" | "fetch" | "replicate" | "replicate_step" | "retry_transfer")
    ) {
        let monitor = gio::NetworkMonitor::default();
        Some(if !monitor.is_network_available() {
            "offline"
        } else if monitor.is_network_metered() {
            "metered"
        } else {
            "unmetered"
        })
    } else {
        None
    };
    let (sender, receiver) = async_channel::bounded(1);
    std::thread::spawn(move || {
        if let Some(state) = network_state {
            if let Err(error) =
                core.command(json!({"action":"set_network_state","state":state}).to_string())
            {
                let _ = sender.send_blocking(Err(error.to_string()));
                return;
            }
        }
        let result = core
            .command(value.to_string())
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()));
        let _ = sender.send_blocking(result);
    });
    receiver.recv().await.map_err(|e| e.to_string())?
}

fn collection_payload(
    room_id: &str,
    kind: &str,
    item: &Value,
    title: &str,
    ids: &[String],
) -> Value {
    let playlist = kind == "playlist";
    let mut value = json!({
        "action": if playlist { "save_playlist" } else { "save_album" },
        "room_id": room_id,
        "title": title,
        "id": item["id"],
    });
    value[if playlist { "story_ids" } else { "asset_ids" }] = json!(ids);
    if !playlist {
        value["pinned"] = json!(item["pinned"] == true);
    }
    value
}

fn submit_collection(
    session: Rc<RefCell<Session>>,
    list: gtk::ListBox,
    button: gtk::Button,
    value: Value,
) {
    button.set_sensitive(false);
    glib::spawn_future_local(async move {
        let core = session.borrow().core.clone();
        let result = async {
            command(core.clone(), value).await?;
            let snapshot = command(core, json!({"action":"snapshot"})).await?;
            session.borrow_mut().snapshot = snapshot;
            render(&session, &list);
            Ok::<(), String>(())
        }
        .await;
        if let Err(error) = result {
            button.set_label(&error);
            button.set_sensitive(true);
        }
    });
}

fn submit_comment(
    session: Rc<RefCell<Session>>,
    list: gtk::ListBox,
    entry: gtk::Entry,
    button: gtk::Button,
    room_id: String,
    asset_id: String,
) {
    let body = entry.text().trim().to_owned();
    if body.is_empty() {
        return;
    }
    button.set_sensitive(false);
    glib::spawn_future_local(async move {
        let core = session.borrow().core.clone();
        let result = async {
            command(
                core.clone(),
                json!({"action":"comment","room_id":room_id,"asset_id":asset_id,"body":body}),
            )
            .await?;
            let snapshot = command(core, json!({"action":"snapshot"})).await?;
            session.borrow_mut().snapshot = snapshot;
            entry.set_text("");
            render(&session, &list);
            Ok::<(), String>(())
        }
        .await;
        if let Err(error) = result {
            button.set_label(&format!("Retry note: {error}"));
            button.set_sensitive(true);
        }
    });
}

fn append_offline_original_controls(
    session: &Rc<RefCell<Session>>,
    list: &gtk::ListBox,
    panel: &gtk::Box,
    room_id: &str,
    room: &Value,
    album_ids: &[String],
) {
    let assets = room["assets"].as_array().cloned().unwrap_or_default();
    let mut wanted: HashSet<String> = album_ids.iter().cloned().collect();
    for asset in &assets {
        if album_ids.iter().any(|id| asset["id"].as_str() == Some(id)) {
            if let Some(components) = asset["components"].as_array() {
                wanted.extend(
                    components
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned),
                );
            }
        }
    }
    let offline_assets: Vec<_> = assets
        .iter()
        .filter(|asset| {
            asset["deleted"] != true && asset["id"].as_str().is_some_and(|id| wanted.contains(id))
        })
        .collect();
    let pinned_count = offline_assets
        .iter()
        .filter(|asset| asset["pinned_offline"] == true)
        .count();
    let all_pinned = !offline_assets.is_empty() && pinned_count == offline_assets.len();
    let action = if all_pinned {
        "unpin_original"
    } else {
        "pin_original"
    };
    let label = if all_pinned {
        "Unpin originals".to_owned()
    } else {
        format!(
            "Save originals offline ({pinned_count}/{})",
            offline_assets.len()
        )
    };
    let button = gtk::Button::with_label(&label);
    button.set_sensitive(!offline_assets.is_empty());
    let asset_ids: Vec<String> = offline_assets
        .iter()
        .filter(|asset| (asset["pinned_offline"] == true) == all_pinned)
        .filter_map(|asset| asset["id"].as_str().map(str::to_owned))
        .collect();
    let pin_session = session.clone();
    let pin_list = list.clone();
    let pin_room_id = room_id.to_owned();
    button.connect_clicked(move |button| {
        button.set_sensitive(false);
        let failure_button = button.clone();
        let session = pin_session.clone();
        let list = pin_list.clone();
        let room_id = pin_room_id.clone();
        let asset_ids = asset_ids.clone();
        glib::spawn_future_local(async move {
            let core = session.borrow().core.clone();
            let result = async {
                for asset_id in asset_ids {
                    command(
                        core.clone(),
                        json!({"action":action,"room_id":room_id,"asset_id":asset_id}),
                    )
                    .await?;
                }
                let snapshot = command(core, json!({"action":"snapshot"})).await?;
                session.borrow_mut().snapshot = snapshot;
                render(&session, &list);
                Ok::<(), String>(())
            }
            .await;
            if let Err(error) = result {
                failure_button.set_label(&error);
                failure_button.set_sensitive(true);
                if let Ok(snapshot) =
                    command(session.borrow().core.clone(), json!({"action":"snapshot"})).await
                {
                    session.borrow_mut().snapshot = snapshot;
                    render(&session, &list);
                }
            }
        });
    });
    panel.append(&button);

    let evictable_ids: Vec<String> = offline_assets
        .iter()
        .filter(|asset| {
            asset["local"] == true
                && asset["pinned_offline"] != true
                && asset["verified_copies"].as_u64().unwrap_or(0) > 1
        })
        .filter_map(|asset| asset["id"].as_str().map(str::to_owned))
        .collect();
    if !evictable_ids.is_empty() {
        let evict =
            gtk::Button::with_label(&format!("Free device storage ({})", evictable_ids.len()));
        evict.set_tooltip_text(Some(
            "Only unpinned originals with another integrity-verified copy can be removed.",
        ));
        let session = session.clone();
        let list = list.clone();
        let room_id = room_id.to_owned();
        evict.connect_clicked(move |button| {
            button.set_sensitive(false);
            let failure_button = button.clone();
            let session = session.clone();
            let list = list.clone();
            let room_id = room_id.clone();
            let asset_ids = evictable_ids.clone();
            glib::spawn_future_local(async move {
                let core = session.borrow().core.clone();
                let result = async {
                    for asset_id in asset_ids {
                        command(
                            core.clone(),
                            json!({"action":"evict_original","room_id":room_id,"asset_id":asset_id}),
                        )
                        .await?;
                    }
                    let snapshot = command(core, json!({"action":"snapshot"})).await?;
                    session.borrow_mut().snapshot = snapshot;
                    render(&session, &list);
                    Ok::<(), String>(())
                }
                .await;
                if let Err(error) = result {
                failure_button.set_label(&error);
                failure_button.set_sensitive(true);
                if let Ok(snapshot) = command(
                    session.borrow().core.clone(),
                    json!({"action":"snapshot"}),
                )
                .await
                {
                    session.borrow_mut().snapshot = snapshot;
                    render(&session, &list);
                }
            }
            });
        });
        panel.append(&evict);
    }
}

fn append_watch_button(row: &gtk::Box, session: &Rc<RefCell<Session>>, asset: Value) {
    let button = gtk::Button::with_label("Watch");
    let state = session.clone();
    button.connect_clicked(move |button| {
        let core = state.borrow().core.clone();
        let asset = asset.clone();
        let button = button.clone();
        button.set_sensitive(false);
        glib::spawn_future_local(async move {
            let result = async {
                let root = preview_root()?;
                let filename = asset["filename"].as_str().unwrap_or("movie.mp4");
                let safe = std::path::Path::new(filename).file_name().ok_or("Invalid filename")?;
                let path = root.join(format!("{}-{}", asset["id"].as_str().unwrap_or(""), safe.to_string_lossy()));
                if asset["local"] != true {
                    command(core.clone(), json!({"action":"fetch","room_id":asset["room_id"],"asset_id":asset["id"]})).await?;
                }
                command(core, json!({"action":"materialize","room_id":asset["room_id"],"asset_id":asset["id"],"destination":path})).await?;
                gio::AppInfo::launch_default_for_uri(&gio::File::for_path(path).uri(), None::<&gio::AppLaunchContext>)
                    .map_err(|error| error.to_string())?;
                Ok::<(), String>(())
            }.await;
            if let Err(error) = result { button.set_label(&error); }
            button.set_sensitive(true);
        });
    });
    row.append(&button);
}

fn render_collections(
    session: &Rc<RefCell<Session>>,
    list: &gtk::ListBox,
    room: &Value,
    kind: &str,
) {
    let room_id = room["id"].as_str().unwrap_or("");
    let field = if kind == "playlist" {
        "story_ids"
    } else {
        "asset_ids"
    };
    let entries = if kind == "playlist" {
        "playlists"
    } else {
        "albums"
    };
    let can_edit = matches!(
        room["role"].as_str(),
        Some("owner" | "admin" | "contributor")
    );
    if can_edit {
        let create = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        create.set_margin_top(12);
        create.set_margin_bottom(12);
        create.set_margin_start(12);
        create.set_margin_end(12);
        let name = gtk::Entry::builder()
            .placeholder_text(format!("New {kind} name"))
            .hexpand(true)
            .build();
        let button = gtk::Button::with_label(&format!("Create {kind}"));
        let s = session.clone();
        let l = list.clone();
        let room_id = room_id.to_owned();
        let collection_kind = kind.to_owned();
        let name_input = name.clone();
        button.connect_clicked(move |button| {
            let title = name_input.text().trim().to_owned();
            if title.is_empty() {
                return;
            }
            let payload = collection_payload(&room_id, &collection_kind, &Value::Null, &title, &[]);
            submit_collection(s.clone(), l.clone(), button.clone(), payload);
        });
        create.append(&name);
        create.append(&button);
        list.append(&create);
    }
    let candidates: Vec<Value> = if kind == "playlist" {
        room["stories"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|story| story["private_draft"] == false && story["published_asset"].is_string())
            .cloned()
            .collect()
    } else {
        room["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|asset| asset["deleted"] != true)
            .cloned()
            .collect()
    };
    for item in room[entries].as_array().into_iter().flatten() {
        let title = item["title"].as_str().unwrap_or("Untitled");
        let ids: Vec<String> = item[field]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|id| id.as_str().map(str::to_owned))
            .collect();
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
        panel.set_margin_top(12);
        panel.set_margin_bottom(12);
        panel.set_margin_start(12);
        panel.set_margin_end(12);
        let heading = gtk::Label::new(Some(&format!(
            "{title} · {} {}",
            ids.len(),
            match (kind, ids.len()) {
                ("playlist", 1) => "film",
                ("playlist", _) => "films",
                (_, 1) => "memory",
                _ => "memories",
            }
        )));
        heading.set_xalign(0.0);
        heading.add_css_class("heading");
        panel.append(&heading);
        if kind == "album" {
            let cover_id = item["cover_asset_id"]
                .as_str()
                .or_else(|| ids.first().map(String::as_str));
            if let Some(cover) = candidates
                .iter()
                .find(|asset| asset["id"].as_str() == cover_id)
            {
                let cover_name = cover["caption"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .or_else(|| cover["filename"].as_str())
                    .unwrap_or("Memory");
                let cover_label = gtk::Label::new(Some(&format!("Cover · {cover_name}")));
                cover_label.set_xalign(0.0);
                cover_label.add_css_class("dim-label");
                panel.append(&cover_label);
            }
            if let Some(caption) = item["caption"].as_str().filter(|value| !value.is_empty()) {
                let note = gtk::Label::new(Some(caption));
                note.set_xalign(0.0);
                note.set_wrap(true);
                panel.append(&note);
            }
            if can_edit {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let note = gtk::Entry::builder()
                    .text(item["caption"].as_str().unwrap_or(""))
                    .placeholder_text("A note about this album")
                    .max_length(2000)
                    .hexpand(true)
                    .build();
                let save = gtk::Button::with_label("Save note");
                let s = session.clone();
                let l = list.clone();
                let r = room_id.to_owned();
                let item_copy = item.clone();
                let title = title.to_owned();
                let order = ids.clone();
                let note_input = note.clone();
                save.connect_clicked(move |button| {
                    let mut payload = collection_payload(&r, "album", &item_copy, &title, &order);
                    payload["caption"] = json!(note_input.text().as_str());
                    submit_collection(s.clone(), l.clone(), button.clone(), payload);
                });
                row.append(&note);
                row.append(&save);
                panel.append(&row);
            }
        }
        if kind == "album" && can_publish_album(room) {
            append_offline_original_controls(session, list, &panel, room_id, room, &ids);
            append_album_sharing_controls(session, &panel, room, item, title);
        }
        if kind == "playlist" {
            let play = gtk::Button::with_label("Play all");
            let room = room.clone();
            let item = item.clone();
            let core = session.borrow().core.clone();
            let title = title.to_owned();
            play.connect_clicked(move |button| match playlist_films(&room, &item) {
                Ok(films) => {
                    if let Some(parent) = button
                        .root()
                        .and_then(|root| root.downcast::<gtk::Window>().ok())
                    {
                        open_playlist(&parent, core.clone(), &title, films);
                    }
                }
                Err(error) => button.set_label(&error),
            });
            panel.append(&play);
        }
        if can_edit && kind == "playlist" {
            let delete = gtk::Button::with_label("Delete playlist");
            let s = session.clone();
            let l = list.clone();
            let room_id = room_id.to_owned();
            let playlist_id = item["id"].as_str().unwrap_or("").to_owned();
            delete.connect_clicked(move |button| {
                let Some(parent) = l
                    .root()
                    .and_then(|root| root.downcast::<gtk::Window>().ok())
                else {
                    return;
                };
                let dialog = gtk::MessageDialog::builder()
                    .transient_for(&parent)
                    .modal(true)
                    .text("Delete this playlist?")
                    .secondary_text("The films and their originals stay in the Room.")
                    .build();
                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Delete playlist", gtk::ResponseType::Accept);
                let s = s.clone();
                let l = l.clone();
                let button = button.clone();
                let room_id = room_id.clone();
                let playlist_id = playlist_id.clone();
                dialog.connect_response(move |dialog, response| {
                    if response == gtk::ResponseType::Accept {
                        submit_collection(
                            s.clone(),
                            l.clone(),
                            button.clone(),
                            json!({"action":"delete_playlist","room_id":room_id,"id":playlist_id}),
                        );
                    }
                    dialog.close();
                });
                dialog.present();
            });
            panel.append(&delete);
        }
        if can_edit {
            let rename = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let name = gtk::Entry::builder().text(title).hexpand(true).build();
            let save = gtk::Button::with_label("Save name");
            let s = session.clone();
            let l = list.clone();
            let r = room_id.to_owned();
            let k = kind.to_owned();
            let item_copy = item.clone();
            let order = ids.clone();
            let name_input = name.clone();
            save.connect_clicked(move |button| {
                let title = name_input.text().trim().to_owned();
                if title.is_empty() {
                    return;
                }
                submit_collection(
                    s.clone(),
                    l.clone(),
                    button.clone(),
                    collection_payload(&r, &k, &item_copy, &title, &order),
                );
            });
            rename.append(&name);
            rename.append(&save);
            panel.append(&rename);
            let available: Vec<Value> = candidates
                .iter()
                .filter(|candidate| {
                    candidate["id"]
                        .as_str()
                        .is_some_and(|id| !ids.iter().any(|existing| existing == id))
                })
                .cloned()
                .collect();
            if !available.is_empty() {
                let add_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let labels: Vec<String> = available
                    .iter()
                    .map(|candidate| {
                        candidate["title"]
                            .as_str()
                            .or(candidate["caption"].as_str())
                            .filter(|name| !name.is_empty())
                            .or(candidate["filename"].as_str())
                            .unwrap_or("Memory")
                            .to_owned()
                    })
                    .collect();
                let choice = gtk::DropDown::from_strings(
                    &labels.iter().map(String::as_str).collect::<Vec<_>>(),
                );
                choice.set_hexpand(true);
                let add = gtk::Button::with_label(if kind == "playlist" {
                    "Add film"
                } else {
                    "Add memory"
                });
                let s = session.clone();
                let l = list.clone();
                let r = room_id.to_owned();
                let k = kind.to_owned();
                let item_copy = item.clone();
                let order = ids.clone();
                let title = title.to_owned();
                let selected = choice.clone();
                add.connect_clicked(move |button| {
                    if let Some(candidate) = available.get(selected.selected() as usize) {
                        if let Some(id) = candidate["id"].as_str() {
                            let mut next = order.clone();
                            next.push(id.to_owned());
                            submit_collection(
                                s.clone(),
                                l.clone(),
                                button.clone(),
                                collection_payload(&r, &k, &item_copy, &title, &next),
                            );
                        }
                    }
                });
                add_row.append(&choice);
                add_row.append(&add);
                panel.append(&add_row);
            }
        }
        for (index, id) in ids.iter().enumerate() {
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let name = candidates
                .iter()
                .find(|candidate| candidate["id"] == id.as_str())
                .and_then(|candidate| {
                    candidate["title"]
                        .as_str()
                        .filter(|name| !name.is_empty())
                        .or(candidate["caption"].as_str())
                        .filter(|name| !name.is_empty())
                        .or(candidate["filename"].as_str())
                })
                .unwrap_or("Unavailable item");
            let label = gtk::Label::new(Some(&format!("{}. {name}", index + 1)));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            line.append(&label);
            if kind == "playlist" {
                if let Some(movie_id) = candidates
                    .iter()
                    .find(|story| story["id"] == id.as_str())
                    .and_then(|story| story["published_asset"].as_str())
                {
                    if let Some(movie) = room["assets"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|asset| asset["id"] == movie_id && asset["deleted"] != true)
                    {
                        append_watch_button(&line, session, movie.clone());
                    }
                }
            }
            if can_edit {
                if kind == "album" {
                    let cover = gtk::Button::with_label("Make cover");
                    let s = session.clone();
                    let l = list.clone();
                    let r = room_id.to_owned();
                    let item_copy = item.clone();
                    let title = title.to_owned();
                    let order = ids.clone();
                    let cover_id = id.clone();
                    cover.connect_clicked(move |button| {
                        let mut payload =
                            collection_payload(&r, "album", &item_copy, &title, &order);
                        payload["cover_asset_id"] = json!(cover_id);
                        submit_collection(s.clone(), l.clone(), button.clone(), payload);
                    });
                    line.append(&cover);
                }
                for (label, target) in [
                    ("Move earlier", index.checked_sub(1)),
                    ("Move later", (index + 1 < ids.len()).then_some(index + 1)),
                    ("Remove", None),
                ] {
                    let button = gtk::Button::with_label(label);
                    button.set_sensitive(label == "Remove" || target.is_some());
                    let s = session.clone();
                    let l = list.clone();
                    let r = room_id.to_owned();
                    let k = kind.to_owned();
                    let item_copy = item.clone();
                    let order = ids.clone();
                    let title = title.to_owned();
                    button.connect_clicked(move |button| {
                        let mut next = order.clone();
                        if let Some(target) = target {
                            next.swap(index, target);
                        } else {
                            next.remove(index);
                        }
                        submit_collection(
                            s.clone(),
                            l.clone(),
                            button.clone(),
                            collection_payload(&r, &k, &item_copy, &title, &next),
                        );
                    });
                    line.append(&button);
                }
            }
            panel.append(&line);
        }
        list.append(&panel);
    }
}
fn can_publish_album(room: &Value) -> bool {
    room["album_only"] != true && matches!(room["role"].as_str(), Some("owner" | "admin"))
}

fn album_publication_asset_ids(room: &Value, album: &Value) -> Vec<String> {
    let Some(assets) = room["assets"].as_array() else {
        return Vec::new();
    };
    let Some(album_ids) = album["asset_ids"].as_array() else {
        return Vec::new();
    };
    let mut selected = Vec::new();
    for id in album_ids.iter().filter_map(Value::as_str) {
        let Some(asset) = assets
            .iter()
            .find(|asset| asset["id"] == id && asset["deleted"] != true)
        else {
            continue;
        };
        if !selected.iter().any(|existing| existing == id) {
            selected.push(id.to_owned());
        }
        for component in asset["components"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let available = assets
                .iter()
                .any(|candidate| candidate["id"] == component && candidate["deleted"] != true);
            if available && !selected.iter().any(|existing| existing == component) {
                selected.push(component.to_owned());
            }
        }
    }
    selected
}

fn append_album_sharing_controls(
    session: &Rc<RefCell<Session>>,
    parent: &gtk::Box,
    room: &Value,
    album: &Value,
    title: &str,
) {
    let toggle = gtk::Button::with_label("Share privately");
    let controls = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let explanation = gtk::Label::new(Some(&format!(
        "Share only “{title}” as a separately encrypted album library. Named People do not receive access; downloaded copies cannot be recalled.",
    )));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.add_css_class("dim-label");
    controls.append(&explanation);
    let device = gtk::Entry::builder()
        .placeholder_text("Viewer’s device code")
        .hexpand(true)
        .build();
    controls.append(&device);
    let invitation = gtk::Entry::builder().editable(false).hexpand(true).build();
    invitation.set_visible(false);
    controls.append(&invitation);
    let status = gtk::Label::new(None);
    status.set_xalign(0.0);
    status.set_wrap(true);
    controls.append(&status);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let publish = gtk::Button::with_label("Publish/update current viewers");
    let invite = gtk::Button::with_label("Create album-only invitation");
    let asset_ids = album_publication_asset_ids(room, album);
    publish.set_sensitive(!asset_ids.is_empty());
    invite.set_sensitive(!asset_ids.is_empty());
    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
    let album_id = album["id"].as_str().unwrap_or_default().to_owned();
    let room_copy = room.clone();
    let ids = asset_ids.clone();
    let s = session.clone();
    let output = invitation.clone();
    let message = status.clone();
    publish.connect_clicked(move |button| {
        run_album_share_command(
            s.clone(),
            button.clone(),
            room_copy.clone(),
            ids.clone(),
            json!({"action":"publish_album","room_id":room_id,"album_id":album_id}),
            false,
            output.clone(),
            message.clone(),
        );
    });
    actions.append(&publish);

    let room_copy = room.clone();
    let ids = asset_ids;
    let s = session.clone();
    let output = invitation.clone();
    let message = status.clone();
    let device_input = device.clone();
    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
    let album_id = album["id"].as_str().unwrap_or_default().to_owned();
    invite.connect_clicked(move |button| {
        let device_id = device_input.text().trim().to_owned();
        if device_id.is_empty() {
            message.set_text("Enter the intended viewer’s device code first.");
            return;
        }
        run_album_share_command(
            s.clone(),
            button.clone(),
            room_copy.clone(),
            ids.clone(),
            json!({"action":"invite_album","room_id":room_id,
                "album_id":album_id,"device_id":device_id}),
            true,
            output.clone(),
            message.clone(),
        );
    });
    actions.append(&invite);
    controls.append(&actions);
    let hide = gtk::Button::with_label("Hide sharing options");
    let revealer = gtk::Revealer::new();
    let reveal_for_hide = revealer.clone();
    hide.connect_clicked(move |_| reveal_for_hide.set_reveal_child(false));
    controls.append(&hide);
    revealer.set_child(Some(&controls));
    revealer.set_reveal_child(false);
    let reveal_for_toggle = revealer.clone();
    toggle.connect_clicked(move |_| reveal_for_toggle.set_reveal_child(true));
    parent.append(&toggle);
    parent.append(&revealer);
    let _ = title;
}

fn run_album_share_command(
    session: Rc<RefCell<Session>>,
    button: gtk::Button,
    room: Value,
    asset_ids: Vec<String>,
    request: Value,
    creates_invitation: bool,
    invitation: gtk::Entry,
    status: gtk::Label,
) {
    button.set_sensitive(false);
    status.set_text("Checking album originals stored on this device…");
    glib::spawn_future_local(async move {
        let core = session.borrow().core.clone();
        let room_id = room["id"].as_str().unwrap_or_default().to_owned();
        let result = async {
            for asset_id in &asset_ids {
                let Some(asset) = room["assets"].as_array().and_then(|assets| {
                    assets
                        .iter()
                        .find(|asset| asset["id"].as_str() == Some(asset_id.as_str()))
                }) else {
                    continue;
                };
                if asset["local"] != true {
                    command(
                        core.clone(),
                        json!({"action":"fetch","room_id":room_id,"asset_id":asset_id}),
                    )
                    .await?;
                }
            }
            let response = command(core.clone(), request).await?;
            command(core.clone(), json!({"action":"sync_all","room_id":room_id})).await?;
            let snapshot = command(core, json!({"action":"snapshot"})).await?;
            session.borrow_mut().snapshot = snapshot;
            Ok::<Value, String>(response)
        }
        .await;
        match result {
            Ok(response) => {
                if creates_invitation {
                    invitation.set_text(response["invitation"].as_str().unwrap_or_default());
                    invitation.set_visible(true);
                    status.set_text("Album-only invitation created. Copy the selected token to the intended viewer.");
                } else {
                    status.set_text(&format!(
                        "Published {} album memories for current viewers.",
                        response["count"].as_u64().unwrap_or_default()
                    ));
                }
            }
            Err(error) => status.set_text(&error),
        }
        button.set_sensitive(true);
    });
}

fn render_storage_sources(
    session: &Rc<RefCell<Session>>,
    list: &gtk::ListBox,
    snapshot: &Value,
    room_id: &str,
) {
    let heading = gtk::Label::new(Some("Upload destinations"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-2");
    list.append(&heading);
    if let Some((_, message)) = session
        .borrow()
        .storage_feedback
        .clone()
        .filter(|(feedback_room, _)| feedback_room == room_id)
    {
        let feedback = gtk::Label::new(Some(&message));
        feedback.set_xalign(0.0);
        feedback.set_wrap(true);
        list.append(&feedback);
    }
    let cleanup = gtk::Box::new(gtk::Orientation::Vertical, 6);
    cleanup.set_margin_top(8);
    cleanup.set_margin_bottom(8);
    let cleanup_title = gtk::Label::new(Some("Free device space"));
    cleanup_title.set_xalign(0.0);
    cleanup_title.add_css_class("heading");
    cleanup.append(&cleanup_title);
    let cleanup_note = gtk::Label::new(Some("Only originals with another verified copy are eligible. Offline pins stay here; older unpinned memories are considered first."));
    cleanup_note.set_xalign(0.0);
    cleanup_note.set_wrap(true);
    cleanup_note.add_css_class("dim-label");
    cleanup.append(&cleanup_note);
    let targets = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (label, bytes) in [
        ("500 MB", 500_u64 * 1_024 * 1_024),
        ("1 GB", 1_024_u64 * 1_024 * 1_024),
        ("5 GB", 5_u64 * 1_024 * 1_024 * 1_024),
    ] {
        let button = gtk::Button::with_label(label);
        let s = session.clone();
        let l = list.clone();
        let target_room = room_id.to_owned();
        button.connect_clicked(move |_| {
            let s = s.clone();
            let l = l.clone();
            let target_room = target_room.clone();
            glib::spawn_future_local(async move {
                let core = s.borrow().core.clone();
                let result = command(
                    core.clone(),
                    json!({"action":"free_device_storage","room_id":target_room.clone(),"bytes_to_free":bytes}),
                )
                .await;
                let message = match result {
                    Ok(result) => {
                        let freed = result["bytes_freed"].as_u64().unwrap_or_default();
                        let complete = result["complete"] == true;
                        let pinned = result["pinned_originals_preserved"]
                            .as_u64()
                            .unwrap_or_default();
                        match command(core, json!({"action":"snapshot"})).await {
                            Ok(snapshot) => {
                                s.borrow_mut().snapshot = snapshot;
                                format!(
                                    "Freed {}. {} {pinned} pinned originals stayed here.",
                                    format_storage_bytes(freed),
                                    if complete {
                                        "Request met."
                                    } else {
                                        "Not enough verified copies to meet the full request."
                                    }
                                )
                            }
                            Err(error) => error,
                        }
                    }
                    Err(error) => error,
                };
                s.borrow_mut().storage_feedback = Some((target_room, message));
                render(&s, &l);
            });
        });
        targets.append(&button);
    }
    cleanup.append(&targets);
    list.append(&cleanup);
    let current_limit = snapshot["rooms"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|room| room["id"] == room_id)
        .and_then(|room| room["device_cache_limit_bytes"].as_u64());
    let policy = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let policy_title = gtk::Label::new(Some(&format!(
        "Automatic device cache · {}",
        current_limit.map_or_else(|| "No limit".to_owned(), format_storage_bytes)
    )));
    policy_title.set_xalign(0.0);
    policy_title.add_css_class("heading");
    policy.append(&policy_title);
    let policy_note = gtk::Label::new(Some("After Room sync, Family Room removes only originals with another verified copy. Offline pins and sole copies stay here."));
    policy_note.set_xalign(0.0);
    policy_note.set_wrap(true);
    policy_note.add_css_class("dim-label");
    policy.append(&policy_note);
    let limits = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (label, max_bytes) in [
        ("No limit", None),
        ("2 GB", Some(2_u64 * 1_024 * 1_024 * 1_024)),
        ("5 GB", Some(5_u64 * 1_024 * 1_024 * 1_024)),
        ("10 GB", Some(10_u64 * 1_024 * 1_024 * 1_024)),
    ] {
        let button_label = if current_limit == max_bytes {
            format!("✓ {label}")
        } else {
            label.to_owned()
        };
        let button = gtk::Button::with_label(&button_label);
        let s = session.clone();
        let l = list.clone();
        let target_room = room_id.to_owned();
        button.connect_clicked(move |_| {
            let s = s.clone();
            let l = l.clone();
            let target_room = target_room.clone();
            glib::spawn_future_local(async move {
                let core = s.borrow().core.clone();
                let message = match command(
                    core.clone(),
                    json!({"action":"set_device_cache_limit","room_id":target_room,"max_bytes":max_bytes}),
                )
                .await
                {
                    Ok(response) => {
                        let cleanup = &response["cleanup"];
                        let freed = cleanup["bytes_freed"].as_u64().unwrap_or_default();
                        let complete = cleanup["complete"] == true;
                        let new_message = max_bytes.map_or_else(
                            || "Automatic cache cleanup is off.".to_owned(),
                            |limit| format!(
                                "Cache limit set to {}. Freed {} now. {}",
                                format_storage_bytes(limit),
                                format_storage_bytes(freed),
                                if complete { "Limit met." } else { "Cleanup will retry after another verified copy is available." }
                            ),
                        );
                        match command(core, json!({"action":"snapshot"})).await {
                            Ok(snapshot) => {
                                s.borrow_mut().snapshot = snapshot;
                                new_message
                            }
                            Err(error) => error,
                        }
                    }
                    Err(error) => error,
                };
                s.borrow_mut().storage_feedback = Some((target_room, message));
                render(&s, &l);
            });
        });
        limits.append(&button);
    }
    policy.append(&limits);
    list.append(&policy);
    for source in snapshot["sources"].as_array().into_iter().flatten() {
        if source["room_id"] != room_id {
            continue;
        }
        let Some(id) = source["id"].as_str() else {
            continue;
        };
        let preferred = source["preferred"] == true;
        let overflow = source["overflow"] == true;
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
        panel.set_margin_top(12);
        panel.set_margin_bottom(12);
        panel.set_margin_start(12);
        panel.set_margin_end(12);
        let name = gtk::Label::new(source["name"].as_str());
        name.set_xalign(0.0);
        name.add_css_class("heading");
        panel.append(&name);
        let description = if preferred {
            "Preferred for new uploads"
        } else if overflow {
            "Used when preferred storage is full"
        } else {
            "Available storage source"
        };
        let detail = gtk::Label::new(Some(description));
        detail.set_xalign(0.0);
        detail.add_css_class("dim-label");
        panel.append(&detail);
        if source["kind"] != "local" {
            let unmetered = source["network_policy"] == "unmetered_only";
            let policy = gtk::Button::with_label(if unmetered {
                "Media: unmetered only"
            } else {
                "Media: any network"
            });
            let s = session.clone();
            let l = list.clone();
            let source_id = id.to_owned();
            policy.connect_clicked(move |button| {
                submit_collection(
                    s.clone(),
                    l.clone(),
                    button.clone(),
                    json!({"action":"set_source_network_policy","id":source_id,
                        "policy":if unmetered { "any" } else { "unmetered_only" }}),
                );
            });
            panel.append(&policy);
        }
        if !preferred {
            let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let make_preferred = gtk::Button::with_label("Make preferred");
            let s = session.clone();
            let l = list.clone();
            let source_id = id.to_owned();
            make_preferred.connect_clicked(move |button| {
                submit_collection(
                    s.clone(),
                    l.clone(),
                    button.clone(),
                    json!({"action":"prefer_source","id":source_id}),
                );
            });
            actions.append(&make_preferred);
            let choose_overflow = gtk::Button::with_label(if overflow {
                "Clear overflow"
            } else {
                "Use for overflow"
            });
            let s = session.clone();
            let l = list.clone();
            let r = room_id.to_owned();
            let source_id = id.to_owned();
            choose_overflow.connect_clicked(move |button| {
                submit_collection(
                    s.clone(),
                    l.clone(),
                    button.clone(),
                    json!({"action":"set_overflow","room_id":r,
                        "id":if overflow { Value::Null } else { json!(source_id) }}),
                );
            });
            actions.append(&choose_overflow);
            panel.append(&actions);
        }
        list.append(&panel);
    }
}

fn format_storage_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.1} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.0} MB", bytes as f64 / 1_000_000.0)
    } else {
        format!("{bytes} bytes")
    }
}

fn moment_local_year(timestamp: i64) -> Option<String> {
    chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
        .map(|date| date.with_timezone(&chrono::Local).format("%Y").to_string())
}

fn render_moment_river(
    session: &Rc<RefCell<Session>>,
    list: &gtk::ListBox,
    room: &Value,
    selected_year: &str,
) {
    let Some(moments) = room["moments"].as_array() else {
        return;
    };
    if moments.is_empty() {
        return;
    }
    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
    let can_edit = matches!(
        room["role"].as_str(),
        Some("owner" | "admin" | "contributor")
    );
    let mut ordered = moments.clone();
    ordered.sort_by(|left, right| {
        left["start"]
            .as_i64()
            .unwrap_or_default()
            .cmp(&right["start"].as_i64().unwrap_or_default())
            .then_with(|| left["id"].as_str().cmp(&right["id"].as_str()))
    });
    let years = ordered
        .iter()
        .filter_map(|moment| moment_local_year(moment["start"].as_i64()?))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let active_year = if years.iter().any(|year| year == selected_year) {
        selected_year
    } else {
        ""
    };
    let visible = ordered
        .iter()
        .filter(|moment| {
            active_year.is_empty()
                || moment_local_year(moment["start"].as_i64().unwrap_or_default()).as_deref()
                    == Some(active_year)
        })
        .cloned()
        .collect::<Vec<_>>();

    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.set_margin_top(10);
    panel.set_margin_bottom(14);
    panel.set_margin_start(12);
    panel.set_margin_end(12);
    let heading = gtk::Label::new(Some("Memory river"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-2");
    panel.append(&heading);
    let subtitle = gtk::Label::new(Some(
        "Move through your Room’s Moments in date order. Larger cards hold more memories.",
    ));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("dim-label");
    panel.append(&subtitle);

    let year_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let mut year_chips = vec![("All years".to_owned(), String::new())];
    year_chips.extend(years.iter().map(|year| (year.clone(), year.clone())));
    for (label, year) in year_chips {
        let chip = gtk::Button::with_label(&label);
        if year == active_year {
            chip.add_css_class("suggested-action");
        }
        let state = session.clone();
        let rows = list.clone();
        chip.connect_clicked(move |_| {
            state.borrow_mut().moment_river_year = year.clone();
            render(&state, &rows);
        });
        year_row.append(&chip);
    }
    let years_scroll = gtk::ScrolledWindow::builder()
        .child(&year_row)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(42)
        .build();
    panel.append(&years_scroll);

    if visible.is_empty() {
        let empty = gtk::Label::new(Some("No Moments in this year."));
        empty.set_xalign(0.0);
        empty.add_css_class("dim-label");
        panel.append(&empty);
    }
    let river = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    for moment in visible {
        let id = moment["id"].as_str().unwrap_or_default().to_owned();
        let title = moment["title"].as_str().unwrap_or("Moment").to_owned();
        let start = moment["start"].as_i64().unwrap_or_default();
        let date = chrono::DateTime::<chrono::Utc>::from_timestamp(start, 0)
            .map(|value| {
                value
                    .with_timezone(&chrono::Local)
                    .format("%b %-d, %Y")
                    .to_string()
            })
            .unwrap_or_else(|| "Moment".into());
        let ids = moment["asset_ids"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect::<Vec<_>>();
        let count = ids.len();
        let featured = moment["featured"].as_bool().unwrap_or(false);
        let width = (244 + count.min(16) as i32 * 7).clamp(252, 360);
        let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        card.add_css_class("card");
        card.set_size_request(width, 176);
        card.set_margin_top(4);
        card.set_margin_bottom(4);
        card.set_margin_start(2);
        card.set_margin_end(2);

        let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let date_label = gtk::Label::new(Some(&date));
        date_label.set_xalign(0.0);
        date_label.set_hexpand(true);
        date_label.add_css_class("dim-label");
        top.append(&date_label);
        if can_edit {
            let feature = gtk::Button::with_label(if featured { "★" } else { "☆" });
            feature.set_tooltip_text(Some(if featured {
                "Remove this Moment from Home spotlight"
            } else {
                "Feature this Moment on Home"
            }));
            let target_id = id.clone();
            let target_title = title.clone();
            let target_ids = ids.clone();
            let target_room = room_id.clone();
            let state = session.clone();
            let rows = list.clone();
            feature.connect_clicked(move |button| {
                let request = json!({
                    "action":"save_moment", "room_id":target_room, "id":target_id,
                    "title":target_title, "asset_ids":target_ids, "featured":!featured
                });
                submit_collection(state.clone(), rows.clone(), button.clone(), request);
            });
            top.append(&feature);
        } else if featured {
            let star = gtk::Label::new(Some("★"));
            star.set_tooltip_text(Some("Featured on Home"));
            top.append(&star);
        }
        card.append(&top);

        let visual = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let image = gtk::Image::from_icon_name("image-x-generic-symbolic");
        image.set_pixel_size(40);
        visual.append(&image);
        let details = gtk::Box::new(gtk::Orientation::Vertical, 4);
        details.set_valign(gtk::Align::Center);
        let title_label = gtk::Label::new(Some(&title));
        title_label.set_xalign(0.0);
        title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        title_label.add_css_class("heading");
        let count_label = gtk::Label::new(Some(&format!(
            "{count} {}",
            if count == 1 { "memory" } else { "memories" }
        )));
        count_label.set_xalign(0.0);
        count_label.add_css_class("dim-label");
        details.append(&title_label);
        details.append(&count_label);
        visual.append(&details);
        card.append(&visual);

        let open = gtk::Button::with_label("Open Moment →");
        open.set_halign(gtk::Align::Start);
        let state = session.clone();
        let rows = list.clone();
        let target_id = id.clone();
        open.connect_clicked(move |_| {
            {
                let mut current = state.borrow_mut();
                current.section = "Library".into();
                current.library_moment = target_id.clone();
                current.library_filters_dirty = true;
                current.selecting_assets = false;
                current.selected_assets.clear();
            }
            render(&state, &rows);
        });
        card.append(&open);
        river.append(&card);
    }
    let river_scroll = gtk::ScrolledWindow::builder()
        .child(&river)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(194)
        .build();
    panel.append(&river_scroll);
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&panel));
    list.append(&row);
}

fn render_on_this_day(session: &Rc<RefCell<Session>>, list: &gtk::ListBox, room: &Value) {
    let ids: Vec<_> = room["on_this_day_asset_ids"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if ids.is_empty() {
        return;
    }
    let assets = room["assets"].as_array().into_iter().flatten();
    let mut memories = Vec::new();
    for id in ids.into_iter().take(8) {
        if let Some(asset) = assets
            .clone()
            .find(|asset| asset["id"].as_str() == Some(id) && asset["deleted"] != true)
        {
            memories.push(asset.clone());
        }
    }
    if memories.is_empty() {
        return;
    }

    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.set_margin_top(8);
    panel.set_margin_bottom(16);
    panel.set_margin_start(12);
    panel.set_margin_end(12);
    let heading = gtk::Label::new(Some("On this day"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-2");
    panel.append(&heading);
    let subtitle = gtk::Label::new(Some("Moments from earlier years"));
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("dim-label");
    panel.append(&subtitle);

    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
    for asset in memories {
        let caption = asset["caption"].as_str().filter(|value| !value.is_empty());
        let title = caption
            .or_else(|| asset["filename"].as_str())
            .unwrap_or("Memory");
        let date = asset["captured_at"]
            .as_i64()
            .and_then(|timestamp| chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0))
            .map(|date| {
                date.with_timezone(&chrono::Local)
                    .format("%b %-d, %Y")
                    .to_string()
            })
            .unwrap_or_default();
        let button = gtk::Button::with_label(&format!("{date}  ·  {title}"));
        button.set_halign(gtk::Align::Start);
        button.set_hexpand(true);
        let s = session.clone();
        let asset = asset.clone();
        let target_room = room_id.clone();
        button.connect_clicked(move |button| {
            let core = s.borrow().core.clone();
            let asset = asset.clone();
            let target_room = target_room.clone();
            let button = button.clone();
            glib::spawn_future_local(async move {
                let result = async {
                    let root = preview_root()?;
                    let filename = asset["filename"].as_str().unwrap_or("original");
                    let safe_name = std::path::Path::new(filename)
                        .file_name()
                        .ok_or_else(|| "Invalid filename".to_owned())?;
                    let path = root.join(format!(
                        "{}-{}",
                        asset["id"].as_str().unwrap_or_default(),
                        safe_name.to_string_lossy()
                    ));
                    let asset_id = asset["id"].as_str().unwrap_or_default();
                    if asset["local"] != true {
                        command(
                            core.clone(),
                            json!({"action":"fetch","room_id":target_room,"asset_id":asset_id}),
                        )
                        .await?;
                    }
                    command(
                        core,
                        json!({"action":"materialize","room_id":target_room,"asset_id":asset_id,"destination":path}),
                    )
                    .await?;
                    gio::AppInfo::launch_default_for_uri(
                        &gio::File::for_path(path).uri(),
                        None::<&gio::AppLaunchContext>,
                    )
                    .map_err(|error| error.to_string())
                }
                .await;
                if let Err(error) = result {
                    button.set_label(&error);
                }
            });
        });
        panel.append(&button);
    }
    list.append(&panel);
}

const MAX_BULK_SELECTION: usize = 500;

fn library_asset_matches(room: &Value, asset: &Value, query: &str) -> bool {
    let search_terms: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
    if search_terms.is_empty() {
        return true;
    }

    let person_names = room["people"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|person| {
            let Some(person_id) = person["id"].as_str() else {
                return false;
            };
            asset["people"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|tagged_id| tagged_id.as_str() == Some(person_id))
        })
        .filter_map(|person| person["name"].as_str())
        .collect::<Vec<_>>()
        .join(" ");

    let searchable = [
        asset["caption"].as_str().unwrap_or_default(),
        asset["filename"].as_str().unwrap_or_default(),
        asset["kind"].as_str().unwrap_or_default(),
        &person_names,
    ]
    .join(" ")
    .to_lowercase();

    search_terms
        .iter()
        .all(|term| searchable.contains(term.as_str()))
}

fn library_asset_matches_filters(
    room: &Value,
    asset: &Value,
    query: &str,
    selected_year: &str,
    selected_moment: &str,
    selected_people: &HashSet<String>,
    match_all_people: bool,
) -> bool {
    if !library_asset_matches(room, asset, query) {
        return false;
    }
    if !selected_year.is_empty() {
        let matches_year = asset["captured_at"]
            .as_i64()
            .and_then(|timestamp| glib::DateTime::from_unix_local(timestamp).ok())
            .is_some_and(|date| date.year().to_string() == selected_year);
        if !matches_year {
            return false;
        }
    }
    if !selected_moment.is_empty()
        && !room["moments"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|moment| {
                moment["id"].as_str() == Some(selected_moment)
                    && moment["asset_ids"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|id| id.as_str() == asset["id"].as_str())
            })
    {
        return false;
    }
    if selected_people.is_empty() {
        return true;
    }
    let tagged_people = asset["people"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    if match_all_people {
        selected_people
            .iter()
            .all(|person| tagged_people.contains(person))
    } else {
        selected_people
            .iter()
            .any(|person| tagged_people.contains(person))
    }
}

fn refresh_library_filter_choices(session: &Rc<RefCell<Session>>) {
    let (
        room,
        year_filter,
        moment_filter,
        people_filter,
        people_list,
        match_all_control,
        main_list,
        refreshing,
    ) = {
        let state = session.borrow();
        let Some(room) = state.snapshot["rooms"]
            .as_array()
            .and_then(|rooms| rooms.iter().find(|room| room["id"] == state.room_id))
            .cloned()
        else {
            return;
        };
        (
            room,
            state.year_filter.clone(),
            state.moment_filter.clone(),
            state.people_filter.clone(),
            state.people_filter_list.clone(),
            state.people_match_all.clone(),
            state.library_list.as_ref().and_then(glib::WeakRef::upgrade),
            state.refreshing_library_filters.clone(),
        )
    };

    let mut years: Vec<String> = room["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|asset| asset["captured_at"].as_i64())
        .filter_map(|timestamp| glib::DateTime::from_unix_local(timestamp).ok())
        .map(|date| date.year().to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    years.sort_by(|left, right| right.cmp(left));

    let mut moments: Vec<(String, String, i64)> = room["moments"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|moment| {
            Some((
                moment["id"].as_str()?.to_owned(),
                moment["title"].as_str().unwrap_or("Moment").to_owned(),
                moment["start"].as_i64().unwrap_or_default(),
            ))
        })
        .collect();
    moments.sort_by(|left, right| right.2.cmp(&left.2));

    let mut people: Vec<(String, String)> = room["people"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|person| {
            Some((
                person["id"].as_str()?.to_owned(),
                person["name"].as_str().unwrap_or("Person").to_owned(),
            ))
        })
        .collect();
    people.sort_by(|left, right| left.1.to_lowercase().cmp(&right.1.to_lowercase()));

    let (selected_year, selected_moment, selected_people, match_all_people) = {
        let mut state = session.borrow_mut();
        if !years.contains(&state.library_year) {
            state.library_year.clear();
        }
        if !moments.iter().any(|(id, _, _)| id == &state.library_moment) {
            state.library_moment.clear();
        }
        state
            .library_people
            .retain(|id| people.iter().any(|(person_id, _)| person_id == id));
        state.library_filters_dirty = false;
        (
            state.library_year.clone(),
            state.library_moment.clone(),
            state.library_people.clone(),
            state.library_match_all_people,
        )
    };

    refreshing.set(true);
    if let Some(filter) = year_filter.and_then(|weak| weak.upgrade()) {
        filter.remove_all();
        filter.append(Some(""), "All years");
        for year in &years {
            filter.append(Some(year), year);
        }
        filter.set_active_id(Some(&selected_year));
    }
    if let Some(filter) = moment_filter.and_then(|weak| weak.upgrade()) {
        filter.remove_all();
        filter.append(Some(""), "All Moments");
        for (id, title, _) in &moments {
            filter.append(Some(id), title);
        }
        filter.set_active_id(Some(&selected_moment));
    }
    if let Some(list) = people_list.and_then(|weak| weak.upgrade()) {
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        for (id, name) in &people {
            let check = gtk::CheckButton::with_label(name);
            check.set_active(selected_people.contains(id));
            list.append(&check);
            let session_state = session.clone();
            let rows = main_list.clone();
            let refreshing = refreshing.clone();
            let id = id.clone();
            check.connect_toggled(move |check| {
                if refreshing.get() {
                    return;
                }
                let mut state = session_state.borrow_mut();
                if check.is_active() {
                    state.library_people.insert(id.clone());
                } else {
                    state.library_people.remove(&id);
                    if state.library_people.is_empty() {
                        state.library_match_all_people = false;
                        state.library_filters_dirty = true;
                    }
                }
                drop(state);
                if let Some(rows) = rows.as_ref() {
                    render(&session_state, rows);
                }
            });
        }
    }
    if let Some(toggle) = match_all_control.and_then(|weak| weak.upgrade()) {
        toggle.set_active(match_all_people);
        toggle.set_sensitive(!selected_people.is_empty());
    }
    if let Some(button) = people_filter.and_then(|weak| weak.upgrade()) {
        let label = if selected_people.is_empty() {
            "People".to_owned()
        } else {
            format!("People · {}", selected_people.len())
        };
        button.set_label(&label);
    }
    refreshing.set(false);
}

fn library_asset_ids(room: &Value, include_deleted: bool) -> Vec<String> {
    room["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|asset| asset["deleted"].as_bool().unwrap_or(false) == include_deleted)
        .filter_map(|asset| asset["id"].as_str().map(str::to_owned))
        .collect()
}

fn selected_library_asset_ids(
    room: &Value,
    selected: &HashSet<String>,
    include_deleted: bool,
) -> Vec<String> {
    library_asset_ids(room, include_deleted)
        .into_iter()
        .filter(|asset_id| selected.contains(asset_id))
        .collect()
}

fn insert_selected_asset(selected: &mut HashSet<String>, asset_id: &str) -> bool {
    if selected.contains(asset_id) {
        return true;
    }
    if selected.len() >= MAX_BULK_SELECTION {
        return false;
    }
    selected.insert(asset_id.to_owned())
}

fn next_bulk_favorite_value(room: &Value, selected: &[String]) -> bool {
    let assets = room["assets"].as_array();
    !selected.iter().all(|asset_id| {
        assets.into_iter().flatten().any(|asset| {
            asset["id"].as_str() == Some(asset_id.as_str()) && asset["favorite"] == true
        })
    })
}

fn can_edit_assets(room: &Value) -> bool {
    matches!(
        room["role"].as_str(),
        Some("owner" | "admin" | "contributor")
    )
}

fn can_change_deletion_for_selection(room: &Value, device_id: &str, selected: &[String]) -> bool {
    if selected.is_empty() {
        return false;
    }
    match room["role"].as_str() {
        Some("owner" | "admin") => true,
        Some("contributor") => selected.iter().all(|asset_id| {
            room["assets"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|asset| {
                    asset["id"].as_str() == Some(asset_id.as_str())
                        && asset["contributor"].as_str() == Some(device_id)
                })
        }),
        _ => false,
    }
}

fn corrected_capture_timestamp(original: i64, year: i32, month: i32, day: i32) -> Option<i64> {
    let local = glib::DateTime::from_unix_local(original).ok()?;
    glib::DateTime::from_local(
        year,
        month,
        day,
        local.hour(),
        local.minute(),
        local.seconds(),
    )
    .ok()
    .map(|date| date.to_unix())
}

fn schedule_library_render(session: Rc<RefCell<Session>>, list: gtk::ListBox) {
    glib::idle_add_local_once(move || render(&session, &list));
}

fn submit_batch_edit(
    session: Rc<RefCell<Session>>,
    list: gtk::ListBox,
    button: gtk::Button,
    request: Value,
) {
    button.set_sensitive(false);
    glib::spawn_future_local(async move {
        let core = session.borrow().core.clone();
        let result = async {
            command(core.clone(), request).await?;
            let snapshot = command(core, json!({"action":"snapshot"})).await?;
            {
                let mut state = session.borrow_mut();
                state.snapshot = snapshot;
                state.selecting_assets = false;
                state.selected_assets.clear();
            }
            render(&session, &list);
            Ok::<(), String>(())
        }
        .await;
        if let Err(error) = result {
            button.set_label(&error);
            button.set_sensitive(true);
        }
    });
}

fn render_person_merge(session: &Rc<RefCell<Session>>, list: &gtk::ListBox, room: &Value) {
    let can_edit = matches!(
        room["role"].as_str(),
        Some("owner" | "admin" | "contributor")
    );
    let people: Vec<(String, String)> = room["people"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|person| {
            Some((
                person["id"].as_str()?.to_owned(),
                person["name"].as_str()?.to_owned(),
            ))
        })
        .collect();
    if !can_edit || people.len() < 2 {
        return;
    }

    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.set_margin_top(10);
    panel.set_margin_bottom(12);
    panel.set_margin_start(12);
    panel.set_margin_end(12);
    panel.add_css_class("card");
    let heading = gtk::Label::new(Some("Merge duplicate People"));
    heading.set_xalign(0.0);
    heading.add_css_class("heading");
    panel.append(&heading);
    let explanation = gtk::Label::new(Some(
        "Use this when two profiles show the same person. This changes People labels and tags only; it never invites anyone or grants Room access.",
    ));
    explanation.set_wrap(true);
    explanation.set_xalign(0.0);
    explanation.add_css_class("dim-label");
    panel.append(&explanation);

    let names: Vec<String> = people.iter().map(|(_, name)| name.clone()).collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let source = gtk::DropDown::from_strings(&name_refs);
    let target = gtk::DropDown::from_strings(&name_refs);
    target.set_selected(1);
    panel.append(&gtk::Label::new(Some("Profile to merge")));
    panel.append(&source);
    panel.append(&gtk::Label::new(Some("Keep this profile")));
    panel.append(&target);

    let merge = gtk::Button::with_label("Merge profiles…");
    merge.set_halign(gtk::Align::Start);
    panel.append(&merge);
    let people = people.clone();
    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
    let session = session.clone();
    let parent_list = list.clone();
    let source_picker = source.clone();
    let target_picker = target.clone();
    merge.connect_clicked(move |button| {
        let source_index = source_picker.selected() as usize;
        let target_index = target_picker.selected() as usize;
        let (Some((source_id, source_name)), Some((target_id, target_name))) =
            (people.get(source_index), people.get(target_index))
        else {
            return;
        };
        if source_id == target_id {
            button.set_label("Choose two different profiles");
            return;
        }
        let Some(parent) = parent_list
            .root()
            .and_then(|root| root.downcast::<gtk::Window>().ok())
        else {
            return;
        };
        let dialog = gtk::MessageDialog::builder()
            .transient_for(&parent)
            .modal(true)
            .text(format!("Merge {source_name} into {target_name}?"))
            .secondary_text("The kept profile will contain both sets of photo tags. This correction applies across this Room.")
            .build();
        dialog.add_button("Cancel", gtk::ResponseType::Cancel);
        dialog.add_button("Merge profiles", gtk::ResponseType::Accept);
        let request = json!({
            "action": "merge_person",
            "room_id": room_id,
            "source_person_id": source_id,
            "target_person_id": target_id,
        });
        let session = session.clone();
        let list = parent_list.clone();
        let button = button.clone();
        dialog.connect_response(move |dialog, response| {
            if response == gtk::ResponseType::Accept {
                submit_batch_edit(session.clone(), list.clone(), button.clone(), request.clone());
            }
            dialog.close();
        });
        dialog.present();
    });
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&panel));
    list.append(&row);
}

fn render_photo_duplicate_review(
    session: &Rc<RefCell<Session>>,
    list: &gtk::ListBox,
    room: &Value,
) {
    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
    let core = session.borrow().core.clone();
    let room_snapshot = room.clone();
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.set_margin_top(10);
    panel.set_margin_bottom(12);
    panel.set_margin_start(12);
    panel.set_margin_end(12);
    panel.add_css_class("card");

    let title = gtk::Label::new(Some("Possible look-alikes"));
    title.set_xalign(0.0);
    title.add_css_class("title-3");
    panel.append(&title);
    let explanation = gtk::Label::new(Some(
        "A private visual check. Compare each suggestion before making changes.",
    ));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.add_css_class("dim-label");
    panel.append(&explanation);

    let scan = gtk::Button::with_label("Find similar photos");
    scan.set_halign(gtk::Align::Start);
    let results = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let results_for_scan = results.clone();
    let room_id_for_scan = room_id.clone();
    scan.connect_clicked(move |button| {
        button.set_sensitive(false);
        button.set_label("Checking photos…");
        let core = core.clone();
        let results = results_for_scan.clone();
        let room_id = room_id_for_scan.clone();
        let room_snapshot = room_snapshot.clone();
        let button = button.clone();
        glib::spawn_future_local(async move {
            let result = command(
                core,
                json!({"action":"suggest_photo_duplicates","room_id":room_id}),
            )
            .await;
            while let Some(child) = results.first_child() {
                results.remove(&child);
            }
            match result {
                Ok(response) => {
                    let indexed = response["indexed_photos"].as_u64().unwrap_or(0);
                    let unavailable = response["unavailable_locally"].as_u64().unwrap_or(0);
                    let skipped = response["unsupported_or_oversized"].as_u64().unwrap_or(0);
                    let status = gtk::Label::new(Some(&format!(
                        "{indexed} checked on this device · {unavailable} originals unavailable · {skipped} skipped"
                    )));
                    status.set_xalign(0.0);
                    status.set_wrap(true);
                    status.add_css_class("dim-label");
                    results.append(&status);

                    let suggestions = response["suggestions"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    if suggestions.is_empty() {
                        let empty = gtk::Label::new(Some(
                            "No close visual matches were found in the photos checked.",
                        ));
                        empty.set_xalign(0.0);
                        empty.set_wrap(true);
                        results.append(&empty);
                    }
                    for suggestion in suggestions.iter().take(30) {
                        let pair = gtk::Box::new(gtk::Orientation::Vertical, 4);
                        pair.add_css_class("card");
                        for side in ["first", "second"] {
                            let asset_id = suggestion[side]["asset_id"].as_str();
                            let asset = room_snapshot["assets"].as_array().and_then(|assets| {
                                assets.iter().find(|asset| {
                                    asset["id"].as_str() == asset_id
                                        && asset["deleted"] != true
                                })
                            });
                            if let Some(asset) = asset {
                                let filename = asset["caption"]
                                    .as_str()
                                    .filter(|caption| !caption.is_empty())
                                    .or(asset["filename"].as_str())
                                    .unwrap_or("Photo");
                                let item = gtk::Label::new(Some(filename));
                                item.set_xalign(0.0);
                                item.set_wrap(true);
                                pair.append(&item);
                            }
                        }
                        results.append(&pair);
                    }
                    if response["comparison_limit_reached"] == true
                        || response["result_limit_reached"] == true
                    {
                        let limit = gtk::Label::new(Some(
                            "Showing a bounded set of matches. Review these before checking again.",
                        ));
                        limit.set_xalign(0.0);
                        limit.set_wrap(true);
                        limit.add_css_class("dim-label");
                        results.append(&limit);
                    }
                }
                Err(error) => {
                    let failure = gtk::Label::new(Some(&error));
                    failure.set_xalign(0.0);
                    failure.set_wrap(true);
                    results.append(&failure);
                }
            }
            button.set_label("Check again");
            button.set_sensitive(true);
        });
    });
    panel.append(&scan);
    panel.append(&results);
    list.append(&panel);
}

fn render_library_batch_actions(session: &Rc<RefCell<Session>>, list: &gtk::ListBox, room: &Value) {
    let state = session.borrow();
    let selecting = state.selecting_assets;
    let is_trash = state.section == "Recently Deleted";
    let selected = selected_library_asset_ids(room, &state.selected_assets, is_trash);
    let room_id = state.room_id.clone();
    let device_id = state.snapshot["device_id"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    let can_edit = can_edit_assets(room);
    drop(state);

    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.set_margin_top(8);
    panel.set_margin_bottom(8);
    panel.set_margin_start(12);
    panel.set_margin_end(12);
    panel.add_css_class("card");

    if !selecting {
        let select = gtk::Button::with_label(if is_trash {
            "Select deleted memories"
        } else {
            "Select memories"
        });
        select.set_halign(gtk::Align::Start);
        select.set_sensitive(can_edit);
        let state = session.clone();
        let rows = list.clone();
        select.connect_clicked(move |_| {
            state.borrow_mut().selecting_assets = true;
            schedule_library_render(state.clone(), rows.clone());
        });
        panel.append(&select);
        list.append(&panel);
        return;
    }

    let selection_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let count_text = if selected.len() == MAX_BULK_SELECTION {
        format!("{} selected · maximum per batch", selected.len())
    } else {
        format!("{} selected", selected.len())
    };
    let count = gtk::Label::new(Some(&count_text));
    count.set_xalign(0.0);
    count.set_hexpand(true);
    count.add_css_class("dim-label");
    selection_row.append(&count);

    let select_all = gtk::Button::with_label("Select up to 500");
    select_all.set_sensitive(can_edit);
    let state = session.clone();
    let rows = list.clone();
    let all_ids: Vec<_> = library_asset_ids(room, is_trash)
        .into_iter()
        .take(MAX_BULK_SELECTION)
        .collect();
    select_all.connect_clicked(move |_| {
        state.borrow_mut().selected_assets = all_ids.iter().cloned().collect();
        schedule_library_render(state.clone(), rows.clone());
    });
    selection_row.append(&select_all);

    let clear = gtk::Button::with_label("Clear");
    clear.set_sensitive(!selected.is_empty());
    let state = session.clone();
    let rows = list.clone();
    clear.connect_clicked(move |_| {
        state.borrow_mut().selected_assets.clear();
        schedule_library_render(state.clone(), rows.clone());
    });
    selection_row.append(&clear);

    let done = gtk::Button::with_label("Done");
    let state = session.clone();
    let rows = list.clone();
    done.connect_clicked(move |_| {
        let mut current = state.borrow_mut();
        current.selecting_assets = false;
        current.selected_assets.clear();
        drop(current);
        schedule_library_render(state.clone(), rows.clone());
    });
    selection_row.append(&done);
    panel.append(&selection_row);

    if !selected.is_empty() {
        if is_trash {
            let restore = gtk::Button::with_label("Restore selected");
            restore.add_css_class("suggested-action");
            restore.set_sensitive(can_change_deletion_for_selection(
                room, &device_id, &selected,
            ));
            let state = session.clone();
            let rows = list.clone();
            let target_room = room_id.clone();
            let ids = selected.clone();
            restore.connect_clicked(move |button| {
                submit_batch_edit(
                    state.clone(),
                    rows.clone(),
                    button.clone(),
                    json!({"action":"patch_assets","room_id":target_room,
                        "asset_ids":ids,"fields":{"deleted":false}}),
                );
            });
            panel.append(&restore);
            list.append(&panel);
            return;
        }

        let edit_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let favorite = gtk::Button::with_label(if next_bulk_favorite_value(room, &selected) {
            "Favorite"
        } else {
            "Unfavorite"
        });
        favorite.set_sensitive(can_edit);
        let state = session.clone();
        let rows = list.clone();
        let ids = selected.clone();
        let should_favorite = next_bulk_favorite_value(room, &selected);
        let target_room = room_id.clone();
        favorite.connect_clicked(move |button| {
            submit_batch_edit(
                state.clone(),
                rows.clone(),
                button.clone(),
                json!({"action":"patch_assets","room_id":target_room,
                    "asset_ids":ids,"fields":{"favorite":should_favorite}}),
            );
        });
        edit_row.append(&favorite);

        let correct_date = gtk::Button::with_label("Correct date");
        correct_date.set_sensitive(can_edit);
        let state = session.clone();
        let rows = list.clone();
        let ids = selected.clone();
        let target_room = room_id.clone();
        let selected_capture_dates: Vec<_> = room["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|asset| ids.iter().any(|id| asset["id"].as_str() == Some(id)))
            .filter_map(|asset| {
                Some((
                    asset["id"].as_str()?.to_owned(),
                    asset["captured_at"].as_i64()?,
                ))
            })
            .collect();
        let picker = gtk::Calendar::new();
        if let Some((_, timestamp)) = selected_capture_dates.first() {
            if let Ok(date) = glib::DateTime::from_unix_local(*timestamp) {
                picker.select_day(&date);
            }
        }
        let date_window = gtk::Window::builder()
            .title("Correct capture date")
            .modal(true)
            .default_width(360)
            .default_height(420)
            .build();
        let date_content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        date_content.set_margin_top(18);
        date_content.set_margin_bottom(18);
        date_content.set_margin_start(18);
        date_content.set_margin_end(18);
        let date_note = gtk::Label::new(Some(
            "Choose a new calendar date. Each memory keeps its local time of day.",
        ));
        date_note.set_wrap(true);
        date_content.append(&date_note);
        date_content.append(&picker);
        let apply_date = gtk::Button::with_label("Apply to selected memories");
        apply_date.add_css_class("suggested-action");
        let apply_state = state.clone();
        let apply_rows = rows.clone();
        let apply_ids = ids.clone();
        let apply_room = target_room.clone();
        let apply_dates = selected_capture_dates.clone();
        let window_to_close = date_window.clone();
        let calendar = picker.clone();
        apply_date.connect_clicked(move |button| {
            let target = calendar.date();
            let updates: Vec<_> = apply_ids
                .iter()
                .filter_map(|asset_id| {
                    let original = apply_dates
                        .iter()
                        .find(|(id, _)| id == asset_id)
                        .map(|(_, timestamp)| *timestamp)?;
                    Some(json!({
                        "asset_id": asset_id,
                        "captured_at": corrected_capture_timestamp(
                            original,
                            target.year(),
                            target.month(),
                            target.day_of_month(),
                        )?,
                    }))
                })
                .collect();
            if updates.len() != apply_ids.len() {
                button.set_label("A selected memory has no date");
                return;
            }
            submit_batch_edit(
                apply_state.clone(),
                apply_rows.clone(),
                button.clone(),
                json!({"action":"patch_asset_dates","room_id":apply_room,
                    "dates":updates}),
            );
            window_to_close.close();
        });
        date_content.append(&apply_date);
        date_window.set_child(Some(&date_content));
        let window_to_open = date_window.clone();
        correct_date.connect_clicked(move |_| window_to_open.present());
        edit_row.append(&correct_date);

        let people: Vec<(String, String)> = room["people"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|person| {
                Some((
                    person["id"].as_str()?.to_owned(),
                    person["name"].as_str()?.to_owned(),
                ))
            })
            .collect();
        let people_names: Vec<_> = people.iter().map(|(_, name)| name.as_str()).collect();
        let person_picker = gtk::DropDown::from_strings(&people_names);
        person_picker.set_hexpand(true);
        person_picker.set_sensitive(can_edit && !people.is_empty());
        edit_row.append(&person_picker);
        for (label, remove) in [("Add Person", false), ("Remove Person", true)] {
            let tag = gtk::Button::with_label(label);
            tag.set_sensitive(can_edit && !people.is_empty());
            let state = session.clone();
            let rows = list.clone();
            let ids = selected.clone();
            let people = people.clone();
            let picker = person_picker.clone();
            let target_room = room_id.clone();
            tag.connect_clicked(move |button| {
                let Some((person_id, _)) = people.get(picker.selected() as usize) else {
                    return;
                };
                submit_batch_edit(
                    state.clone(),
                    rows.clone(),
                    button.clone(),
                    json!({"action":"tag_assets","room_id":target_room,
                        "person_id":person_id,"asset_ids":ids,"remove":remove}),
                );
            });
            edit_row.append(&tag);
        }
        panel.append(&edit_row);

        let album_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let album_title = gtk::Entry::builder()
            .placeholder_text("New album name")
            .hexpand(true)
            .build();
        let create_album = gtk::Button::with_label("Create album");
        create_album.set_sensitive(can_edit);
        let state = session.clone();
        let rows = list.clone();
        let ids = selected.clone();
        let target_room = room_id.clone();
        let title_entry = album_title.clone();
        create_album.connect_clicked(move |button| {
            let title = title_entry.text().trim().to_owned();
            if title.is_empty() {
                title_entry.set_placeholder_text(Some("Enter an album name"));
                return;
            }
            submit_batch_edit(
                state.clone(),
                rows.clone(),
                button.clone(),
                json!({"action":"save_album","room_id":target_room,
                    "title":title,"asset_ids":ids}),
            );
        });
        album_row.append(&album_title);
        album_row.append(&create_album);
        panel.append(&album_row);

        let may_trash = can_change_deletion_for_selection(room, &device_id, &selected);
        let trash_row = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let trash_button = gtk::Button::with_label("Move to Recently Deleted");
        trash_button.add_css_class("destructive-action");
        trash_button.set_sensitive(may_trash);
        let dialog = gtk::Window::builder()
            .title("Move memories to Recently Deleted?")
            .modal(true)
            .default_width(380)
            .build();
        let dialog_content = gtk::Box::new(gtk::Orientation::Vertical, 14);
        dialog_content.set_margin_top(20);
        dialog_content.set_margin_bottom(20);
        dialog_content.set_margin_start(20);
        dialog_content.set_margin_end(20);
        let explanation = gtk::Label::new(Some(
            "These memories will leave the Library. Their originals stay saved and can be restored later.",
        ));
        explanation.set_wrap(true);
        dialog_content.append(&explanation);
        let dialog_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        dialog_actions.set_halign(gtk::Align::End);
        let cancel = gtk::Button::with_label("Cancel");
        let close = dialog.clone();
        cancel.connect_clicked(move |_| close.close());
        dialog_actions.append(&cancel);
        let confirm = gtk::Button::with_label("Move to Recently Deleted");
        confirm.add_css_class("destructive-action");
        let confirm_state = session.clone();
        let confirm_rows = list.clone();
        let confirm_room = room_id.clone();
        let confirm_ids = selected.clone();
        let close = dialog.clone();
        confirm.connect_clicked(move |button| {
            submit_batch_edit(
                confirm_state.clone(),
                confirm_rows.clone(),
                button.clone(),
                json!({"action":"patch_assets","room_id":confirm_room,
                    "asset_ids":confirm_ids,"fields":{"deleted":true}}),
            );
            close.close();
        });
        dialog_actions.append(&confirm);
        dialog_content.append(&dialog_actions);
        dialog.set_child(Some(&dialog_content));
        let to_present = dialog.clone();
        trash_button.connect_clicked(move |_| to_present.present());
        trash_row.append(&trash_button);
        if !may_trash {
            let restriction = gtk::Label::new(Some(
                "Select only memories you contributed, or ask a Room owner or admin.",
            ));
            restriction.add_css_class("dim-label");
            restriction.set_xalign(0.0);
            trash_row.append(&restriction);
        }
        panel.append(&trash_row);
    }

    list.append(&panel);
}

fn append_asset_selection(
    row: &gtk::Box,
    list: &gtk::ListBox,
    session: &Rc<RefCell<Session>>,
    room: &Value,
    item: &Value,
) {
    let state = session.borrow();
    if !state.selecting_assets {
        return;
    }
    let Some(asset_id) = item["id"].as_str() else {
        return;
    };
    let asset_id = asset_id.to_owned();
    let selected = state.selected_assets.contains(&asset_id);
    let can_select =
        can_edit_assets(room) && (selected || state.selected_assets.len() < MAX_BULK_SELECTION);
    drop(state);

    let checkbox = gtk::CheckButton::with_label("Select");
    checkbox.set_active(selected);
    checkbox.set_sensitive(can_select);
    checkbox.set_tooltip_text(Some("Select this memory for a batch action"));
    let state = session.clone();
    let rows = list.clone();
    checkbox.connect_toggled(move |checkbox| {
        let mut session = state.borrow_mut();
        if checkbox.is_active() {
            insert_selected_asset(&mut session.selected_assets, &asset_id);
        } else {
            session.selected_assets.remove(&asset_id);
        }
        drop(session);
        schedule_library_render(state.clone(), rows.clone());
    });
    row.append(&checkbox);
}

fn append_room_setup_step(panel: &gtk::Box, title: &str, complete: bool, detail: &str) {
    let item = gtk::Box::new(gtk::Orientation::Vertical, 3);
    let check = gtk::CheckButton::with_label(title);
    check.set_active(complete);
    check.set_sensitive(false);
    item.append(&check);

    let explanation = gtk::Label::new(Some(detail));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.add_css_class("dim-label");
    item.append(&explanation);
    panel.append(&item);
}

fn render_room_setup_checklist(session: &Rc<RefCell<Session>>, list: &gtk::ListBox, room: &Value) {
    let (snapshot, recovery_key) = {
        let state = session.borrow();
        (state.snapshot.clone(), state.recovery_key.clone())
    };
    let status = room_setup_status(room, &snapshot, &recovery_key);
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 10);
    panel.add_css_class("card");
    panel.set_margin_bottom(12);

    let heading = gtk::Label::new(Some("Get your Room ready"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-2");
    panel.append(&heading);

    if !status.can_manage_storage {
        let member_note = gtk::Label::new(Some(
            "The Room owner manages shared storage and upload destinations. You can join without an account on the owner's storage provider.",
        ));
        member_note.set_xalign(0.0);
        member_note.set_wrap(true);
        member_note.add_css_class("dim-label");
        panel.append(&member_note);
    } else {
        append_room_setup_step(
            &panel,
            "Choose preferred storage",
            status.has_preferred_storage,
            if status.has_preferred_storage {
                "A destination is ready for new Room uploads."
            } else {
                "Continue in Storage to connect a source and choose the destination for this Room."
            },
        );
        append_room_setup_step(
            &panel,
            "Choose an automatic upload folder",
            status.has_active_folder_watch,
            if status.has_active_folder_watch {
                "This Room checks the selected folder while Family Room is open."
            } else {
                "Set up a folder below; automatic scans run while Family Room is open. Manual import is always available."
            },
        );
        append_room_setup_step(
            &panel,
            "Recovery key available on this device",
            status.recovery_key_available,
            if status.recovery_key_available {
                "The key is stored in the OS credential store. Reveal it in the left pane and save it separately before losing this device."
            } else {
                "Set up a recovery key before adding memories."
            },
        );
        append_room_setup_step(
            &panel,
            "Add your first photos or videos",
            status.has_first_contribution,
            if status.has_first_contribution {
                "This Room already has a contribution."
            } else {
                "Use Import photos and videos above to add the first memories."
            },
        );
    }

    list.append(&panel);
}

fn render_upload_watch_controls(session: &Rc<RefCell<Session>>, list: &gtk::ListBox, room: &Value) {
    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
    let room_name = room["name"].as_str().unwrap_or("this Room").to_owned();
    let role = room["role"].as_str().unwrap_or("viewer");
    let watches = session.borrow().snapshot["watches"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.add_css_class("card");
    panel.set_margin_bottom(12);

    let heading = gtk::Label::new(Some("Automatic uploads from this device"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-3");
    panel.append(&heading);

    let explanation = gtk::Label::new(Some(&format!(
        "Choose a local folder to share new files directly inside it into {room_name}. Family Room checks watched folders while this app is open; it does not run as a background service after you quit."
    )));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.add_css_class("dim-label");
    panel.append(&explanation);

    if role == "viewer" || role == "removed" {
        let note = gtk::Label::new(Some(
            "Your Room role does not allow contributions. Ask an owner to update your access.",
        ));
        note.set_xalign(0.0);
        note.set_wrap(true);
        panel.append(&note);
        list.append(&panel);
        return;
    }

    let include_existing = gtk::CheckButton::with_label(
        "Also import existing files in this folder (historical import)",
    );
    panel.append(&include_existing);
    let excluded_extensions = gtk::Entry::builder()
        .placeholder_text("Excluded extensions, comma separated (for example: mp4, heic)")
        .hexpand(true)
        .build();
    panel.append(&excluded_extensions);
    let feedback = gtk::Label::new(None);
    feedback.set_xalign(0.0);
    feedback.set_wrap(true);
    feedback.add_css_class("dim-label");
    panel.append(&feedback);

    let add_watch = gtk::Button::with_label("Choose folder and start automatic uploads");
    add_watch.set_halign(gtk::Align::Start);
    let add_session = session.clone();
    let add_list = list.clone();
    let historical = include_existing.clone();
    let exclusions = excluded_extensions.clone();
    let message = feedback.clone();
    let target_room = room_id.clone();
    let target_room_name = room_name.clone();
    add_watch.connect_clicked(move |_| {
        let Some(parent) = add_session
            .borrow()
            .parent_window
            .as_ref()
            .and_then(|window| window.upgrade())
        else {
            message.set_text("The folder picker is unavailable.");
            return;
        };
        let dialog = gtk::FileDialog::builder()
            .title("Choose a folder to share with this Room")
            .build();
        let add_session = add_session.clone();
        let add_list = add_list.clone();
        let historical = historical.is_active();
        let excluded_extensions = exclusions.text().to_string();
        let target_room = target_room.clone();
        let target_room_name = target_room_name.clone();
        let message = message.clone();
        glib::spawn_future_local(async move {
            match dialog.select_folder_future(Some(&parent)).await {
                Ok(folder) => match folder.path() {
                    Some(path) => {
                        let path = path.to_string_lossy().into_owned();
                        let extensions = excluded_extensions
                            .split(',')
                            .map(str::trim)
                            .filter(|extension| !extension.is_empty())
                            .collect::<Vec<_>>();
                        let core = add_session.borrow().core.clone();
                        let result = command(
                            core.clone(),
                            json!({
                                "action": "watch_folder",
                                "room_id": target_room,
                                "path": path,
                                "include_existing": historical,
                                "excluded_extensions": extensions,
                            }),
                        )
                        .await;
                        match result {
                            Ok(_) => {
                                let scan = command(core.clone(), json!({"action":"scan_watches"}))
                                    .await
                                    .unwrap_or_else(|error| json!({"errors":[error]}));
                                let _ = command(
                                    core.clone(),
                                    json!({"action":"sync_all","room_id":target_room}),
                                )
                                .await;
                                if let Ok(snapshot) =
                                    command(core, json!({"action":"snapshot"})).await
                                {
                                    add_session.borrow_mut().snapshot = snapshot;
                                }
                                let imported = scan["imported"].as_u64().unwrap_or_default();
                                message.set_text(&format!("Folder is connected to {target_room_name}. {imported} existing items were imported; future files will be checked while Family Room is open."));
                                render(&add_session, &add_list);
                            }
                            Err(error) => message.set_text(&error),
                        }
                    }
                    None => message.set_text("The selected folder has no local path."),
                },
                Err(error) => message.set_text(&format!("Could not open folder: {error}")),
            }
        });
    });
    panel.append(&add_watch);

    for watch in watches.iter().filter(|watch| {
        watch["room_id"].as_str() == Some(room_id.as_str())
            && watch["path"]
                .as_str()
                .is_some_and(|path| !path.starts_with("photos://"))
    }) {
        let watch_id = watch["id"].as_str().unwrap_or_default().to_owned();
        let watch_path = watch["path"].as_str().unwrap_or_default();
        let paused = watch["paused"].as_bool().unwrap_or(false);
        let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let label = gtk::Label::new(Some(watch_path));
        label.set_xalign(0.0);
        label.set_wrap(true);
        row.append(&label);

        let row_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let exclusions = watch["excluded_extensions"]
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let exclusion_entry = gtk::Entry::builder()
            .text(&exclusions)
            .placeholder_text("Excluded extensions")
            .hexpand(true)
            .build();
        row_actions.append(&exclusion_entry);

        let update = gtk::Button::with_label("Save exclusions");
        let update_session = session.clone();
        let update_list = list.clone();
        let update_id = watch_id.clone();
        let update_entry = exclusion_entry.clone();
        let update_message = feedback.clone();
        update.connect_clicked(move |_| {
            let core = update_session.borrow().core.clone();
            let extensions = update_entry
                .text()
                .split(',')
                .map(str::trim)
                .filter(|extension| !extension.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let session = update_session.clone();
            let list = update_list.clone();
            let id = update_id.clone();
            let message = update_message.clone();
            glib::spawn_future_local(async move {
                if let Err(error) = command(
                    core.clone(),
                    json!({"action":"set_watch_exclusions","id":id,"excluded_extensions":extensions}),
                )
                .await
                {
                    message.set_text(&error);
                    return;
                }
                if let Ok(snapshot) = command(core, json!({"action":"snapshot"})).await {
                    session.borrow_mut().snapshot = snapshot;
                }
                message.set_text("Upload exclusions saved.");
                render(&session, &list);
            });
        });
        row_actions.append(&update);

        let pause = gtk::Button::with_label(if paused { "Resume" } else { "Pause" });
        let pause_session = session.clone();
        let pause_list = list.clone();
        let pause_id = watch_id.clone();
        pause.connect_clicked(move |_| {
            let core = pause_session.borrow().core.clone();
            let session = pause_session.clone();
            let list = pause_list.clone();
            let id = pause_id.clone();
            glib::spawn_future_local(async move {
                if command(
                    core.clone(),
                    json!({"action":"pause_watch","id":id,"paused":!paused}),
                )
                .await
                .is_ok()
                {
                    if let Ok(snapshot) = command(core, json!({"action":"snapshot"})).await {
                        session.borrow_mut().snapshot = snapshot;
                    }
                    render(&session, &list);
                }
            });
        });
        row_actions.append(&pause);
        row.append(&row_actions);
        panel.append(&row);
    }

    list.append(&panel);
}

fn set_room_storage_controls_visible(session: &Session, visible: bool) {
    if let Some(entry) = session
        .storage_gateway_entry
        .as_ref()
        .and_then(|widget| widget.upgrade())
    {
        entry.set_visible(visible);
    }
    if let Some(button) = session
        .storage_gateway_connect
        .as_ref()
        .and_then(|widget| widget.upgrade())
    {
        button.set_visible(visible);
    }
}

fn render(session: &Rc<RefCell<Session>>, list: &gtk::ListBox) {
    let refresh_filters = {
        let state = session.borrow();
        matches!(state.section.as_str(), "Library" | "Recently Deleted")
            && state.library_filters_dirty
    };
    if refresh_filters {
        refresh_library_filter_choices(session);
    }
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let state = session.borrow();
    if state.section == "Room settings" {
        let pending = state.snapshot["pending_invitations"].clone();
        let room = state.snapshot["rooms"]
            .as_array()
            .and_then(|rooms| rooms.iter().find(|r| r["id"] == state.room_id))
            .cloned();
        set_room_storage_controls_visible(
            &state,
            room.as_ref()
                .is_some_and(|room| matches!(room["role"].as_str(), Some("owner" | "admin"))),
        );
        drop(state);
        render_pending_invitations(session, list, &pending);
        if let Some(room) = room {
            render_room_setup_checklist(session, list, &room);
            render_upload_watch_controls(session, list, &room);
            render_room_members(session, list, &room);
            render_leave_room_control(session, list, &room);
        }
        return;
    }
    let Some(room) = state.snapshot["rooms"]
        .as_array()
        .and_then(|a| a.iter().find(|r| r["id"] == state.room_id))
    else {
        set_room_storage_controls_visible(&state, false);
        return;
    };
    set_room_storage_controls_visible(
        &state,
        matches!(room["role"].as_str(), Some("owner" | "admin")),
    );
    if state.section == "Home"
        && matches!(
            room["role"].as_str(),
            Some("owner" | "admin" | "contributor")
        )
    {
        render_moment_suggestions(session, list, room, state.core.clone());
    }
    if state.section == "Home" {
        render_on_this_day(session, list, room);
        render_moment_river(session, list, room, &state.moment_river_year);
    }
    if state.section == "Storage" {
        if matches!(room["role"].as_str(), Some("owner" | "admin")) {
            render_storage_sources(session, list, &state.snapshot, &state.room_id);
        } else {
            let heading = gtk::Label::new(Some("Room storage"));
            heading.set_xalign(0.0);
            heading.add_css_class("title-2");
            list.append(&heading);
            let owner_note = gtk::Label::new(Some(
                "The Room owner manages shared storage and upload destinations. You can join without an account on the owner's storage provider.",
            ));
            owner_note.set_xalign(0.0);
            owner_note.set_wrap(true);
            owner_note.add_css_class("dim-label");
            list.append(&owner_note);
        }
        return;
    }
    if state.section == "Ask your videos" {
        return;
    }
    if state.section == "Albums" {
        render_collections(session, list, room, "album");
        return;
    }
    if state.section == "Theater" {
        render_collections(session, list, room, "playlist");
    }
    let field = match state.section.as_str() {
        "Albums" => "albums",
        "People" => "people",
        "Theater" => "stories",
        _ => "assets",
    };
    if state.section == "People" {
        render_person_merge(session, list, room);
    }
    if state.section == "Library" {
        render_photo_duplicate_review(session, list, room);
    }
    if field == "assets" && matches!(state.section.as_str(), "Library" | "Recently Deleted") {
        render_library_batch_actions(session, list, room);
    }
    if field == "assets"
        && state.section == "Recently Deleted"
        && library_asset_ids(room, true).is_empty()
    {
        let empty = gtk::Label::new(Some(
            "Recently Deleted is empty. Original files remain safe in the Room.",
        ));
        empty.set_wrap(true);
        empty.set_xalign(0.0);
        empty.add_css_class("dim-label");
        let row = gtk::ListBoxRow::new();
        row.set_child(Some(&empty));
        list.append(&row);
    }
    if let Some(items) = room[field].as_array() {
        for item in items {
            let is_asset = field == "assets";
            let is_in_trash = state.section == "Recently Deleted";
            if is_asset && item["deleted"].as_bool().unwrap_or(false) != is_in_trash {
                continue;
            }
            if is_asset
                && matches!(state.section.as_str(), "Library" | "Recently Deleted")
                && !library_asset_matches_filters(
                    room,
                    item,
                    &state.library_search,
                    &state.library_year,
                    &state.library_moment,
                    &state.library_people,
                    state.library_match_all_people,
                )
            {
                continue;
            }
            let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
            row.set_margin_top(12);
            row.set_margin_bottom(12);
            row.set_margin_start(12);
            row.set_margin_end(12);
            let title = item["caption"]
                .as_str()
                .filter(|s| !s.is_empty())
                .or(item["filename"].as_str())
                .or(item["title"].as_str())
                .or(item["name"].as_str())
                .unwrap_or("Memory");
            let label = gtk::Label::new(Some(title));
            label.set_xalign(0.0);
            label.add_css_class("heading");
            row.append(&label);
            if is_asset && matches!(state.section.as_str(), "Library" | "Recently Deleted") {
                append_asset_selection(&row, list, session, room, item);
            }
            if field == "assets" {
                let label = gtk::Label::new(item["availability"].as_str());
                label.set_xalign(0.0);
                label.add_css_class("dim-label");
                row.append(&label);
                let button = gtk::Button::with_label("Open original");
                let asset = item.clone();
                let state = session.clone();
                button.connect_clicked(move|button| { let core=state.borrow().core.clone(); let asset=asset.clone(); let button=button.clone(); glib::spawn_future_local(async move {
                button.set_sensitive(false);
                let result=async {
                    let root=preview_root()?;
                    let filename=asset["filename"].as_str().unwrap_or("original"); let safe=std::path::Path::new(filename).file_name().ok_or("Invalid filename")?;
                    let path=root.join(format!("{}-{}",asset["id"].as_str().unwrap_or(""),safe.to_string_lossy()));
                    if asset["local"]!=true { command(core.clone(),json!({"action":"fetch","room_id":asset["room_id"],"asset_id":asset["id"]})).await?; }
                    command(core,json!({"action":"materialize","room_id":asset["room_id"],"asset_id":asset["id"],"destination":path})).await?;
                    gio::AppInfo::launch_default_for_uri(&gio::File::for_path(path).uri(),None::<&gio::AppLaunchContext>).map_err(|e|e.to_string())?; Ok::<(),String>(())
                }.await;
                if let Err(error)=result { button.set_label(&error); } button.set_sensitive(true);
            }); });
                row.append(&button);
                if item["kind"] == "video" && !item["deleted"].as_bool().unwrap_or(false) {
                    let export = gtk::Button::with_label("Export spoken captions");
                    let state = session.clone();
                    let list = list.clone();
                    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
                    let asset_id = item["id"].as_str().unwrap_or_default().to_owned();
                    export.connect_clicked(move |button| {
                        let Some(parent) = list.root().and_then(|root| root.downcast::<gtk::Window>().ok()) else {
                            return;
                        };
                        let state = state.clone();
                        let button = button.clone();
                        let parent = parent.clone();
                        let room_id = room_id.clone();
                        let asset_id = asset_id.clone();
                        glib::spawn_future_local(async move {
                            button.set_sensitive(false);
                            let result = async {
                                let core = state.borrow().core.clone();
                                let result = command(core, json!({
                                    "action":"export_video_transcript",
                                    "room_id":room_id,
                                    "asset_id":asset_id,
                                })).await?;
                                if result["cue_count"].as_u64().unwrap_or(0) == 0 {
                                    return Err("No spoken captions are indexed yet. Use Ask your videos first.".to_owned());
                                }
                                let webvtt = result["webvtt"].as_str().ok_or("Caption export was empty")?;
                                let dialog = gtk::FileDialog::builder()
                                    .title("Save spoken captions")
                                    .initial_name("captions.vtt")
                                    .build();
                                let file = dialog.save_future(Some(&parent)).await.map_err(|error| error.to_string())?;
                                let path = file.path().ok_or("Choose a local file location")?;
                                std::fs::write(path, webvtt).map_err(|error| error.to_string())?;
                                Ok::<(), String>(())
                            }.await;
                            match result {
                                Ok(()) => button.set_label("Captions saved"),
                                Err(error) => button.set_label(&error),
                            }
                            button.set_sensitive(true);
                        });
                    });
                    row.append(&export);
                }
                let asset_id = item["id"].as_str().unwrap_or("").to_owned();
                let liked_reactions: Vec<_> = room["reactions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|reaction| {
                        reaction["asset_id"] == asset_id && reaction["liked"] == true
                    })
                    .collect();
                let liked = liked_reactions
                    .iter()
                    .any(|reaction| reaction["author"] == session.borrow().snapshot["device_id"]);
                let reaction = gtk::Button::with_label(&format!(
                    "{} · {}",
                    if liked { "Unlike" } else { "Like" },
                    liked_reactions.len()
                ));
                reaction.set_sensitive(room["role"] != "removed");
                let s = session.clone();
                let l = list.clone();
                let room_id = session.borrow().room_id.clone();
                let note_room_id = room_id.clone();
                let note_asset_id = asset_id.clone();
                reaction.connect_clicked(move |button| {
                    submit_collection(
                        s.clone(),
                        l.clone(),
                        button.clone(),
                        json!({"action":"set_like","room_id":room_id,"asset_id":asset_id,"liked":!liked}),
                    );
                });
                row.append(&reaction);
                let notes = room["comments"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|note| note["asset_id"] == note_asset_id)
                    .collect::<Vec<_>>();
                let expander = gtk::Expander::new(Some(&format!("Family notes · {}", notes.len())));
                let notes_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
                for note in notes {
                    let note_card = gtk::Box::new(gtk::Orientation::Vertical, 3);
                    let body = gtk::Label::new(note["body"].as_str());
                    body.set_xalign(0.0);
                    body.set_wrap(true);
                    body.set_selectable(true);
                    note_card.append(&body);
                    let author = note["author"].as_str().unwrap_or("Member");
                    let attribution = if note["author"] == session.borrow().snapshot["device_id"] {
                        "You".to_owned()
                    } else {
                        format!("Member {}", author.chars().take(8).collect::<String>())
                    };
                    let byline = gtk::Label::new(Some(&attribution));
                    byline.set_xalign(0.0);
                    byline.add_css_class("dim-label");
                    note_card.append(&byline);
                    notes_panel.append(&note_card);
                }
                let composer = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                let note_entry = gtk::Entry::builder()
                    .placeholder_text("Leave a family note")
                    .hexpand(true)
                    .max_length(10_000)
                    .build();
                let post_note = gtk::Button::with_label("Post note");
                post_note.set_sensitive(false);
                {
                    let post_note = post_note.clone();
                    note_entry.connect_changed(move |entry| {
                        let body = entry.text().to_string();
                        post_note
                            .set_sensitive(!body.trim().is_empty() && body.trim().len() <= 10_000);
                    });
                }
                {
                    let state = session.clone();
                    let rows = list.clone();
                    let entry = note_entry.clone();
                    let room_id = note_room_id;
                    let asset_id = note_asset_id;
                    post_note.connect_clicked(move |button| {
                        submit_comment(
                            state.clone(),
                            rows.clone(),
                            entry.clone(),
                            button.clone(),
                            room_id.clone(),
                            asset_id.clone(),
                        );
                    });
                }
                composer.append(&note_entry);
                composer.append(&post_note);
                notes_panel.append(&composer);
                expander.set_child(Some(&notes_panel));
                row.append(&expander);

                let history_expander = gtk::Expander::new(Some("Caption history"));
                let history_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
                let load_history = gtk::Button::with_label("Load saved captions");
                load_history.set_halign(gtk::Align::Start);
                let history_rows = gtk::Box::new(gtk::Orientation::Vertical, 8);
                history_panel.append(&load_history);
                history_panel.append(&history_rows);
                history_expander.set_child(Some(&history_panel));
                row.append(&history_expander);
                {
                    let state = session.clone();
                    let rows = list.clone();
                    let history_rows = history_rows.clone();
                    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
                    let asset_id = item["id"].as_str().unwrap_or_default().to_owned();
                    let current_caption = item["caption"].as_str().unwrap_or_default().to_owned();
                    let may_restore = can_edit_assets(room);
                    load_history.connect_clicked(move |button| {
                        button.set_sensitive(false);
                        let trigger = button.clone();
                        let state = state.clone();
                        let rows = rows.clone();
                        let history_rows = history_rows.clone();
                        let room_id = room_id.clone();
                        let asset_id = asset_id.clone();
                        let current_caption = current_caption.clone();
                        let trigger = trigger.clone();
                        glib::spawn_future_local(async move {
                            let core = state.borrow().core.clone();
                            match command(
                                core,
                                json!({"action":"asset_history","room_id":room_id,"asset_id":asset_id}),
                            )
                            .await
                            {
                                Ok(response) => {
                                    while let Some(child) = history_rows.first_child() {
                                        history_rows.remove(&child);
                                    }
                                    for revision in response["history"].as_array().into_iter().flatten() {
                                        let caption = revision["caption"].as_str().unwrap_or_default().to_owned();
                                        let author = revision["author"].as_str().unwrap_or("Member");
                                        let attribution = if revision["author"] == state.borrow().snapshot["device_id"] {
                                            "You".to_owned()
                                        } else {
                                            format!("Member {}", author.chars().take(8).collect::<String>())
                                        };
                                        let item_panel = gtk::Box::new(gtk::Orientation::Vertical, 4);
                                        let text = gtk::Label::new(Some(if caption.is_empty() { "(No caption)" } else { &caption }));
                                        text.set_xalign(0.0);
                                        text.set_wrap(true);
                                        item_panel.append(&text);
                                        let metadata = gtk::Label::new(Some(&format!(
                                            "{attribution} · revision {}",
                                            revision["clock"].as_u64().unwrap_or_default()
                                        )));
                                        metadata.set_xalign(0.0);
                                        metadata.add_css_class("dim-label");
                                        item_panel.append(&metadata);
                                        if may_restore && caption != current_caption {
                                            let restore = gtk::Button::with_label("Use this caption");
                                            restore.set_halign(gtk::Align::Start);
                                            let state = state.clone();
                                            let rows = rows.clone();
                                            let room_id = room_id.clone();
                                            let asset_id = asset_id.clone();
                                            restore.connect_clicked(move |button| {
                                                submit_collection(
                                                    state.clone(),
                                                    rows.clone(),
                                                    button.clone(),
                                                    json!({"action":"patch_asset","room_id":room_id,"asset_id":asset_id,"fields":{"caption":caption}}),
                                                );
                                            });
                                            item_panel.append(&restore);
                                        }
                                        history_rows.append(&item_panel);
                                    }
                                    trigger.set_label("Reload saved captions");
                                }
                                Err(error) => trigger.set_label(&format!("History unavailable: {error}")),
                            }
                            trigger.set_sensitive(true);
                        });
                    });
                }
            }
            list.append(&row);
        }
    }
}

fn render_moment_suggestions(
    session: &Rc<RefCell<Session>>,
    list: &gtk::ListBox,
    room: &Value,
    core: Arc<FamilyCore>,
) {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    panel.set_margin_top(12);
    panel.set_margin_bottom(12);
    panel.set_margin_start(12);
    panel.set_margin_end(12);
    let heading = gtk::Label::new(Some("Possible Moments"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-2");
    panel.append(&heading);
    let note = gtk::Label::new(Some(
        "Capture-time groupings stay suggestions until you review and save them.",
    ));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("dim-label");
    panel.append(&note);
    let review = gtk::Button::with_label("Find possible Moments");
    review.set_halign(gtk::Align::Start);
    let room_id = room["id"].as_str().unwrap_or_default().to_owned();
    let parent = list
        .root()
        .and_then(|root| root.downcast::<gtk::Window>().ok());
    let state = session.clone();
    let rows = list.clone();
    let panel_for_error = panel.clone();
    review.connect_clicked(move |button| {
        button.set_sensitive(false);
        let trigger = button.clone();
        let core = core.clone();
        let state = state.clone();
        let rows = rows.clone();
        let parent = parent.clone();
        let room_id = room_id.clone();
        let panel_for_error = panel_for_error.clone();
        glib::spawn_future_local(async move {
            match command(
                core.clone(),
                json!({"action":"suggest_moments","room_id":room_id}),
            )
            .await
            {
                Ok(result) => {
                    let suggestions = result["suggestions"].clone();
                    show_moment_suggestion_reviews(parent, state, rows, core, room_id, suggestions);
                }
                Err(error) => {
                    let message = gtk::Label::new(Some(&format!(
                        "Could not load Moment suggestions: {error}"
                    )));
                    message.set_xalign(0.0);
                    message.set_wrap(true);
                    panel_for_error.append(&message);
                }
            }
            trigger.set_sensitive(true);
        });
    });
    panel.append(&review);
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&panel));
    list.append(&row);
}

fn selected_moment_asset_ids(suggested: &[String], selected: &HashSet<String>) -> Vec<String> {
    suggested
        .iter()
        .filter(|asset_id| selected.contains(*asset_id))
        .cloned()
        .collect()
}

fn show_moment_suggestion_reviews(
    parent: Option<gtk::Window>,
    session: Rc<RefCell<Session>>,
    list: gtk::ListBox,
    core: Arc<FamilyCore>,
    room_id: String,
    suggestions: Value,
) {
    let window = gtk::Window::builder()
        .title("Review possible Moments")
        .default_width(560)
        .default_height(680)
        .modal(true)
        .build();
    if let Some(parent) = parent.as_ref() {
        window.set_transient_for(Some(parent));
    }
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let heading = gtk::Label::new(Some("Review before adding to your Room"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-2");
    content.append(&heading);
    let explanation = gtk::Label::new(Some(
        "These groups come from nearby capture times. Adjust each title and selection; nothing is added until you save it.",
    ));
    explanation.set_wrap(true);
    explanation.set_xalign(0.0);
    explanation.add_css_class("dim-label");
    content.append(&explanation);
    let candidates = suggestions.as_array().cloned().unwrap_or_default();
    if candidates.is_empty() {
        let empty = gtk::Label::new(Some("No new capture-time groupings are ready to review."));
        empty.set_xalign(0.0);
        content.append(&empty);
    }
    for suggestion in candidates {
        let asset_ids = suggestion["asset_ids"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect::<Vec<_>>();
        let start = suggestion["start"].as_i64().unwrap_or_default();
        let date = glib::DateTime::from_unix_local(start)
            .ok()
            .and_then(|date| date.format("%B %-d, %Y").ok())
            .map(|date| date.to_string())
            .unwrap_or_else(|| "Moment".into());
        let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        card.add_css_class("card");
        card.set_margin_top(4);
        card.set_margin_bottom(4);
        let title = gtk::Entry::builder()
            .text(format!("Moment · {date}"))
            .placeholder_text("Moment title")
            .hexpand(true)
            .build();
        card.append(&title);
        let featured = gtk::CheckButton::with_label("Feature on Home");
        card.append(&featured);
        let checks = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let mut selected_checks = Vec::new();
        for asset in session.borrow().snapshot["rooms"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|room| room["id"] == room_id)
            .and_then(|room| room["assets"].as_array())
            .into_iter()
            .flatten()
            .filter(|asset| {
                asset_ids.iter().any(|id| asset["id"] == *id) && asset["deleted"] != true
            })
        {
            let label = asset["caption"]
                .as_str()
                .filter(|label| !label.is_empty())
                .or(asset["filename"].as_str())
                .unwrap_or("Memory");
            let check = gtk::CheckButton::with_label(label);
            check.set_active(true);
            checks.append(&check);
            selected_checks.push((asset["id"].as_str().unwrap_or_default().to_owned(), check));
        }
        let scroll = gtk::ScrolledWindow::builder()
            .child(&checks)
            .min_content_height(100)
            .max_content_height(260)
            .build();
        card.append(&scroll);
        let status = gtk::Label::new(None);
        status.set_xalign(0.0);
        status.set_wrap(true);
        card.append(&status);
        let save = gtk::Button::with_label("Save Moment");
        save.set_halign(gtk::Align::Start);
        let selected_checks = selected_checks.clone();
        let suggested_ids = asset_ids.clone();
        let title_for_save = title.clone();
        let featured_for_save = featured.clone();
        let status_for_save = status.clone();
        let state_for_save = session.clone();
        let rows_for_save = list.clone();
        let window_for_save = window.clone();
        let core_for_save = core.clone();
        let room_for_save = room_id.clone();
        save.connect_clicked(move |button| {
            let title = title_for_save.text().trim().to_owned();
            let checked_ids = selected_checks
                .iter()
                .filter(|(_, check)| check.is_active())
                .map(|(id, _)| id.clone())
                .collect::<HashSet<_>>();
            let selected = selected_moment_asset_ids(&suggested_ids, &checked_ids);
            if title.is_empty() || selected.is_empty() {
                status_for_save.set_text(if title.is_empty() {
                    "Add a title before saving this Moment."
                } else {
                    "Choose at least one memory before saving this Moment."
                });
                return;
            }
            button.set_sensitive(false);
            let request = json!({"action":"save_moment","room_id":room_for_save,"title":title,"featured":featured_for_save.is_active(),"asset_ids":selected});
            let state = state_for_save.clone();
            let rows = rows_for_save.clone();
            let window = window_for_save.clone();
            let core = core_for_save.clone();
            let status = status_for_save.clone();
            let button = button.clone();
            glib::spawn_future_local(async move {
                match command(core, request).await {
                    Ok(_) => {
                        window.close();
                        let rooms = {
                            let session = state.borrow();
                            session
                                .rooms_widget
                                .as_ref()
                                .and_then(glib::WeakRef::upgrade)
                        };
                        if let Some(rooms) = rooms {
                            if let Err(error) = reload(state, rooms, rows).await {
                                status.set_text(&format!("Moment saved, but Home could not refresh: {error}"));
                            }
                        } else {
                            status.set_text("Moment saved. Reopen Home to refresh the Room.");
                        }
                    }
                    Err(error) => {
                        status.set_text(&error);
                        button.set_sensitive(true);
                    }
                }
            });
        });
        card.append(&save);
        content.append(&card);
    }
    let close = gtk::Button::with_label("Done");
    close.set_halign(gtk::Align::End);
    let window_to_close = window.clone();
    close.connect_clicked(move |_| window_to_close.close());
    content.append(&close);
    let outer = gtk::ScrolledWindow::builder().child(&content).build();
    window.set_child(Some(&outer));
    window.present();
}

fn member_role_options(room: &Value, member: &Value) -> Vec<&'static str> {
    if room["role"] != "owner" || member["device_id"] == room["owner_id"] {
        return Vec::new();
    }
    if room["album_only"] == true {
        vec!["viewer"]
    } else {
        vec!["viewer", "contributor", "admin"]
    }
}

fn render_pending_invitations(
    session: &Rc<RefCell<Session>>,
    list: &gtk::ListBox,
    pending: &Value,
) {
    let Some(invitations) = pending
        .as_array()
        .filter(|invitations| !invitations.is_empty())
    else {
        let empty = gtk::Label::new(Some("No invitations are waiting for review."));
        empty.set_xalign(0.0);
        list.append(&empty);
        return;
    };
    let heading = gtk::Label::new(Some("Invitations to review"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-2");
    list.append(&heading);
    let note = gtk::Label::new(Some(
        "Declining removes the invitation from this device only; the sender is not notified.",
    ));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("dim-label");
    list.append(&note);
    for invitation in invitations {
        let id = invitation["id"].as_str().unwrap_or_default().to_owned();
        let room_id = invitation["room_id"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let owner_id = invitation["owner_id"].as_str().unwrap_or_default();
        let role = invitation["role"].as_str().unwrap_or("viewer");
        let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        card.set_margin_top(8);
        card.set_margin_bottom(8);
        card.set_margin_start(12);
        card.set_margin_end(12);
        let title = gtk::Label::new(Some(
            invitation["name"].as_str().unwrap_or("Room invitation"),
        ));
        title.set_xalign(0.0);
        title.add_css_class("heading");
        card.append(&title);
        let summary = format!(
            "Device {} · {role} access{}",
            &owner_id[..owner_id.len().min(8)],
            if invitation["album_only"] == true {
                " · shared album only"
            } else {
                ""
            }
        );
        let summary = gtk::Label::new(Some(&summary));
        summary.set_xalign(0.0);
        summary.set_wrap(true);
        summary.add_css_class("dim-label");
        card.append(&summary);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let accept = gtk::Button::with_label("Accept");
        let decline = gtk::Button::with_label("Decline");
        actions.append(&accept);
        actions.append(&decline);
        card.append(&actions);
        list.append(&card);
        for (button, action) in [
            (accept, "accept_invitation"),
            (decline, "decline_invitation"),
        ] {
            let s = session.clone();
            let l = list.clone();
            let id = id.clone();
            let room_id = room_id.clone();
            let weak_rooms = session.borrow().rooms_widget.clone();
            button.connect_clicked(move |button| {
                button.set_sensitive(false);
                let s = s.clone();
                let l = l.clone();
                let id = id.clone();
                let room_id = room_id.clone();
                let weak_rooms = weak_rooms.clone();
                let action = action.to_owned();
                glib::spawn_future_local(async move {
                    let core = s.borrow().core.clone();
                    match command(core, json!({"action":action,"id":id})).await {
                        Ok(_) => {
                            if action == "accept_invitation" {
                                s.borrow_mut().room_id = room_id;
                            }
                            if let Some(rooms) = weak_rooms.and_then(|weak| weak.upgrade()) {
                                if let Err(error) = reload(s.clone(), rooms, l.clone()).await {
                                    let message = gtk::Label::new(Some(&error));
                                    message.set_wrap(true);
                                    l.append(&message);
                                }
                            } else {
                                render(&s, &l);
                            }
                        }
                        Err(error) => {
                            let message = gtk::Label::new(Some(&error));
                            message.set_wrap(true);
                            l.append(&message);
                        }
                    }
                });
            });
        }
    }
}

fn render_room_members(session: &Rc<RefCell<Session>>, list: &gtk::ListBox, room: &Value) {
    let heading = gtk::Label::new(Some(&format!(
        "Members of {}",
        room["name"].as_str().unwrap_or("Room")
    )));
    heading.set_xalign(0.0);
    heading.add_css_class("title-2");
    list.append(&heading);

    let note = gtk::Label::new(Some(
        "People named in photos do not receive access. Removing a device rotates Room keys for future content; downloaded copies cannot be recalled.",
    ));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("dim-label");
    list.append(&note);

    let owner_controls = room["role"] == "owner";
    let members = room["members"].as_array().cloned().unwrap_or_default();
    if members.is_empty() {
        let empty = gtk::Label::new(Some("No member devices are listed yet."));
        empty.set_xalign(0.0);
        list.append(&empty);
        return;
    }

    for member in members {
        let device_id = member["device_id"].as_str().unwrap_or_default().to_owned();
        let role = member["role"].as_str().unwrap_or("viewer").to_owned();
        let is_owner = device_id == room["owner_id"].as_str().unwrap_or_default();
        let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        card.set_margin_top(10);
        card.set_margin_bottom(10);
        card.set_margin_start(12);
        card.set_margin_end(12);

        let title = gtk::Label::new(Some(if is_owner {
            "Room owner"
        } else {
            "Member device"
        }));
        title.set_xalign(0.0);
        title.add_css_class("heading");
        card.append(&title);

        let identity =
            gtk::Label::new(Some(&format!("{device_id}\nRole: {}", role.to_uppercase())));
        identity.set_xalign(0.0);
        identity.set_selectable(true);
        identity.set_wrap(true);
        identity.add_css_class("dim-label");
        card.append(&identity);

        let options = member_role_options(room, &member);
        if !options.is_empty() {
            let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            for new_role in options {
                let button = gtk::Button::with_label(new_role);
                button.set_sensitive(role != new_role);
                let state = session.clone();
                let rows = list.clone();
                let target_room = room["id"].as_str().unwrap_or_default().to_owned();
                let target_device = device_id.clone();
                let role_name = new_role.to_owned();
                button.connect_clicked(move |button| {
                    submit_member_change(
                        state.clone(),
                        rows.clone(),
                        button.clone(),
                        json!({"action":"set_role","room_id":target_room,
                            "device_id":target_device,"role":role_name}),
                    );
                });
                actions.append(&button);
            }
            card.append(&actions);

            let remove = gtk::Button::with_label("Remove device…");
            let confirmation = gtk::Revealer::new();
            confirmation.set_reveal_child(false);
            let confirm_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let warning = gtk::Label::new(Some("Remove this device from the Room?"));
            warning.set_wrap(true);
            warning.set_hexpand(true);
            confirm_row.append(&warning);
            let confirm = gtk::Button::with_label("Remove access");
            let state = session.clone();
            let rows = list.clone();
            let target_room = room["id"].as_str().unwrap_or_default().to_owned();
            let target_device = device_id.clone();
            confirm.connect_clicked(move |button| {
                submit_member_change(
                    state.clone(),
                    rows.clone(),
                    button.clone(),
                    json!({"action":"revoke_device","room_id":target_room,
                        "device_id":target_device}),
                );
            });
            confirm_row.append(&confirm);
            let cancel = gtk::Button::with_label("Cancel");
            let confirm_revealer = confirmation.clone();
            cancel.connect_clicked(move |_| confirm_revealer.set_reveal_child(false));
            confirm_row.append(&cancel);
            confirmation.set_child(Some(&confirm_row));
            let reveal = confirmation.clone();
            remove.connect_clicked(move |_| reveal.set_reveal_child(true));
            card.append(&remove);
            card.append(&confirmation);

            let prepare_transfer =
                gtk::Button::with_label("Prepare ownership offer for this device");
            let offer_output = gtk::Entry::new();
            offer_output.set_editable(false);
            offer_output.set_placeholder_text(Some("Signed offer will appear here"));
            offer_output.set_visible(false);
            let transfer_note = gtk::Label::new(None);
            transfer_note.set_xalign(0.0);
            transfer_note.set_wrap(true);
            transfer_note.add_css_class("dim-label");
            let state = session.clone();
            let target_room = room["id"].as_str().unwrap_or_default().to_owned();
            let target_device = device_id.clone();
            let output = offer_output.clone();
            let note = transfer_note.clone();
            prepare_transfer.connect_clicked(move |button| {
                button.set_sensitive(false);
                let core = state.borrow().core.clone();
                let request = json!({"action":"prepare_owner_transfer",
                    "room_id":target_room,"device_id":target_device});
                let button = button.clone();
                let output = output.clone();
                let note = note.clone();
                glib::spawn_future_local(async move {
                    match command(core, request).await {
                        Ok(result) => {
                            let offer = result["offer"].as_str().unwrap_or_default();
                            if offer.is_empty() {
                                note.set_text("The ownership offer could not be created.");
                            } else {
                                output.set_text(offer);
                                output.set_visible(true);
                                note.set_text("Send this signed offer privately to this Member. It is bound to their device.");
                                button.set_label("Offer ready");
                            }
                        }
                        Err(error) => note.set_text(&error),
                    }
                    button.set_sensitive(true);
                });
            });
            card.append(&prepare_transfer);
            card.append(&offer_output);
            card.append(&transfer_note);
        } else if is_owner {
            let detail =
                gtk::Label::new(Some("Transfer ownership before removing the owner device."));
            detail.set_xalign(0.0);
            detail.set_wrap(true);
            detail.add_css_class("dim-label");
            card.append(&detail);
        } else if !owner_controls {
            let detail = gtk::Label::new(Some("Only the Room owner can change device access."));
            detail.set_xalign(0.0);
            detail.set_wrap(true);
            detail.add_css_class("dim-label");
            card.append(&detail);
        }

        card.add_css_class("card");
        list.append(&card);
    }

    if owner_controls {
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
        panel.set_margin_top(12);
        panel.set_margin_bottom(12);
        panel.set_margin_start(12);
        panel.set_margin_end(12);
        let heading = gtk::Label::new(Some("Invite a Member"));
        heading.set_xalign(0.0);
        heading.add_css_class("heading");
        panel.append(&heading);
        let device = gtk::Entry::new();
        device.set_placeholder_text(Some("Member's device code"));
        panel.append(&device);
        let invitation = gtk::Entry::new();
        invitation.set_editable(false);
        invitation.set_placeholder_text(Some("The device-bound invitation will appear here"));
        invitation.set_visible(false);
        panel.append(&invitation);
        let status = gtk::Label::new(None);
        status.set_xalign(0.0);
        status.set_wrap(true);
        status.add_css_class("dim-label");
        panel.append(&status);
        let roles: &[&str] = if room["album_only"] == true {
            &["viewer"]
        } else {
            &["contributor", "viewer", "admin"]
        };
        let role_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        for role in roles {
            let button = gtk::Button::with_label(&format!("Invite as {role}"));
            let state = session.clone();
            let target_room = room["id"].as_str().unwrap_or_default().to_owned();
            let member_device = device.clone();
            let invite_output = invitation.clone();
            let feedback = status.clone();
            let role = (*role).to_owned();
            button.connect_clicked(move |button| {
                let device_id = member_device.text().trim().to_owned();
                if device_id.is_empty() {
                    feedback.set_text("Enter the intended Member's device code first.");
                    return;
                }
                button.set_sensitive(false);
                let core = state.borrow().core.clone();
                let request = json!({"action":"invite","room_id":target_room,
                    "device_id":device_id,"role":role});
                let state = state.clone();
                let target_room = target_room.clone();
                let button = button.clone();
                let invite_output = invite_output.clone();
                let feedback = feedback.clone();
                glib::spawn_future_local(async move {
                    let result = async {
                        let response = command(core.clone(), request).await?;
                        let token = response["invitation"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned();
                        if token.is_empty() {
                            return Err("The core returned no invitation.".to_owned());
                        }
                        let sources = state.borrow().snapshot["sources"]
                            .as_array()
                            .cloned()
                            .unwrap_or_default();
                        let mut published = false;
                        for source in sources.iter().filter(|source| {
                            source["room_id"] == target_room && source["paused"] != true
                        }) {
                            if command(
                                core.clone(),
                                json!({"action":"refresh_access",
                                "id":source["id"]}),
                            )
                            .await
                            .is_ok()
                            {
                                published = true;
                            }
                        }
                        Ok::<_, String>((token, published))
                    }
                    .await;
                    match result {
                        Ok((token, published)) => {
                            invite_output.set_text(&token);
                            invite_output.set_visible(true);
                            feedback.set_text(if published {
                                "Invitation ready. Connected storage is prepared for this Member."
                            } else {
                                "Invitation ready. Sync the Room before the new Member connects."
                            });
                            let core = state.borrow().core.clone();
                            if let Ok(snapshot) = command(core, json!({"action":"snapshot"})).await
                            {
                                state.borrow_mut().snapshot = snapshot;
                            }
                        }
                        Err(error) => feedback.set_text(&error),
                    }
                    button.set_sensitive(true);
                });
            });
            role_actions.append(&button);
        }
        panel.append(&role_actions);
        panel.add_css_class("card");
        list.append(&panel);
    }

    if !owner_controls && room["role"] != "removed" {
        let panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
        panel.set_margin_top(12);
        panel.set_margin_bottom(12);
        panel.set_margin_start(12);
        panel.set_margin_end(12);
        let heading = gtk::Label::new(Some("Accept ownership offer"));
        heading.set_xalign(0.0);
        heading.add_css_class("heading");
        panel.append(&heading);
        let explanation = gtk::Label::new(Some(
            "Paste an offer shared with this device. Acceptance makes this device the Room owner and rotates the key for future contributions.",
        ));
        explanation.set_xalign(0.0);
        explanation.set_wrap(true);
        explanation.add_css_class("dim-label");
        panel.append(&explanation);
        let offer = gtk::Entry::new();
        offer.set_placeholder_text(Some("Signed ownership offer"));
        panel.append(&offer);
        let review = gtk::Button::with_label("Review ownership offer");
        let confirmation = gtk::Revealer::new();
        confirmation.set_reveal_child(false);
        let confirm_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let warning = gtk::Label::new(Some(
            "After acceptance, the previous owner becomes an admin (or viewer for an album-only Room). Previously downloaded copies remain with Members.",
        ));
        warning.set_wrap(true);
        warning.set_hexpand(true);
        confirm_row.append(&warning);
        let accept = gtk::Button::with_label("Accept ownership");
        let state = session.clone();
        let rows = list.clone();
        let target_room = room["id"].as_str().unwrap_or_default().to_owned();
        let offer_input = offer.clone();
        let status = gtk::Label::new(None);
        status.set_xalign(0.0);
        status.set_wrap(true);
        let result_status = status.clone();
        accept.connect_clicked(move |button| {
            button.set_sensitive(false);
            let core = state.borrow().core.clone();
            let encoded_offer = offer_input.text().trim().to_owned();
            let request = json!({"action":"accept_owner_transfer",
                "room_id":target_room.clone(),"offer":encoded_offer});
            let target_room = target_room.clone();
            let offer_input = offer_input.clone();
            let state = state.clone();
            let rows = rows.clone();
            let button = button.clone();
            let status = result_status.clone();
            glib::spawn_future_local(async move {
                let result = async {
                    command(core.clone(), request).await?;
                    let before_sync = command(core.clone(), json!({"action":"snapshot"})).await?;
                    let source_count = before_sync["sources"].as_array().map_or(0, |sources| {
                        sources.iter().filter(|source| source["room_id"] == target_room
                            && source["paused"] != true).count()
                    });
                    let sync = command(core.clone(), json!({"action":"sync_all",
                        "room_id":target_room})).await?;
                    let errors = sync["errors"].as_array().is_some_and(|items| !items.is_empty());
                    let snapshot = command(core, json!({"action":"snapshot"})).await?;
                    Ok::<_, String>((snapshot, errors || source_count == 0))
                }.await;
                match result {
                    Ok((snapshot, has_errors)) => {
                        state.borrow_mut().snapshot = snapshot;
                        offer_input.set_text("");
                        status.set_text(if has_errors {
                            "Ownership transferred on this device. Storage will receive the signed update when it reconnects."
                        } else {
                            "Ownership transferred and the signed update was sent to connected storage."
                        });
                        if let Some(updated_room) = state.borrow().snapshot["rooms"]
                            .as_array()
                            .and_then(|rooms| rooms.iter().find(|item| item["id"] == target_room))
                            .cloned()
                        {
                            while let Some(child) = rows.first_child() { rows.remove(&child); }
                            render_room_members(&state, &rows, &updated_room);
                        }
                    }
                    Err(error) => status.set_text(&error),
                }
                button.set_sensitive(true);
            });
        });
        confirm_row.append(&accept);
        let cancel = gtk::Button::with_label("Cancel");
        let reveal = confirmation.clone();
        cancel.connect_clicked(move |_| reveal.set_reveal_child(false));
        confirm_row.append(&cancel);
        confirmation.set_child(Some(&confirm_row));
        let reveal = confirmation.clone();
        let offer_to_check = offer.clone();
        let status_for_review = status.clone();
        review.connect_clicked(move |_| {
            if offer_to_check.text().trim().is_empty() {
                status_for_review.set_text("Paste the signed offer before reviewing it.");
            } else {
                status_for_review.set_text("");
                reveal.set_reveal_child(true);
            }
        });
        panel.append(&review);
        panel.append(&confirmation);
        panel.append(&status);
        panel.add_css_class("card");
        list.append(&panel);
    }
}

fn render_leave_room_control(session: &Rc<RefCell<Session>>, list: &gtk::ListBox, room: &Value) {
    let role = room["role"].as_str().unwrap_or("viewer");
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.set_margin_top(12);
    card.set_margin_bottom(12);
    card.set_margin_start(12);
    card.set_margin_end(12);
    let heading = gtk::Label::new(Some("Leave Room"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-3");
    card.append(&heading);
    if role == "owner" {
        let note = gtk::Label::new(Some(
            "Transfer ownership to another Member before leaving. This keeps an owner in the Room.",
        ));
        note.set_xalign(0.0);
        note.set_wrap(true);
        note.add_css_class("dim-label");
        card.append(&note);
    } else {
        let note = gtk::Label::new(Some(
            "This removes this device’s Room keys and storage connections. Ask an owner to remove this device from the shared roster to revoke its grant for the Room.",
        ));
        note.set_xalign(0.0);
        note.set_wrap(true);
        note.add_css_class("dim-label");
        card.append(&note);
        let reveal = gtk::Revealer::new();
        reveal.set_reveal_child(false);
        let action = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let leave = gtk::Button::with_label(if role == "removed" {
            "Forget this Room"
        } else {
            "Leave Room…"
        });
        let confirmation = gtk::Label::new(Some(
            "This device will stop syncing and forget its Room keys. Ask an owner to remove this device from the shared roster to revoke its grant. A saved device backup may restore this Room.",
        ));
        confirmation.set_wrap(true);
        confirmation.set_hexpand(true);
        let confirm = gtk::Button::with_label(if role == "removed" {
            "Forget Room"
        } else {
            "Leave Room"
        });
        confirm.add_css_class("destructive-action");
        let state = session.clone();
        let rows = list.clone();
        let target_room = room["id"].as_str().unwrap_or_default().to_owned();
        let target_reveal = reveal.clone();
        confirm.connect_clicked(move |button| {
            target_reveal.set_reveal_child(false);
            submit_leave_room(
                state.clone(),
                rows.clone(),
                button.clone(),
                target_room.clone(),
            );
        });
        action.append(&confirmation);
        action.append(&confirm);
        let cancel = gtk::Button::with_label("Cancel");
        let target_reveal = reveal.clone();
        cancel.connect_clicked(move |_| target_reveal.set_reveal_child(false));
        action.append(&cancel);
        reveal.set_child(Some(&action));
        let target_reveal = reveal.clone();
        leave.connect_clicked(move |_| target_reveal.set_reveal_child(true));
        card.append(&leave);
        card.append(&reveal);
    }
    card.add_css_class("card");
    list.append(&card);
}

fn submit_leave_room(
    session: Rc<RefCell<Session>>,
    list: gtk::ListBox,
    button: gtk::Button,
    room_id: String,
) {
    button.set_sensitive(false);
    glib::spawn_future_local(async move {
        let core = session.borrow().core.clone();
        let rooms = session
            .borrow()
            .rooms_widget
            .as_ref()
            .and_then(|rooms| rooms.upgrade());
        let result = async {
            command(core, json!({"action":"leave_room","room_id":room_id})).await?;
            let rooms = rooms.ok_or_else(|| "Room list is unavailable".to_owned())?;
            reload(session.clone(), rooms, list.clone()).await?;
            Ok::<(), String>(())
        }
        .await;
        if let Err(error) = result {
            button.set_label(&error);
            button.set_sensitive(true);
        }
    });
}

fn submit_member_change(
    session: Rc<RefCell<Session>>,
    list: gtk::ListBox,
    button: gtk::Button,
    request: Value,
) {
    button.set_sensitive(false);
    glib::spawn_future_local(async move {
        let core = session.borrow().core.clone();
        let room_id = request["room_id"].as_str().unwrap_or_default().to_owned();
        let result = async {
            command(core.clone(), request).await?;
            command(core.clone(), json!({"action":"sync_all","room_id":room_id})).await?;
            let snapshot = command(core, json!({"action":"snapshot"})).await?;
            session.borrow_mut().snapshot = snapshot;
            render(&session, &list);
            Ok::<(), String>(())
        }
        .await;
        if let Err(error) = result {
            button.set_label(&error);
            button.set_sensitive(true);
        }
    });
}
async fn reload(
    session: Rc<RefCell<Session>>,
    rooms: gtk::DropDown,
    list: gtk::ListBox,
) -> Result<(), String> {
    let core = session.borrow().core.clone();
    let value = command(core, json!({"action":"snapshot"})).await?;
    let names = value["rooms"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| r["name"].as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let selected = {
        let mut s = session.borrow_mut();
        s.snapshot = value;
        s.library_filters_dirty = true;
        let restore_enabled = can_restore_backup(&s.snapshot)
            && s.restore_recovery_entry
                .as_ref()
                .and_then(glib::WeakRef::upgrade)
                .is_some_and(|entry| !entry.text().trim().is_empty());
        if let Some(button) = s
            .restore_backup_button
            .as_ref()
            .and_then(glib::WeakRef::upgrade)
        {
            button.set_sensitive(restore_enabled);
        }
        let index = s.snapshot["rooms"]
            .as_array()
            .and_then(|a| a.iter().position(|r| r["id"] == s.room_id))
            .unwrap_or(0);
        s.room_id = s.snapshot["rooms"][index]["id"]
            .as_str()
            .unwrap_or("")
            .into();
        index as u32
    };
    rooms.set_model(Some(&gtk::StringList::new(
        &names.iter().map(String::as_str).collect::<Vec<_>>(),
    )));
    rooms.set_selected(selected);
    render(&session, &list);
    Ok(())
}

fn schedule_folder_watch_scans(
    session: Rc<RefCell<Session>>,
    rooms: gtk::DropDown,
    list: gtk::ListBox,
    status: gtk::Label,
) {
    let scanning = session.borrow().watch_scanning.clone();
    glib::timeout_add_local(Duration::from_secs(15), move || {
        let has_active_watch =
            session.borrow().snapshot["watches"]
                .as_array()
                .is_some_and(|watches| {
                    watches.iter().any(|watch| {
                        watch["paused"] != true
                            && watch["path"]
                                .as_str()
                                .is_some_and(|path| !path.starts_with("photos://"))
                    })
                });
        if !has_active_watch || scanning.replace(true) {
            return glib::ControlFlow::Continue;
        }

        let session = session.clone();
        let rooms = rooms.clone();
        let list = list.clone();
        let status = status.clone();
        let scanning = scanning.clone();
        glib::spawn_future_local(async move {
            let core = session.borrow().core.clone();
            let result = command(core.clone(), json!({"action":"scan_watches"})).await;
            match result {
                Ok(scan) => {
                    let imported = scan["imported"].as_u64().unwrap_or_default();
                    let mut sync_errors = Vec::new();
                    if imported > 0 {
                        let target_rooms: HashSet<String> = session.borrow().snapshot["watches"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|watch| {
                                watch["paused"] != true
                                    && watch["path"]
                                        .as_str()
                                        .is_some_and(|path| !path.starts_with("photos://"))
                            })
                            .filter_map(|watch| watch["room_id"].as_str().map(str::to_owned))
                            .collect();
                        for room_id in target_rooms {
                            match command(
                                core.clone(),
                                json!({"action":"sync_all","room_id":room_id}),
                            )
                            .await
                            {
                                Ok(response) => sync_errors.extend(
                                    response["errors"]
                                        .as_array()
                                        .into_iter()
                                        .flatten()
                                        .filter_map(Value::as_str)
                                        .map(str::to_owned),
                                ),
                                Err(error) => sync_errors.push(error),
                            }
                        }
                        if let Err(error) = reload(session.clone(), rooms, list).await {
                            sync_errors.push(error);
                        }
                    }
                    let errors = scan["errors"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .chain(sync_errors)
                        .collect::<Vec<_>>();
                    if imported > 0 {
                        status.set_text(&format!(
                            "Automatic uploads found {imported} new item(s){}.",
                            if errors.is_empty() {
                                "; sync queued".to_owned()
                            } else {
                                format!("; some items need attention: {}", errors.join(" · "))
                            }
                        ));
                    } else if !errors.is_empty() {
                        status.set_text(&format!(
                            "Automatic folder upload needs attention: {}",
                            errors.join(" · ")
                        ));
                    }
                }
                Err(error) => status.set_text(&format!("Automatic folder scan failed: {error}")),
            }
            scanning.set(false);
        });
        glib::ControlFlow::Continue
    });
}

fn should_index_video(is_local: bool, evidence_missing: bool, include_remote: bool) -> bool {
    evidence_missing && (is_local || include_remote)
}

fn start_video_search(
    session: Rc<RefCell<Session>>,
    list: gtk::ListBox,
    status: gtk::Label,
    coverage: gtk::Label,
    parent: gtk::Window,
    button: gtk::Button,
    query: String,
    modality: String,
    include_remote: bool,
) {
    if query.trim().is_empty() {
        return;
    }
    button.set_sensitive(false);
    let prompt = gtk::Label::new(Some(&query));
    prompt.set_xalign(1.0);
    prompt.set_wrap(true);
    prompt.add_css_class("accent");
    prompt.set_margin_top(12);
    list.append(&prompt);
    glib::spawn_future_local(async move {
        let (core, room_id, snapshot, context_query) = {
            let state = session.borrow();
            let context = state
                .last_video_prompt
                .as_ref()
                .filter(|(context_room, _)| context_room == &state.room_id)
                .map(|(_, prompt)| prompt.clone());
            (
                state.core.clone(),
                state.room_id.clone(),
                state.snapshot.clone(),
                context,
            )
        };
        let Some(room) = snapshot["rooms"]
            .as_array()
            .and_then(|rooms| rooms.iter().find(|room| room["id"] == room_id))
            .cloned()
        else {
            status.set_label("Choose a Room before searching videos.");
            button.set_sensitive(true);
            return;
        };
        status.set_label("Checking videos stored on this device…");
        let video_status = match command(
            core.clone(),
            json!({"action":"video_index_status","room_id":room_id}),
        )
        .await
        {
            Ok(value) => value,
            Err(error) => {
                status.set_label(&error);
                button.set_sensitive(true);
                return;
            }
        };
        let vision_url = std::env::var("FAMILY_ROOM_VISION_URL").unwrap_or_default();
        let vision_requested = ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"]
            .iter()
            .any(|prefix| vision_url.starts_with(prefix));
        let pending = video_status["videos"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|video| {
                let frames_missing = video["frames_indexed"] != true;
                let audio_missing = video["audio_indexed"] != true;
                let requested_evidence_missing = match modality.as_str() {
                    "frame" => frames_missing,
                    "audio" => audio_missing,
                    _ => frames_missing || audio_missing,
                } || (modality != "audio"
                    && vision_requested
                    && video["visual_labels_indexed"] != true);
                should_index_video(
                    video["local"] == true,
                    requested_evidence_missing,
                    include_remote,
                )
            })
            .filter_map(|video| {
                let id = video["asset_id"].as_str()?;
                room["assets"]
                    .as_array()?
                    .iter()
                    .find(|asset| asset["id"] == id && asset["deleted"] != true)
                    .cloned()
                    .map(|asset| (asset, video["local"] == true))
            })
            .collect::<Vec<_>>();
        let mut indexing_note = String::new();
        for (position, (asset, already_local)) in pending.iter().enumerate() {
            status.set_label(&format!(
                "{} video {} of {}…",
                if *already_local {
                    "Indexing local"
                } else {
                    "Downloading and indexing remote"
                },
                position + 1,
                pending.len()
            ));
            let Some(asset_id) = asset["id"].as_str() else {
                continue;
            };
            let root = match preview_root() {
                Ok(root) => root,
                Err(error) => {
                    indexing_note = error;
                    break;
                }
            };
            let filename = std::path::Path::new(asset["filename"].as_str().unwrap_or("video.mp4"))
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("video.mp4"));
            let path = root.join(format!(
                "{}-{}",
                uuid::Uuid::new_v4(),
                filename.to_string_lossy()
            ));
            if !already_local {
                if let Err(error) = command(
                    core.clone(),
                    json!({"action":"fetch","room_id":room_id,"asset_id":asset_id}),
                )
                .await
                {
                    indexing_note = error;
                    break;
                }
            }
            if let Err(error) = command(core.clone(), json!({"action":"materialize","room_id":room_id,"asset_id":asset_id,"destination":path})).await {
                indexing_note = error; break;
            }
            let (sender, receiver) = async_channel::bounded(1);
            let worker_path = path.clone();
            let include_audio = modality != "frame";
            let include_frames = modality != "audio";
            std::thread::spawn(move || {
                let _ = sender.send_blocking(family_video_indexer::analyze_video_scoped(
                    &worker_path,
                    include_audio,
                    include_frames,
                ));
            });
            let analysis = receiver.recv().await;
            let _ = std::fs::remove_file(&path);
            match analysis {
                Ok(Ok(analysis)) => {
                    let request = json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
                        "observations":analysis.observations,"audio_indexed":analysis.audio_indexed,
                    "frames_indexed":analysis.frames_indexed,"frame_interval_ms":analysis.frame_interval_ms,
                    "visual_labels_indexed":analysis.visual_labels_indexed,
                        "preserve_existing":true});
                    if let Err(error) = command(core.clone(), request).await {
                        indexing_note = error;
                        break;
                    }
                    if include_frames && !analysis.visual_message.is_empty() {
                        if !indexing_note.is_empty() {
                            indexing_note.push(' ');
                        }
                        indexing_note.push_str(&analysis.visual_message);
                    }
                    if include_audio && !analysis.audio_indexed {
                        indexing_note = analysis.audio_message;
                    }
                    if include_frames && !analysis.frames_indexed {
                        indexing_note =
                            "Frame text could not be indexed; install FFmpeg and Tesseract.".into();
                    }
                }
                Ok(Err(error)) => {
                    indexing_note = error;
                    break;
                }
                Err(error) => {
                    indexing_note = error.to_string();
                    break;
                }
            }
        }
        status.set_label("Searching this Room’s private video evidence…");
        let result = match command(
            core.clone(),
            json!({"action":"search_video","room_id":room_id,"query":query,"modality":modality,"context_query":context_query}),
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                status.set_label(&error);
                button.set_sensitive(true);
                return;
            }
        };
        session.borrow_mut().last_video_prompt = Some((room_id.clone(), query.clone()));
        let counts = &result["coverage"];
        coverage.set_label(&format!(
            "{} of {} videos indexed here · {} with speech · {} with frames",
            counts["indexed"], counts["videos"], counts["audio_indexed"], counts["frames_indexed"]
        ));
        let answer = gtk::Label::new(result["answer"].as_str());
        answer.set_xalign(0.0);
        answer.set_wrap(true);
        answer.add_css_class("title-3");
        list.append(&answer);
        for matched in result["matches"].as_array().into_iter().flatten() {
            let Some(asset_id) = matched["asset_id"].as_str() else {
                continue;
            };
            let Some(asset) = room["assets"]
                .as_array()
                .and_then(|items| items.iter().find(|item| item["id"] == asset_id))
                .cloned()
            else {
                continue;
            };
            let start_ms = matched["start_ms"].as_u64().unwrap_or(0);
            let end_ms = matched["end_ms"].as_u64().unwrap_or(start_ms);
            let is_exact = matched["match_kind"] == "exact";
            let row = gtk::Box::new(gtk::Orientation::Vertical, 7);
            row.set_margin_top(10);
            row.set_margin_bottom(10);
            row.set_margin_start(12);
            row.set_margin_end(12);
            let title = asset["caption"]
                .as_str()
                .filter(|text| !text.is_empty())
                .or(asset["filename"].as_str())
                .unwrap_or("Video moment");
            let label = gtk::Label::new(Some(title));
            label.set_xalign(0.0);
            label.add_css_class("heading");
            row.append(&label);
            let has_context_clue = matched["context_evidence"]
                .as_array()
                .is_some_and(|items| !items.is_empty());
            let match_kind_text = match (matched["match_kind"].as_str(), has_context_clue) {
                (Some("exact"), true) => "EXACT PHRASE · NEARBY CLUE",
                (Some("exact"), _) => "EXACT PHRASE",
                (Some("evidence"), _) => "INDEXED EVIDENCE",
                _ => "SUGGESTED MATCH",
            };
            let match_kind = gtk::Label::new(Some(match_kind_text));
            match_kind.set_xalign(0.0);
            match_kind.add_css_class(if is_exact { "accent" } else { "dim-label" });
            row.append(&match_kind);
            let time_range = if end_ms > start_ms {
                format!("{}–{}", timestamp_label(start_ms), timestamp_label(end_ms))
            } else {
                format!("At {}", timestamp_label(start_ms))
            };
            let open = gtk::Button::with_label(&format!("Open {time_range}"));
            let playback_core = core.clone();
            let playback_parent = parent.clone();
            let playback_asset = asset.clone();
            open.connect_clicked(move |_| {
                open_video_match(
                    &playback_parent,
                    playback_core.clone(),
                    playback_asset.clone(),
                    start_ms,
                )
            });
            open.set_halign(gtk::Align::Start);
            row.append(&open);
            if let Some(clues) = matched["context_evidence"].as_array() {
                if !clues.is_empty() {
                    let heading = gtk::Label::new(Some("NEARBY CLUE"));
                    heading.set_xalign(0.0);
                    heading.add_css_class("accent");
                    row.append(&heading);
                    for clue in clues.iter().take(2) {
                        let kind = match clue["modality"].as_str() {
                            Some("speech") => "Spoken",
                            Some("screen_text") => "On screen",
                            Some("sound_event") => "Possible sound",
                            _ => "Visual suggestion",
                        };
                        let text = clue["text"].as_str().unwrap_or("");
                        let stamp = timestamp_label(clue["start_ms"].as_u64().unwrap_or(start_ms));
                        let line = gtk::Label::new(Some(&format!("{kind} · {stamp}\n{text}")));
                        line.set_xalign(0.0);
                        line.set_wrap(true);
                        line.add_css_class("dim-label");
                        row.append(&line);
                    }
                }
            }
            for evidence in matched["evidence"].as_array().into_iter().flatten().take(3) {
                let kind = match evidence["modality"].as_str() {
                    Some("speech") => "Spoken",
                    Some("screen_text") => "On screen",
                    _ => "Visual suggestion",
                };
                let text = evidence["text"].as_str().unwrap_or("");
                let stamp = timestamp_label(evidence["start_ms"].as_u64().unwrap_or(start_ms));
                let line = gtk::Label::new(Some(&format!("{kind} · {stamp}\n{text}")));
                line.set_xalign(0.0);
                line.set_wrap(true);
                line.add_css_class("dim-label");
                row.append(&line);
            }
            row.add_css_class("card");
            list.append(&row);
        }
        let final_status = if indexing_note.is_empty() {
            if include_remote {
                "Private search · selected remote originals are now stored on this device"
                    .to_string()
            } else {
                "Private search · only locally stored videos were indexed".to_string()
            }
        } else {
            format!("{indexing_note} Existing device-local evidence was searched.")
        };
        status.set_label(&final_status);
        button.set_sensitive(true);
    });
}

fn timestamp_label(milliseconds: u64) -> String {
    format!(
        "{:02}:{:02}",
        milliseconds / 60_000,
        milliseconds / 1_000 % 60
    )
}

fn open_video_match(parent: &gtk::Window, core: Arc<FamilyCore>, asset: Value, start_at_ms: u64) {
    let title = format!("Video moment · {}", timestamp_label(start_at_ms));
    open_screening(parent, core, &title, vec![asset], start_at_ms);
}

fn activate(app: &gtk::Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Family Room")
        .default_width(1050)
        .default_height(750)
        .build();
    let (core, recovery_key) = match open() {
        Ok(opened) => opened,
        Err(error) => {
            window.set_child(Some(&gtk::Label::new(Some(&error.to_string()))));
            window.present();
            return;
        }
    };
    let session = Rc::new(RefCell::new(Session {
        core,
        recovery_key,
        snapshot: json!({}),
        room_id: String::new(),
        last_video_prompt: None,
        storage_feedback: None,
        section: "Home".into(),
        library_search: String::new(),
        library_year: String::new(),
        library_moment: String::new(),
        moment_river_year: String::new(),
        library_people: HashSet::new(),
        library_match_all_people: false,
        library_filters_dirty: true,
        refreshing_library_filters: Rc::new(Cell::new(false)),
        year_filter: None,
        moment_filter: None,
        people_filter: None,
        people_filter_list: None,
        people_match_all: None,
        library_list: None,
        selecting_assets: false,
        selected_assets: HashSet::new(),
        rooms_widget: None,
        restore_backup_button: None,
        restore_recovery_entry: None,
        storage_gateway_entry: None,
        storage_gateway_connect: None,
        parent_window: Some(window.clone().upcast::<gtk::Window>().downgrade()),
        watch_scanning: Rc::new(Cell::new(false)),
    }));
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 20);
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 10);
    sidebar.set_margin_start(20);
    sidebar.set_margin_top(24);
    sidebar.set_width_request(180);
    let rooms = gtk::DropDown::from_strings(&[]);
    session.borrow_mut().rooms_widget = Some(rooms.downgrade());
    sidebar.append(&rooms);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.set_margin_top(24);
    content.set_margin_end(24);
    content.set_hexpand(true);
    let heading = gtk::Label::new(Some("A home for your memories"));
    heading.set_xalign(0.0);
    heading.add_css_class("title-1");
    content.append(&heading);
    let library_filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let library_search = gtk::SearchEntry::builder()
        .placeholder_text("Find caption, filename, or Person")
        .hexpand(true)
        .build();
    let year_filter = gtk::ComboBoxText::new();
    year_filter.set_width_request(135);
    year_filter.append(Some(""), "All years");
    year_filter.set_active_id(Some(""));
    let moment_filter = gtk::ComboBoxText::new();
    moment_filter.set_width_request(210);
    moment_filter.append(Some(""), "All Moments");
    moment_filter.set_active_id(Some(""));
    let people_filter = gtk::MenuButton::new();
    people_filter.set_label("People");
    people_filter.set_width_request(170);
    let people_popover = gtk::Popover::new();
    let people_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    people_panel.set_margin_top(12);
    people_panel.set_margin_bottom(12);
    people_panel.set_margin_start(12);
    people_panel.set_margin_end(12);
    let people_match_all = gtk::CheckButton::with_label("Match all selected People");
    let clear_people = gtk::Button::with_label("Clear People filters");
    clear_people.set_halign(gtk::Align::Start);
    let people_filter_list = gtk::Box::new(gtk::Orientation::Vertical, 4);
    people_panel.append(&people_match_all);
    people_panel.append(&clear_people);
    people_panel.append(&people_filter_list);
    people_popover.set_child(Some(&people_panel));
    people_filter.set_popover(Some(&people_popover));
    {
        let mut state = session.borrow_mut();
        state.year_filter = Some(year_filter.downgrade());
        state.moment_filter = Some(moment_filter.downgrade());
        state.people_filter = Some(people_filter.downgrade());
        state.people_filter_list = Some(people_filter_list.downgrade());
        state.people_match_all = Some(people_match_all.downgrade());
    }
    library_filters.append(&library_search);
    library_filters.append(&year_filter);
    library_filters.append(&moment_filter);
    library_filters.append(&people_filter);
    library_filters.set_visible(false);
    content.append(&library_filters);
    let status = gtk::Label::new(None);
    status.set_wrap(true);
    status.set_xalign(0.0);
    content.append(&status);
    let finder = gtk::Box::new(gtk::Orientation::Vertical, 12);
    finder.set_visible(false);
    let finder_heading = gtk::Label::new(Some("Ask your videos"));
    finder_heading.set_xalign(0.0);
    finder_heading.add_css_class("title-2");
    finder.append(&finder_heading);
    let finder_note = gtk::Label::new(Some("Search spoken words and what appears in local video frames. FFmpeg and Tesseract find on-screen words; optional whisper.cpp transcribes speech. A loopback-only local vision model can suggest visible objects in up to 48 sampled frames. Verify visual labels in the original video; no remote model is used. Follow-up clues build on your last question; a new quoted phrase starts fresh."));
    finder_note.set_wrap(true);
    finder_note.set_xalign(0.0);
    finder_note.add_css_class("dim-label");
    finder.append(&finder_note);
    let include_remote = gtk::CheckButton::with_label(
        "Include remote-only videos (download originals to this device)",
    );
    include_remote.set_halign(gtk::Align::Start);
    finder.append(&include_remote);
    let question = gtk::Entry::builder()
        .placeholder_text("Where does the sign say ‘Happy Birthday’?")
        .hexpand(true)
        .build();
    let mode = gtk::DropDown::from_strings(&["Speech + frames", "Spoken words", "On-screen text"]);
    let ask = gtk::Button::with_label("Find moments");
    let clear = gtk::Button::with_label("Clear");
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.append(&question);
    actions.append(&mode);
    actions.append(&ask);
    actions.append(&clear);
    finder.append(&actions);
    let coverage = gtk::Label::new(None);
    coverage.set_xalign(0.0);
    coverage.add_css_class("dim-label");
    finder.append(&coverage);
    content.append(&finder);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    session.borrow_mut().library_list = Some(list.downgrade());
    {
        let s = session.clone();
        let l = list.clone();
        library_search.connect_search_changed(move |entry| {
            s.borrow_mut().library_search = entry.text().to_string();
            render(&s, &l);
        });
    }
    {
        let s = session.clone();
        let l = list.clone();
        let refreshing = session.borrow().refreshing_library_filters.clone();
        year_filter.connect_changed(move |filter| {
            if refreshing.get() {
                return;
            }
            s.borrow_mut().library_year = filter
                .active_id()
                .map_or_else(String::new, |id| id.to_string());
            render(&s, &l);
        });
    }
    {
        let s = session.clone();
        let l = list.clone();
        let refreshing = session.borrow().refreshing_library_filters.clone();
        moment_filter.connect_changed(move |filter| {
            if refreshing.get() {
                return;
            }
            s.borrow_mut().library_moment = filter
                .active_id()
                .map_or_else(String::new, |id| id.to_string());
            render(&s, &l);
        });
    }
    {
        let s = session.clone();
        let l = list.clone();
        let refreshing = session.borrow().refreshing_library_filters.clone();
        people_match_all.connect_toggled(move |toggle| {
            if refreshing.get() {
                return;
            }
            s.borrow_mut().library_match_all_people = toggle.is_active();
            render(&s, &l);
        });
    }
    {
        let s = session.clone();
        let l = list.clone();
        clear_people.connect_clicked(move |_| {
            {
                let mut state = s.borrow_mut();
                state.library_people.clear();
                state.library_match_all_people = false;
                state.library_filters_dirty = true;
            }
            render(&s, &l);
        });
    }
    let scroll = gtk::ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .build();
    let create = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let room_name = gtk::Entry::builder()
        .placeholder_text("New Room name")
        .hexpand(true)
        .build();
    let create_button = gtk::Button::with_label("Create Room");
    create.append(&room_name);
    create.append(&create_button);
    content.append(&create);
    let add = gtk::Button::with_label("Import photos and videos");
    content.append(&add);
    content.append(&scroll);
    for name in [
        "Home",
        "Library",
        "Recently Deleted",
        "Albums",
        "People",
        "Theater",
        "Ask your videos",
        "Room settings",
        "Storage",
    ] {
        let button = gtk::Button::with_label(name);
        let s = session.clone();
        let l = list.clone();
        let finder = finder.clone();
        let create = create.clone();
        let add = add.clone();
        let scroll = scroll.clone();
        let heading = heading.clone();
        let library_filters = library_filters.clone();
        let search_box = library_search.clone();
        let year_filter = year_filter.clone();
        let moment_filter = moment_filter.clone();
        let searchable_section = matches!(name, "Library" | "Recently Deleted");
        button.connect_clicked(move |_| {
            let mut state = s.borrow_mut();
            if state.section != name {
                state.selecting_assets = false;
                state.selected_assets.clear();
            }
            state.section = name.into();
            if !searchable_section {
                state.library_search.clear();
                state.library_year.clear();
                state.library_moment.clear();
                state.library_people.clear();
                state.library_match_all_people = false;
                state.library_filters_dirty = true;
            }
            drop(state);
            library_filters.set_visible(searchable_section);
            if !searchable_section {
                let refreshing = s.borrow().refreshing_library_filters.clone();
                refreshing.set(true);
                search_box.set_text("");
                year_filter.set_active_id(Some(""));
                moment_filter.set_active_id(Some(""));
                refreshing.set(false);
            }
            let is_finder = name == "Ask your videos";
            finder.set_visible(is_finder);
            create.set_visible(!is_finder);
            add.set_visible(!is_finder);
            scroll.set_visible(true);
            heading.set_text(if is_finder {
                "Find exact video moments"
            } else if name == "Theater" {
                "Theater"
            } else if name == "Room settings" {
                "Room members"
            } else {
                name
            });
            render(&s, &l);
        });
        sidebar.append(&button);
    }
    {
        let s = session.clone();
        let l = list.clone();
        let st = status.clone();
        let c = coverage.clone();
        let p = window.clone().upcast::<gtk::Window>();
        let b = ask.clone();
        let q = question.clone();
        let m = mode.clone();
        let r = include_remote.clone();
        ask.connect_clicked(move |_| {
            let modality = match m.selected() {
                1 => "audio",
                2 => "frame",
                _ => "all",
            };
            start_video_search(
                s.clone(),
                l.clone(),
                st.clone(),
                c.clone(),
                p.clone(),
                b.clone(),
                q.text().to_string(),
                modality.into(),
                r.is_active(),
            );
        });
    }
    {
        let s = session.clone();
        let l = list.clone();
        let st = status.clone();
        let c = coverage.clone();
        let p = window.clone().upcast::<gtk::Window>();
        let b = ask.clone();
        let q = question.clone();
        let m = mode.clone();
        let r = include_remote.clone();
        question.connect_activate(move |_| {
            let modality = match m.selected() {
                1 => "audio",
                2 => "frame",
                _ => "all",
            };
            start_video_search(
                s.clone(),
                l.clone(),
                st.clone(),
                c.clone(),
                p.clone(),
                b.clone(),
                q.text().to_string(),
                modality.into(),
                r.is_active(),
            );
        });
    }
    {
        let l = list.clone();
        let st = status.clone();
        let c = coverage.clone();
        let q = question.clone();
        let context_session = session.clone();
        clear.connect_clicked(move |_| {
            context_session.borrow_mut().last_video_prompt = None;
            q.set_text("");
            st.set_label("");
            c.set_label("");
            while let Some(child) = l.first_child() {
                l.remove(&child);
            }
        });
    }
    let code = gtk::Label::new(None);
    code.set_selectable(true);
    code.set_wrap(true);
    sidebar.append(&code);
    let invitation = gtk::Entry::builder()
        .placeholder_text("Device-bound invitation")
        .build();
    sidebar.append(&invitation);
    let invitation_review = gtk::Label::new(None);
    let reviewed_token = Rc::new(RefCell::new(None::<String>));
    invitation_review.set_xalign(0.0);
    invitation_review.set_wrap(true);
    sidebar.append(&invitation_review);
    let review_invitation = gtk::Button::with_label("Review invitation");
    sidebar.append(&review_invitation);
    let save_invitation = gtk::Button::with_label("Save for review");
    save_invitation.set_sensitive(false);
    sidebar.append(&save_invitation);
    {
        let s = session.clone();
        let token = invitation.clone();
        let preview = invitation_review.clone();
        let stage = save_invitation.clone();
        let reviewed = reviewed_token.clone();
        review_invitation.connect_clicked(move |button| {
            button.set_sensitive(false);
            let review = button.clone();
            let s = s.clone();
            let token = token.text().to_string();
            let preview = preview.clone();
            let stage = stage.clone();
            let reviewed = reviewed.clone();
            let reviewed_value = token.clone();
            glib::spawn_future_local(async move {
                let core = s.borrow().core.clone();
                match command(
                    core,
                    json!({"action":"preview_invitation","invitation":token}),
                )
                .await
                {
                    Ok(value) => {
                        preview.set_text(&format!(
                            "Invited to {} · {} access · owner device {}",
                            value["name"].as_str().unwrap_or("Room"),
                            value["role"].as_str().unwrap_or("viewer"),
                            value["owner_id"]
                                .as_str()
                                .unwrap_or("")
                                .chars()
                                .take(8)
                                .collect::<String>()
                        ));
                        *reviewed.borrow_mut() = Some(reviewed_value);
                        stage.set_sensitive(true);
                    }
                    Err(error) => {
                        *reviewed.borrow_mut() = None;
                        preview.set_text(&error);
                        stage.set_sensitive(false);
                    }
                }
                review.set_sensitive(true);
            });
        });
    }
    {
        let s = session.clone();
        let r = rooms.clone();
        let l = list.clone();
        let entry = invitation.clone();
        let preview = invitation_review.clone();
        let reviewed = reviewed_token.clone();
        let stage = save_invitation.clone();
        save_invitation.connect_clicked(move |button| {
            button.set_sensitive(false);
            let s = s.clone();
            let r = r.clone();
            let l = l.clone();
            let entry = entry.clone();
            let token = reviewed.borrow().clone().unwrap_or_default();
            let reviewed = reviewed.clone();
            let preview = preview.clone();
            let stage = stage.clone();
            glib::spawn_future_local(async move {
                let core = s.borrow().core.clone();
                match command(
                    core,
                    json!({"action":"stage_invitation","invitation":token}),
                )
                .await
                {
                    Ok(_) => {
                        *reviewed.borrow_mut() = None;
                        entry.set_text("");
                        s.borrow_mut().section = "Room settings".into();
                        if let Err(error) = reload(s.clone(), r, l).await {
                            preview.set_text(&error);
                        } else {
                            preview.set_text(
                                "Invitation saved. Accept or decline it in Room settings.",
                            );
                        }
                        stage.set_sensitive(false);
                    }
                    Err(error) => {
                        preview.set_text(&error);
                        stage.set_sensitive(true);
                    }
                }
            });
        });
    }
    {
        let reviewed = reviewed_token.clone();
        let stage = save_invitation.clone();
        invitation.connect_changed(move |_| {
            *reviewed.borrow_mut() = None;
            stage.set_sensitive(false);
        });
    }
    let gateway = gtk::Entry::builder()
        .placeholder_text("Gateway HTTPS address")
        .build();
    let connect = gtk::Button::with_label("Connect gateway");
    let sync = gtk::Button::with_label("Sync Room");
    {
        let mut state = session.borrow_mut();
        state.storage_gateway_entry = Some(gateway.downgrade());
        state.storage_gateway_connect = Some(connect.downgrade());
    }
    sidebar.append(&gateway);
    sidebar.append(&connect);
    sidebar.append(&sync);

    let recovery_heading = gtk::Label::new(Some("Recovery & backup"));
    recovery_heading.set_xalign(0.0);
    recovery_heading.add_css_class("heading");
    sidebar.append(&recovery_heading);

    let recovery_key_label = gtk::Label::new(None);
    recovery_key_label.set_selectable(true);
    recovery_key_label.set_wrap(true);
    recovery_key_label.set_visible(false);
    sidebar.append(&recovery_key_label);

    let reveal_recovery_key = gtk::Button::with_label("Show recovery key");
    sidebar.append(&reveal_recovery_key);
    {
        let session = session.clone();
        let label = recovery_key_label.clone();
        reveal_recovery_key.connect_clicked(move |button| {
            if label.is_visible() {
                label.set_visible(false);
                label.set_text("");
                button.set_label("Show recovery key");
            } else {
                label.set_text(&session.borrow().recovery_key);
                label.set_visible(true);
                button.set_label("Hide recovery key");
            }
        });
    }

    let backup = gtk::Button::with_label("Create encrypted backup");
    sidebar.append(&backup);
    {
        let session = session.clone();
        let parent = window.clone();
        let status = status.clone();
        backup.connect_clicked(move |_| {
            let session = session.clone();
            let parent = parent.clone();
            let status = status.clone();
            glib::spawn_future_local(async move {
                let dialog = gtk::FileDialog::builder()
                    .title("Save encrypted Family Room backup")
                    .initial_name("Family-Room.frbackup")
                    .build();
                match dialog.save_future(Some(&parent)).await {
                    Ok(file) => match file.path() {
                        Some(path) => {
                            let core = session.borrow().core.clone();
                            let path = path.to_string_lossy().into_owned();
                            match command(core, json!({"action":"backup", "path":path})).await {
                                Ok(_) => status.set_text("Encrypted backup saved."),
                                Err(error) => status.set_text(&format!("Backup failed: {error}")),
                            }
                        }
                        None => status.set_text("Choose a local file location for the backup."),
                    },
                    Err(error) => status.set_text(&format!("Backup not saved: {error}")),
                }
            });
        });
    }

    let restore_recovery_entry = gtk::PasswordEntry::builder()
        .placeholder_text("Recovery key")
        .show_peek_icon(true)
        .build();
    sidebar.append(&restore_recovery_entry);
    session.borrow_mut().restore_recovery_entry = Some(restore_recovery_entry.downgrade());

    let restore_backup = gtk::Button::with_label("Restore into empty library");
    restore_backup.set_sensitive(false);
    sidebar.append(&restore_backup);
    session.borrow_mut().restore_backup_button = Some(restore_backup.downgrade());

    {
        let session = session.clone();
        let restore = restore_backup.clone();
        restore_recovery_entry.connect_changed(move |entry| {
            let session = session.borrow();
            restore.set_sensitive(
                can_restore_backup(&session.snapshot) && !entry.text().trim().is_empty(),
            );
        });
    }

    {
        let session = session.clone();
        let parent = window.clone();
        let rooms = rooms.clone();
        let list = list.clone();
        let status = status.clone();
        let key_entry = restore_recovery_entry.clone();
        restore_backup.connect_clicked(move |_| {
            let session = session.clone();
            let parent = parent.clone();
            let rooms = rooms.clone();
            let list = list.clone();
            let status = status.clone();
            let key_entry = key_entry.clone();
            glib::spawn_future_local(async move {
                if !can_restore_backup(&session.borrow().snapshot) {
                    status.set_text("Restore is available only when this library has no Rooms or pending invitations.");
                    return;
                }
                let recovery_key = key_entry.text().trim().to_owned();
                if recovery_key.is_empty() {
                    status.set_text("Enter the recovery key for this backup.");
                    return;
                }

                let dialog = gtk::FileDialog::builder()
                    .title("Choose encrypted Family Room backup")
                    .build();
                match dialog.open_future(Some(&parent)).await {
                    Ok(file) => match file.path() {
                        Some(path) => {
                            let core = session.borrow().core.clone();
                            let path = path.to_string_lossy().into_owned();
                            match command(
                                core,
                                json!({"action":"restore", "path":path, "recovery_key":recovery_key}),
                            )
                            .await
                            {
                                Ok(_) => {
                                    key_entry.set_text("");
                                    if let Err(error) = reload(session.clone(), rooms, list).await {
                                        status.set_text(&format!("Backup restored, but the library could not refresh: {error}"));
                                    } else {
                                        status.set_text("Backup restored. Review storage sources before syncing.");
                                    }
                                }
                                Err(error) => status.set_text(&format!("Restore failed: {error}")),
                            }
                        }
                        None => status.set_text("Choose a local backup file."),
                    },
                    Err(error) => status.set_text(&format!("Restore canceled: {error}")),
                }
            });
        });
    }
    {
        let s = session.clone();
        let r = rooms.clone();
        let l = list.clone();
        let status = status.clone();
        connect.connect_clicked(move|_| { let s=s.clone(); let r=r.clone(); let l=l.clone(); let status=status.clone(); let endpoint=gateway.text().to_string(); glib::spawn_future_local(async move { let core=s.borrow().core.clone(); let room_id=s.borrow().room_id.clone(); match command(core,json!({"action":"add_source","room_id":room_id,"name":"Room gateway","kind":"gateway","endpoint":endpoint})).await { Ok(_)=>{if let Err(e)=reload(s,r,l).await {status.set_text(&e);}},Err(e)=>status.set_text(&e)} }); });
    }
    {
        let s = session.clone();
        let r = rooms.clone();
        let l = list.clone();
        let status = status.clone();
        sync.connect_clicked(move |_| {
            let s = s.clone();
            let r = r.clone();
            let l = l.clone();
            let status = status.clone();
            glib::spawn_future_local(async move {
                let core = s.borrow().core.clone();
                let room_id = s.borrow().room_id.clone();
                match command(core, json!({"action":"sync_all","room_id":room_id})).await {
                    Ok(result) => {
                    if result["errors"]
                        .as_array()
                        .is_some_and(|errors| !errors.is_empty())
                    {
                        status.set_text(&result["errors"].to_string());
                    } else if result["cache_cleanup"]["enabled"] == true
                        && result["cache_cleanup"]["complete"] == false
                    {
                        status.set_text("Room synchronized. Automatic cache cleanup is waiting for another verified copy; originals remain saved here.");
                    } else {
                        status.set_text("Room synchronized.");
                    }
                        if let Err(e) = reload(s, r, l).await {
                            status.set_text(&e);
                        }
                    }
                    Err(e) => status.set_text(&e),
                }
            });
        });
    }
    {
        let s = session.clone();
        let r = rooms.clone();
        let l = list.clone();
        let status = status.clone();
        create_button.connect_clicked(move |_| {
            let s = s.clone();
            let r = r.clone();
            let l = l.clone();
            let status = status.clone();
            let name = room_name.text().to_string();
            glib::spawn_future_local(async move {
                let core = s.borrow().core.clone();
                match command(core, json!({"action":"create_room","name":name})).await {
                    Ok(value) => {
                        s.borrow_mut().room_id = value["id"].as_str().unwrap_or("").into();
                        s.borrow_mut().moment_river_year.clear();
                        if let Err(e) = reload(s, r, l).await {
                            status.set_text(&e);
                        }
                    }
                    Err(e) => status.set_text(&e),
                }
            });
        });
    }
    {
        let s = session.clone();
        let l = list.clone();
        rooms.connect_selected_notify(move |dropdown| {
            let index = dropdown.selected() as usize;
            let room = s.borrow().snapshot["rooms"][index]["id"]
                .as_str()
                .unwrap_or("")
                .to_owned();
            let mut state = s.borrow_mut();
            state.room_id = room;
            state.moment_river_year.clear();
            state.library_filters_dirty = true;
            state.selecting_assets = false;
            state.selected_assets.clear();
            drop(state);
            render(&s, &l);
        });
    }
    {
        let s = session.clone();
        let r = rooms.clone();
        let l = list.clone();
        let status = status.clone();
        let parent = window.clone();
        add.connect_clicked(move |_| {
            let s = s.clone();
            let r = r.clone();
            let l = l.clone();
            let status = status.clone();
            let parent = parent.clone();
            let destination = s.borrow().room_id.clone();
            glib::spawn_future_local(async move {
                let dialog = gtk::FileDialog::builder()
                    .title("Bring in memories")
                    .build();
                if let Ok(files) = dialog.open_multiple_future(Some(&parent)).await {
                    let core = s.borrow().core.clone();
                    let room_id = destination;
                    for i in 0..files.n_items() {
                        if let Some(file) = files.item(i).and_downcast::<gio::File>() {
                            if let Some(path) = file.path() {
                                if let Err(e) = command(
                                    core.clone(),
                                    json!({"action":"import","room_id":room_id,"path":path}),
                                )
                                .await
                                {
                                    status.set_text(&e);
                                }
                            }
                        }
                    }
                    if let Err(e) = reload(s, r, l).await {
                        status.set_text(&e);
                    }
                }
            });
        });
    }
    root.append(&sidebar);
    root.append(&content);
    window.set_child(Some(&root));
    window.present();
    glib::spawn_future_local(async move {
        if let Err(e) = reload(session.clone(), rooms.clone(), list.clone()).await {
            status.set_text(&e);
        }
        let device = session.borrow().snapshot["device_id"]
            .as_str()
            .unwrap_or("")
            .to_owned();
        code.set_text(&format!("Device code\n{device}"));
        schedule_folder_watch_scans(session, rooms, list, status);
    });
}
fn main() {
    #[cfg(target_os = "linux")]
    let app = adw::Application::builder()
        .application_id("com.familyroom.app")
        .build();
    #[cfg(not(target_os = "linux"))]
    let app = gtk::Application::builder()
        .application_id("com.familyroom.app")
        .build();
    app.connect_activate(|app| activate(app.upcast_ref()));
    app.run();
}

#[cfg(test)]
mod collection_tests {
    use super::*;

    #[test]
    fn spoken_caption_cues_are_validated_sorted_and_active_with_half_open_bounds() {
        let cues = spoken_caption_cues(&json!({
            "cues": [
                {"start_ms": 1_250, "end_ms": 2_500, "text": "Second cue"},
                {"start_ms": 0, "end_ms": 1_000, "text": " First cue "},
                {"start_ms": 5, "end_ms": 5, "text": "Zero width"},
                {"start_ms": 6, "end_ms": 7, "text": "  "},
                {"start_ms": 0, "end_ms": 1_000, "text": "First cue"}
            ]
        }));

        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].text, "First cue");
        assert!(cues[0].is_active_at(0));
        assert!(cues[0].is_active_at(999));
        assert!(!cues[0].is_active_at(1_000));
        assert_eq!(cues[1].text, "Second cue");
    }

    #[test]
    fn backup_restore_requires_empty_library_and_no_pending_invitations() {
        assert!(can_restore_backup(&json!({
            "rooms": [],
            "pending_invitations": []
        })));
        assert!(!can_restore_backup(&json!({
            "rooms": [{"id":"room"}],
            "pending_invitations": []
        })));
        assert!(!can_restore_backup(&json!({
            "rooms": [],
            "pending_invitations": [{"id":"invite"}]
        })));
        assert!(!can_restore_backup(&json!({"rooms": []})));
    }

    #[test]
    fn room_setup_status_is_room_scoped_and_ignores_deleted_media() {
        let room = json!({
            "id": "room-a",
            "role": "owner",
            "assets": [
                {"kind": "photo", "deleted": true},
                {"kind": "video", "deleted": false}
            ]
        });
        let snapshot = json!({
            "sources": [
                {"room_id": "room-a", "preferred": true, "paused": true},
                {"room_id": "room-b", "preferred": true, "paused": false}
            ]
        });

        assert_eq!(
            room_setup_status(&room, &snapshot, " recovery key "),
            RoomSetupStatus {
                can_manage_storage: true,
                has_preferred_storage: false,
                recovery_key_available: true,
                has_first_contribution: true,
                has_active_folder_watch: false,
            }
        );

        let ready_snapshot = json!({
            "sources": [
                {"room_id": "room-a", "preferred": true, "paused": false}
            ],
            "watches": [
                {"room_id": "room-a", "path": "/photos", "paused": false}
            ]
        });
        let ready_status = room_setup_status(&room, &ready_snapshot, "key");
        assert!(ready_status.has_preferred_storage);
        assert!(ready_status.has_active_folder_watch);

        let member = json!({"id": "room-a", "role": "viewer", "assets": []});
        let member_status = room_setup_status(&member, &snapshot, "");
        assert!(!member_status.can_manage_storage);
        assert!(!member_status.recovery_key_available);
    }

    #[test]
    fn reviewed_moment_selection_stays_with_suggestion_and_in_capture_order() {
        let suggested = vec!["first".into(), "second".into(), "third".into()];
        let selected = HashSet::from([
            "third".to_owned(),
            "not-in-suggestion".to_owned(),
            "first".to_owned(),
        ]);
        assert_eq!(
            selected_moment_asset_ids(&suggested, &selected),
            vec!["first", "third"]
        );
    }

    #[test]
    fn remote_video_indexing_requires_explicit_opt_in() {
        assert!(should_index_video(true, true, false));
        assert!(!should_index_video(false, true, false));
        assert!(should_index_video(false, true, true));
        assert!(!should_index_video(false, false, true));
    }

    #[test]
    fn album_publication_only_fetches_album_originals_and_live_components() {
        let room = json!({
            "assets": [
                {"id":"photo","deleted":false,"components":["motion","deleted-motion"]},
                {"id":"motion","deleted":false},
                {"id":"deleted-motion","deleted":true},
                {"id":"deleted-photo","deleted":true,"components":["orphan-motion"]},
                {"id":"orphan-motion","deleted":false},
                {"id":"unrelated","deleted":false}
            ]
        });
        let album = json!({"asset_ids":["photo","deleted-photo"]});
        assert_eq!(
            album_publication_asset_ids(&room, &album),
            vec!["photo", "motion"]
        );
    }

    #[test]
    fn album_publication_is_limited_to_owner_and_admin_room_roles() {
        assert!(can_publish_album(&json!({"role":"owner"})));
        assert!(can_publish_album(&json!({"role":"admin"})));
        assert!(!can_publish_album(&json!({"role":"contributor"})));
        assert!(!can_publish_album(&json!({"role":"viewer"})));
        assert!(!can_publish_album(
            &json!({"role":"owner","album_only":true})
        ));
    }

    #[test]
    fn member_controls_are_owner_scoped_and_album_only_limits_roles() {
        let owner_room = json!({"role":"owner","owner_id":"owner-device"});
        let member = json!({"device_id":"member-device"});
        let owner = json!({"device_id":"owner-device"});
        assert_eq!(
            member_role_options(&owner_room, &member),
            vec!["viewer", "contributor", "admin"]
        );
        assert!(member_role_options(&owner_room, &owner).is_empty());

        let album_room = json!({"role":"owner","owner_id":"owner-device","album_only":true});
        assert_eq!(member_role_options(&album_room, &member), vec!["viewer"]);

        let viewer_room = json!({"role":"viewer","owner_id":"owner-device"});
        assert!(member_role_options(&viewer_room, &member).is_empty());
    }

    #[test]
    fn album_command_keeps_order_and_pinning() {
        let item = json!({"id":"album","pinned":true});
        let value = collection_payload(
            "room",
            "album",
            &item,
            "Autumn",
            &["second".into(), "first".into()],
        );
        assert_eq!(value["action"], "save_album");
        assert_eq!(value["asset_ids"], json!(["second", "first"]));
        assert_eq!(value["pinned"], true);
    }

    #[test]
    fn playlist_command_uses_published_story_references() {
        let value = collection_payload(
            "room",
            "playlist",
            &json!({"id":"queue"}),
            "Friday",
            &["film".into()],
        );
        assert_eq!(value["action"], "save_playlist");
        assert_eq!(value["story_ids"], json!(["film"]));
        assert!(value.get("asset_ids").is_none());
    }
    #[test]
    fn playlist_playback_preserves_order_and_rejects_missing_films() {
        let room = json!({
            "stories": [
                {"id":"first","private_draft":false,"published_asset":"movie-a"},
                {"id":"second","private_draft":false,"published_asset":"movie-b"},
                {"id":"draft","private_draft":true,"published_asset":"movie-c"}
            ],
            "assets": [
                {"id":"movie-a","deleted":false},
                {"id":"movie-b","deleted":false},
                {"id":"movie-c","deleted":false}
            ]
        });
        let ordered = playlist_films(&room, &json!({"story_ids":["second","first"]})).unwrap();
        assert_eq!(
            ordered
                .iter()
                .map(|movie| movie["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["movie-b", "movie-a"]
        );
        assert!(playlist_films(&room, &json!({"story_ids":["draft"]})).is_err());
        assert!(playlist_films(&room, &json!({"story_ids":["missing"]})).is_err());
        assert!(playlist_films(&room, &json!({"story_ids":[]})).is_err());
        let mut deleted = room.clone();
        deleted["assets"][0]["deleted"] = json!(true);
        assert!(playlist_films(&deleted, &json!({"story_ids":["first"]})).is_err());
    }

    #[test]
    fn library_batch_selection_excludes_deleted_assets_and_caps_at_core_limit() {
        let room = json!({"assets":[
            {"id":"photo","deleted":false},
            {"id":"video","deleted":false},
            {"id":"deleted","deleted":true}
        ]});
        let selected = ["photo", "deleted", "stale"]
            .into_iter()
            .map(str::to_owned)
            .collect::<HashSet<_>>();
        assert_eq!(
            selected_library_asset_ids(&room, &selected, false),
            vec!["photo"]
        );
        assert_eq!(
            selected_library_asset_ids(&room, &selected, true),
            vec!["deleted"]
        );

        let mut bounded = (0..MAX_BULK_SELECTION)
            .map(|index| format!("asset-{index}"))
            .collect::<HashSet<_>>();
        assert!(insert_selected_asset(&mut bounded, "asset-0"));
        assert!(!insert_selected_asset(&mut bounded, "one-too-many"));
        assert_eq!(bounded.len(), MAX_BULK_SELECTION);
    }

    #[test]
    fn bulk_favorite_targets_one_consistent_state_for_mixed_selection() {
        let room = json!({"assets":[
            {"id":"favorite","favorite":true},
            {"id":"ordinary","favorite":false}
        ]});
        assert!(!next_bulk_favorite_value(&room, &["favorite".into()]));
        assert!(next_bulk_favorite_value(
            &room,
            &["favorite".into(), "ordinary".into()]
        ));
    }

    #[test]
    fn library_batch_actions_follow_room_edit_roles() {
        for role in ["owner", "admin", "contributor"] {
            assert!(can_edit_assets(&json!({"role":role})), "{role}");
        }
        for role in ["viewer", "removed", "unknown"] {
            assert!(!can_edit_assets(&json!({"role":role})), "{role}");
        }
    }

    #[test]
    fn deletion_actions_are_limited_to_owner_admin_or_selected_contributors() {
        let room = json!({"role":"contributor","assets":[
            {"id":"mine","contributor":"device-a"},
            {"id":"theirs","contributor":"device-b"}
        ]});
        assert!(can_change_deletion_for_selection(
            &room,
            "device-a",
            &["mine".into()]
        ));
        assert!(!can_change_deletion_for_selection(
            &room,
            "device-a",
            &["mine".into(), "theirs".into()]
        ));
        assert!(can_change_deletion_for_selection(
            &json!({"role":"admin"}),
            "device-a",
            &["theirs".into()]
        ));
        assert!(!can_change_deletion_for_selection(&room, "device-a", &[]));
    }

    #[test]
    fn bulk_capture_date_correction_preserves_local_time_and_spacing() {
        let original = glib::DateTime::from_local(2020, 2, 3, 14, 27, 31.0).unwrap();
        let first = original.to_unix();
        let second = first + 3_600;
        let corrected_first = corrected_capture_timestamp(first, 2024, 11, 5).unwrap();
        let corrected_second = corrected_capture_timestamp(second, 2024, 11, 5).unwrap();
        let local = glib::DateTime::from_unix_local(corrected_first).unwrap();

        assert_eq!(
            (local.year(), local.month(), local.day_of_month()),
            (2024, 11, 5)
        );
        assert_eq!((local.hour(), local.minute(), local.second()), (14, 27, 31));
        assert_eq!(corrected_second - corrected_first, 3_600);
        assert!(corrected_capture_timestamp(first, 2024, 2, 30).is_none());
    }

    #[test]
    fn library_search_combines_text_year_moment_and_any_or_all_people_filters() {
        let room = json!({
            "people":[
                {"id":"person-1","name":"Grandma May"},
                {"id":"person-2","name":"Uncle Rob"}
            ],
            "moments":[
                {"id":"moment-birthday","title":"Birthday at the lake","asset_ids":["asset-both","asset-one"]},
                {"id":"moment-home","title":"Birthday at home","asset_ids":["asset-one"]}
            ]
        });
        let captured_at = glib::DateTime::from_local(2020, 5, 6, 14, 27, 31.0)
            .unwrap()
            .to_unix();
        let both_people = json!({
            "id":"asset-both",
            "caption":"Birthday at lake",
            "filename":"summer-trip.mov",
            "kind":"video",
            "captured_at":captured_at,
            "people":["person-1","person-2"]
        });
        let one_person = json!({
            "id":"asset-one",
            "caption":"Birthday at home",
            "filename":"birthday.mov",
            "kind":"video",
            "captured_at":captured_at,
            "people":["person-1"]
        });
        let selected = HashSet::from(["person-1".to_owned(), "person-2".to_owned()]);

        for query in [
            "birthday",
            "summer-trip",
            "video",
            "grandma",
            "birthday uncle",
        ] {
            assert!(library_asset_matches_filters(
                &room,
                &both_people,
                query,
                "2020",
                "",
                &selected,
                false,
            ));
        }
        assert!(library_asset_matches_filters(
            &room,
            &both_people,
            "birthday",
            "2020",
            "moment-birthday",
            &selected,
            false,
        ));
        assert!(library_asset_matches_filters(
            &room,
            &both_people,
            "birthday",
            "2020",
            "",
            &selected,
            true,
        ));
        assert!(library_asset_matches_filters(
            &room,
            &one_person,
            "birthday",
            "2020",
            "",
            &selected,
            false,
        ));
        assert!(!library_asset_matches_filters(
            &room,
            &one_person,
            "birthday",
            "2020",
            "",
            &selected,
            true,
        ));
        assert!(!library_asset_matches_filters(
            &room,
            &both_people,
            "birthday",
            "2021",
            "",
            &selected,
            false,
        ));
        assert!(!library_asset_matches_filters(
            &room,
            &both_people,
            "birthday",
            "2020",
            "",
            &HashSet::from(["person-3".to_owned()]),
            false,
        ));
        assert!(!library_asset_matches_filters(
            &room,
            &both_people,
            "birthday",
            "2020",
            "moment-home",
            &selected,
            false,
        ));
        assert!(library_asset_matches_filters(
            &room,
            &one_person,
            "",
            "",
            "",
            &HashSet::new(),
            true,
        ));
    }
}
