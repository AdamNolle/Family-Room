use crate::{
    access, crypto,
    error::{invalid, CoreError, Result},
    model::*,
    signing,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use zeroize::Zeroize;
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

pub struct Engine {
    pub root: PathBuf,
    master: [u8; 32],
    pub state: State,
    _lock: fs::File,
    network_state: String,
    video_search_cache: Option<(String, Option<crate::video_index::VideoSearchCache>)>,
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.master.zeroize();
        self.state.signing_seed.zeroize();
        for r in &mut self.state.rooms {
            r.secret.zeroize();
            for value in r.epoch_secrets.values_mut() {
                value.zeroize();
            }
        }
        for source in &mut self.state.sources {
            source.token.zeroize();
        }
        for invitation in &mut self.state.pending_invitations {
            invitation.invitation.zeroize();
        }
    }
}
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
pub fn id() -> String {
    Uuid::new_v4().to_string()
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| invalid(format!("Missing {key}")))
}

fn person_merge_map(entities: &BTreeMap<String, Value>) -> BTreeMap<String, String> {
    entities
        .iter()
        .filter_map(|(key, value)| {
            let source = key.strip_prefix("person_merge:")?;
            let target = value["target_person_id"].as_str()?;
            Some((source.to_owned(), target.to_owned()))
        })
        .collect()
}

fn canonical_person_id(person_id: &str, merges: &BTreeMap<String, String>) -> String {
    let mut current = person_id.to_owned();
    let mut path = Vec::new();
    let mut visited = BTreeMap::new();
    loop {
        if let Some(cycle_start) = visited.get(&current) {
            // Concurrent offline merges can form a cycle. All devices choose
            // the same visible profile by retaining the lexicographically
            // first ID in that cycle.
            return path[*cycle_start..]
                .iter()
                .min()
                .cloned()
                .unwrap_or(current);
        }
        visited.insert(current.clone(), path.len());
        path.push(current.clone());
        match merges.get(&current) {
            Some(target) => current.clone_from(target),
            None => return current,
        }
    }
}
fn original_export_filename(asset: &MediaAsset) -> String {
    let cleaned: String = asset
        .filename
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim_end_matches([' ', '.']);
    let (stem, extension) = cleaned
        .rsplit_once('.')
        .filter(|(_, ext)| {
            !ext.is_empty() && ext.len() <= 16 && ext.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .map_or((cleaned, String::new()), |(stem, ext)| {
            (stem, format!(".{ext}"))
        });
    // Budget bytes as well as characters for Unix and Windows filesystem limits.
    // The content ID prefix prevents device names and case-folding collisions.
    let limit = 160 - extension.len();
    let mut shortened = String::new();
    for c in stem.chars() {
        if shortened.len() + c.len_utf8() > limit {
            break;
        }
        shortened.push(c);
    }
    let shortened = shortened.trim_end_matches([' ', '.']);
    format!(
        "{}-{}{}",
        asset.id,
        if shortened.is_empty() {
            "original"
        } else {
            shortened
        },
        extension
    )
}
fn validate_story(story: &StoryProject) -> Result<()> {
    if story.format_version != crate::timeline::FORMAT_VERSION {
        return Err(invalid(
            "This film uses an unsupported project format. Update Family Room before editing it",
        ));
    }
    title(&story.title)?;
    if story.width < 16
        || story.height < 16
        || story.width > 7680
        || story.height > 7680
        || story.clips.len() > 10000
        || story.chapters.len() > 1000
    {
        return Err(invalid("Invalid film dimensions or project size"));
    }
    let mut ids = BTreeSet::new();
    for clip in &story.clips {
        safe_id(&clip.id)?;
        safe_id(&clip.asset_id)?;
        if !ids.insert(&clip.id)
            || clip.track > 15
            || clip.title.chars().count() > 2000
            || ![
                clip.start,
                clip.trim_in,
                clip.duration,
                clip.speed,
                clip.volume,
                clip.opacity,
                clip.exposure,
                clip.saturation,
                clip.fade_in,
                clip.fade_out,
                clip.fade_in_offset,
                clip.fade_out_offset,
            ]
            .iter()
            .all(|n| n.is_finite())
            || !(0.0..=86400.0).contains(&clip.start)
            || !(0.0..=86400.0).contains(&clip.trim_in)
            || !(0.001..=86400.0).contains(&clip.duration)
            || !(0.05..=20.0).contains(&clip.speed)
            || clip.start + clip.duration / clip.speed > 86400.0
            || !(0.0..=4.0).contains(&clip.volume)
            || !(0.0..=1.0).contains(&clip.opacity)
            || !(-8.0..=8.0).contains(&clip.exposure)
            || !(0.0..=4.0).contains(&clip.saturation)
            || !(0.0..=86400.0).contains(&clip.fade_in)
            || !(0.0..=86400.0).contains(&clip.fade_out)
            || !(0.0..=86400.0).contains(&clip.fade_in_offset)
            || !(0.0..=86400.0).contains(&clip.fade_out_offset)
            || clip.opacity_keyframes.len() + clip.volume_keyframes.len() > 10000
        {
            return Err(invalid("Invalid timeline clip"));
        }
        for (frames, maximum) in [
            (&clip.opacity_keyframes, 1.0),
            (&clip.volume_keyframes, 4.0),
        ] {
            for frame in frames {
                if !frame.time.is_finite()
                    || !(0.0..=clip.duration).contains(&frame.time)
                    || !frame.value.is_finite()
                    || !(0.0..=maximum).contains(&frame.value)
                {
                    return Err(invalid("Invalid keyframe"));
                }
            }
        }
    }
    for chapter in &story.chapters {
        if !chapter["title"]
            .as_str()
            .is_some_and(|s| !s.trim().is_empty() && s.chars().count() <= 200)
            || !chapter["time"]
                .as_f64()
                .is_some_and(|n| n.is_finite() && (0.0..=86400.0).contains(&n))
        {
            return Err(invalid("Invalid chapter"));
        }
    }
    Ok(())
}

fn title(s: &str) -> Result<String> {
    let s = s.trim();
    if s.is_empty() || s.chars().count() > 200 {
        return Err(invalid("Use a name between 1 and 200 characters"));
    }
    Ok(s.into())
}
fn exclusions(value: Option<&Value>) -> Result<Vec<String>> {
    let values: Vec<String> = serde_json::from_value(value.cloned().unwrap_or(json!([])))?;
    if values.len() > 64 {
        return Err(invalid("Use at most 64 excluded file types"));
    }
    let mut normalized = BTreeSet::new();
    for value in values {
        let extension = value.trim().trim_start_matches('.').to_ascii_lowercase();
        if extension.is_empty() {
            continue;
        }
        if extension.len() > 16 || !extension.bytes().all(|c| c.is_ascii_alphanumeric()) {
            return Err(invalid("Use file extensions such as jpg or mp4"));
        }
        normalized.insert(extension);
    }
    Ok(normalized.into_iter().collect())
}
fn safe_id(s: &str) -> Result<()> {
    if s.is_empty() || !s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-') {
        return Err(invalid("Invalid object identifier"));
    }
    Ok(())
}
pub fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Missing parent directory"))?;
    fs::create_dir_all(parent)?;
    let mut f = tempfile::NamedTempFile::new_in(parent)?;
    f.write_all(bytes)?;
    f.as_file().sync_all()?;
    f.persist(path).map_err(|e| CoreError::from(e.error))?;
    #[cfg(unix)]
    {
        fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}
impl Engine {
    pub fn open(root: impl Into<PathBuf>, master: [u8; 32]) -> Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("catalog.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock)
            .map_err(|_| invalid("This library is already open in another process"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        }
        let state = match fs::read(root.join("catalog.fr")) {
            Ok(bytes) => serde_json::from_slice(&crypto::decrypt(
                &master,
                &bytes,
                b"family-room-catalog-v1",
            )?)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => State {
                version: VERSION,
                signing_seed: hex::encode(crypto::key()),
                rooms: vec![],
                sources: vec![],
                watches: vec![],
                transfers: vec![],
                replicas: vec![],
                video_indexes: vec![],
                photo_fingerprints: vec![],
                drafts: vec![],
                album_shares: vec![],
                damaged_originals: vec![],
                pending_invitations: vec![],
                offline_pins: vec![],
                device_cache_limits: vec![],
            },
            Err(e) => return Err(e.into()),
        };
        let engine = Self {
            root,
            master,
            state,
            _lock: lock,
            network_state: "unknown".into(),
            video_search_cache: None,
        };
        engine.validate_state()?;
        engine.persist()?;
        Ok(engine)
    }
    fn validate_state(&self) -> Result<()> {
        if self.state.version != VERSION {
            return Err(invalid("This catalog requires a newer app"));
        }
        signing::identity(&self.state.signing_seed)?;
        for draft in &self.state.drafts {
            validate_story(draft)?;
        }
        for r in &self.state.rooms {
            safe_id(&r.id)?;
            crypto::parse_key(&r.secret)?;
            let root_owner = access::root_owner_id(&r.access_changes, &r.owner_id);
            let grant_owner = access::grant_issuer(r, &r.grant).unwrap_or(root_owner);
            signing::verify_grant(&r.grant, grant_owner)?;
            access::verify_chain(&r.access_changes, &r.id, root_owner, &r.owner_id)?;
            for value in r.epoch_secrets.values() {
                crypto::parse_key(value)?;
            }
            for op in &r.operations {
                let grant_owner = access::grant_issuer(r, &op.grant).unwrap_or(root_owner);
                signing::verify_operation(op, grant_owner, &r.id)?;
                if !access::allowed_operation(r, op)? {
                    return Err(CoreError::AccessDenied);
                }
                self.validate_operation_payload(op)?;
            }
        }
        Ok(())
    }
    pub fn persist(&self) -> Result<()> {
        atomic(
            &self.root.join("catalog.fr"),
            &crypto::encrypt(
                &self.master,
                &serde_json::to_vec(&self.state)?,
                b"family-room-catalog-v1",
            )?,
        )
    }
    pub fn device(&self) -> Result<String> {
        signing::device(&self.state.signing_seed)
    }
    pub fn room(&self, room_id: &str) -> Result<&Room> {
        self.state
            .rooms
            .iter()
            .find(|r| r.id == room_id)
            .ok_or_else(|| invalid("Room not found"))
    }
    fn room_mut(&mut self, room_id: &str) -> Result<&mut Room> {
        self.state
            .rooms
            .iter_mut()
            .find(|r| r.id == room_id)
            .ok_or_else(|| invalid("Room not found"))
    }
    pub fn blob(&self, room_id: &str, asset_id: &str) -> Result<PathBuf> {
        safe_id(room_id)?;
        safe_id(asset_id)?;
        Ok(self
            .root
            .join("objects")
            .join(room_id)
            .join(format!("{asset_id}.frblob")))
    }
    fn validate_asset_patch(
        &self,
        room_id: &str,
        asset: &MediaAsset,
        fields: &serde_json::Map<String, Value>,
    ) -> Result<()> {
        if fields.is_empty() {
            return Err(invalid("Media edit has no fields"));
        }
        let role = self.room(room_id)?.grant.role;
        let device = self.device()?;
        for (key, value) in fields {
            let valid = match key.as_str() {
                "caption" => value.as_str().is_some_and(|s| s.len() <= 10_000),
                "favorite" | "deleted" => value.is_boolean(),
                "captured_at" => value.is_i64(),
                "people" | "components" => value
                    .as_array()
                    .is_some_and(|values| values.iter().all(Value::is_string)),
                _ => false,
            };
            if !valid {
                return Err(invalid("Invalid media edit"));
            }
            if key == "deleted"
                && asset.contributor != device
                && !matches!(role, Role::Owner | Role::Admin)
            {
                return Err(CoreError::AccessDenied);
            }
        }
        Ok(())
    }

    fn write_op(&mut self, room_id: &str, kind: &str, entity_id: &str, value: Value) -> Result<()> {
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let clock = room
            .operations
            .iter()
            .map(|o| o.clock)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| invalid("Invalid synchronization clock"))?;
        let mut op = Operation {
            id: id(),
            clock,
            author: self.device()?,
            room_id: room_id.into(),
            grant: room.grant.clone(),
            kind: kind.into(),
            entity_id: entity_id.into(),
            value,
            signature: String::new(),
        };
        signing::sign_operation(&mut op, &self.state.signing_seed)?;
        signing::verify_operation(&op, &room.owner_id, room_id)?;
        self.room_mut(room_id)?.operations.push(op);
        Ok(())
    }
    pub fn entities(&self, room_id: &str) -> Result<BTreeMap<String, Value>> {
        let mut ops = self.room(room_id)?.operations.clone();
        ops.sort_by(|a, b| (&a.clock, &a.author, &a.id).cmp(&(&b.clock, &b.author, &b.id)));
        let mut result = BTreeMap::new();
        for op in ops {
            match op.kind.as_str() {
                "asset" => {
                    // Concurrent imports of identical bytes share one identity.
                    // Later creation records must not reset accepted metadata edits.
                    result
                        .entry(format!("asset:{}", op.entity_id))
                        .or_insert(op.value);
                }
                "album" | "album_member" | "playlist" | "moment" | "person" | "person_merge"
                | "comment" | "reaction" | "story" | "settings" => {
                    result.insert(format!("{}:{}", op.kind, op.entity_id), op.value);
                }
                "asset_patch" => {
                    if let Some(Value::Object(asset)) =
                        result.get_mut(&format!("asset:{}", op.entity_id))
                    {
                        if let Some(fields) = op.value.as_object() {
                            for (key, value) in fields {
                                asset.insert(key.clone(), value.clone());
                            }
                        }
                    }
                }
                _ => return Err(invalid("Unsupported sync operation")),
            }
        }
        // Per-asset membership registers preserve independent offline additions/removals.
        let memberships: Vec<_> = result
            .iter()
            .filter(|(k, _)| k.starts_with("album_member:"))
            .map(|(_, value)| value.clone())
            .collect();
        for member in memberships {
            let album_id = text(&member, "album_id")?;
            let asset_id = text(&member, "asset_id")?;
            let included = member["included"]
                .as_bool()
                .ok_or_else(|| invalid("Invalid album membership"))?;
            if let Some(album) = result.get_mut(&format!("album:{album_id}")) {
                let ids = album["asset_ids"]
                    .as_array_mut()
                    .ok_or_else(|| invalid("Invalid album"))?;
                if included {
                    if !ids.iter().any(|id| id == asset_id) {
                        ids.push(json!(asset_id));
                    }
                } else {
                    ids.retain(|id| id != asset_id);
                }
            }
        }
        Ok(result)
    }
    fn merge_person(&mut self, request: &Value) -> Result<Value> {
        let room_id = text(request, "room_id")?;
        if self.room(room_id)?.grant.role == Role::Viewer {
            return Err(CoreError::AccessDenied);
        }
        let source_person_id = text(request, "source_person_id")?;
        let target_person_id = text(request, "target_person_id")?;
        safe_id(source_person_id)?;
        safe_id(target_person_id)?;
        if source_person_id == target_person_id {
            return Err(invalid("Choose two different People"));
        }

        let entities = self.entities(room_id)?;
        let source: PersonTag = serde_json::from_value(
            entities
                .get(&format!("person:{source_person_id}"))
                .cloned()
                .ok_or_else(|| invalid("Person to merge was not found"))?,
        )?;
        let target: PersonTag = serde_json::from_value(
            entities
                .get(&format!("person:{target_person_id}"))
                .cloned()
                .ok_or_else(|| invalid("Keep Person was not found"))?,
        )?;
        if source.room_id != room_id || target.room_id != room_id {
            return Err(invalid("People must belong to this Room"));
        }

        let merges = person_merge_map(&entities);
        let source_root = canonical_person_id(source_person_id, &merges);
        let target_root = canonical_person_id(target_person_id, &merges);
        if source_root != source_person_id {
            return Err(invalid("Person has already been merged"));
        }
        if source_root == target_root {
            return Err(invalid("These People are already together"));
        }

        let merge = PersonMerge {
            room_id: room_id.into(),
            source_person_id: source_person_id.into(),
            target_person_id: target_root.clone(),
        };
        self.write_op(
            room_id,
            "person_merge",
            source_person_id,
            serde_json::to_value(merge)?,
        )?;
        Ok(json!({
            "source_person_id":source_person_id,
            "target_person_id":target_root
        }))
    }

    pub fn assets(&self, room_id: &str) -> Result<Vec<MediaAsset>> {
        let entities = self.entities(room_id)?;
        let merges = person_merge_map(&entities);
        entities
            .into_iter()
            .filter(|(k, _)| k.starts_with("asset:"))
            .map(|(_, value)| {
                let mut asset: MediaAsset = serde_json::from_value(value)?;
                let mut seen_people = BTreeSet::new();
                asset.people = asset
                    .people
                    .into_iter()
                    .map(|person_id| canonical_person_id(&person_id, &merges))
                    .filter(|person_id| seen_people.insert(person_id.clone()))
                    .collect();
                Ok(asset)
            })
            .collect()
    }
    fn recoverable_assets(&self, room_id: &str) -> Result<Vec<MediaAsset>> {
        let mut assets: BTreeMap<_, _> = self
            .assets(room_id)?
            .into_iter()
            .map(|a| (a.id.clone(), a))
            .collect();
        for op in &self.room(room_id)?.rejected_operations {
            if op.kind == "asset" {
                let asset: MediaAsset = serde_json::from_value(op.value.clone())?;
                assets.entry(asset.id.clone()).or_insert(asset);
            }
        }
        Ok(assets.into_values().collect())
    }
    pub fn snapshot(&self) -> Result<Value> {
        let mut rooms = vec![];
        let today = chrono::Local::now().date_naive();
        for r in &self.state.rooms {
            let entities = self.entities(&r.id)?;
            let collect = |prefix: &str| {
                entities
                    .iter()
                    .filter(|(k, _)| k.starts_with(prefix))
                    .map(|(_, v)| v.clone())
                    .collect::<Vec<_>>()
            };
            let merges = person_merge_map(&entities);
            let people: Vec<_> = collect("person:")
                .into_iter()
                .filter(|person| {
                    person["id"].as_str().is_some_and(|person_id| {
                        canonical_person_id(person_id, &merges) == person_id
                    })
                })
                .collect();
            let assets = self
                .assets(&r.id)?
                .iter()
                .map(|a| {
                    let present = self
                        .blob(&r.id, &a.id)
                        .map(|p| p.is_file())
                        .unwrap_or(false);
                    let damaged = self.state.damaged_originals.contains(&a.id);
                    let local = present && !damaged;
                    let copies = usize::from(local)
                        + self
                            .state
                            .replicas
                            .iter()
                            .filter(|p| p.asset_id == a.id && p.verified)
                            .count();
                    let mut value = serde_json::to_value(a).unwrap_or(Value::Null);
                    value["local"] = json!(local);
                    value["damaged"] = json!(damaged);
                    value["pinned_offline"] = json!(self
                        .state
                        .offline_pins
                        .iter()
                        .any(|pin| { pin.room_id == r.id && pin.asset_id == a.id }));
                    value["verified_copies"] = json!(copies);
                    value["availability"] = json!(if damaged {
                        "Local original needs repair"
                    } else if local {
                        "Saved on this device"
                    } else if copies > 0 {
                        "Original available from connected storage"
                    } else {
                        "Waiting for an original"
                    });
                    value
                })
                .collect::<Vec<_>>();
            let on_this_day_asset_ids = crate::memory_calendar::asset_ids(&assets, today);
            let mut members = BTreeMap::new();
            for op in &r.operations {
                members.insert(op.author.clone(), op.grant.clone());
            }
            members.extend(r.issued_grants.clone());
            members.insert(r.grant.device_id.clone(), r.grant.clone());
            if let Some(policy) = r.access_changes.last() {
                members = policy.members.clone();
            }
            let mut story_ops: Vec<_> = r
                .operations
                .iter()
                .filter(|op| op.kind == "story")
                .collect();
            story_ops
                .sort_by(|a, b| (&a.clock, &a.author, &a.id).cmp(&(&b.clock, &b.author, &b.id)));
            let mut stories: Vec<_> = story_ops
                .into_iter()
                .map(|op| {
                    let mut value = op.value.clone();
                    value["private_draft"] = json!(false);
                    value
                })
                .collect();
            stories.extend(
                self.state
                    .drafts
                    .iter()
                    .filter(|p| {
                        p.room_id == r.id
                            && !p.archived
                            && !r
                                .operations
                                .iter()
                                .any(|op| op.kind == "story" && op.entity_id == p.version_id)
                            && (!p.auto_generated || p.created_at >= now() - 14 * 86400)
                    })
                    .filter_map(|p| {
                        serde_json::to_value(p).ok().map(|mut value| {
                            value["private_draft"] = json!(true);
                            value
                        })
                    }),
            );
            let playlists: Vec<_> = collect("playlist:")
                .into_iter()
                .filter(|p| p["deleted"] != true)
                .collect();
            rooms.push(json!({"id":r.id,"name":r.name,"owner_id":r.owner_id,"role":if access::allowed_device(r,&self.device()?) { serde_json::to_value(r.grant.role)? } else { json!("removed") },"album_only":r.album_only,"access_revision":access::revision(r),"key_epoch":access::epoch(r),"rejected_edits":r.rejected_operations.len(),"device_cache_limit_bytes":self.state.device_cache_limits.iter().find(|policy| policy.room_id == r.id).map(|policy| policy.max_bytes),"on_this_day_asset_ids":on_this_day_asset_ids,"assets":assets,"moments":collect("moment:"),"albums":collect("album:"),"playlists":playlists,"people":people,"stories":stories,"comments":collect("comment:"),"reactions":collect("reaction:"),"members":members.values().collect::<Vec<_>>()}));
        }
        let sources=self.state.sources.iter().map(|s|json!({"id":s.id,"room_id":s.room_id,"name":s.name,"kind":s.kind,"endpoint":s.endpoint,"paused":s.paused,"preferred":self.is_preferred(s),"overflow":s.overflow,"network_policy":s.network_policy,"budget":s.budget,"capabilities":crate::providers::capabilities(&s.kind)})).collect::<Vec<_>>();
        let pending_invitations = self
            .state
            .pending_invitations
            .iter()
            .map(|invitation| {
                json!({
                    "id": invitation.id,
                    "room_id": invitation.room_id,
                    "name": invitation.name,
                    "owner_id": invitation.owner_id,
                    "role": invitation.role,
                    "album_only": invitation.album_only,
                    "created_at": invitation.created_at
                })
            })
            .collect::<Vec<_>>();
        let transfers=self.state.transfers.iter().map(|t|json!({"id":t.id,"room_id":t.room_id,"asset_id":t.asset_id,"source_id":t.source_id,"state":t.state,"transferred":t.transferred,"total":t.total,"error":t.error})).collect::<Vec<_>>();
        Ok(
            json!({"version":VERSION,"timeline_formats":[crate::timeline::FORMAT_VERSION],"device_id":self.device()?,"network_state":self.network_state,"rooms":rooms,"sources":sources,"watches":self.state.watches,"transfers":transfers,"pending_invitations":pending_invitations}),
        )
    }
    pub fn execute(&mut self, command: Value) -> Result<Value> {
        if matches!(
            command["action"].as_str(),
            Some(
                "snapshot"
                    | "preview_invitation"
                    | "search_video"
                    | "video_index_status"
                    | "export_video_transcript"
                    | "suggest_moments"
                    | "set_network_state",
            )
        ) {
            return self.dispatch(command);
        }
        if matches!(
            command["action"].as_str(),
            Some("index_video" | "clear_video_index" | "restore")
        ) {
            self.video_search_cache = None;
        }
        let mut before = self.state.clone();
        let result = self.dispatch(command).and_then(|value| {
            self.persist()?;
            Ok(value)
        });
        if result.is_err() {
            // Failed reads are observations about physical copies, independent
            // of a catalog transaction. Rolling back must not re-certify them.
            let mut changed_health = false;
            for damaged in &self.state.damaged_originals {
                if !before.damaged_originals.contains(damaged) {
                    before.damaged_originals.push(damaged.clone());
                    changed_health = true;
                }
            }
            for replica in &self.state.replicas {
                if !replica.verified {
                    for old in &mut before.replicas {
                        if old.verified
                            && old.asset_id == replica.asset_id
                            && old.source_id == replica.source_id
                        {
                            old.verified = false;
                            changed_health = true;
                        }
                    }
                }
            }
            // Valid owner-signed security updates remain monotonic even if a
            // subsequent download or payload in the same command fails.
            let learned: Vec<_> = self
                .state
                .rooms
                .iter()
                .filter(|r| {
                    before.rooms.iter().any(|old| {
                        old.id == r.id
                            && old.owner_id == r.owner_id
                            && old.secret == r.secret
                            && access::revision(r) > access::revision(old)
                    })
                })
                .map(|r| (r.id.clone(), r.access_changes.clone()))
                .collect();
            let changed_access = !learned.is_empty();
            for (room_id, chain) in learned {
                if let Some(room) = before.rooms.iter_mut().find(|r| r.id == room_id) {
                    access::apply(room, chain, &before.signing_seed)?;
                }
            }
            self.state = before;
            if changed_access || changed_health {
                self.persist()?;
            }
        }
        result
    }
    fn dispatch(&mut self, c: Value) -> Result<Value> {
        match text(&c, "action")? {
            "snapshot" => self.snapshot(),
            "merge_person" => self.merge_person(&c),
            "set_network_state" => {
                let state = text(&c, "state")?;
                if !["unknown", "offline", "metered", "unmetered"].contains(&state) {
                    return Err(invalid("Unknown network state"));
                }
                self.network_state = state.into();
                Ok(json!({"saved":true}))
            }
            "index_video" => self.index_video(&c),
            "search_video" => self.search_video(&c),
            "video_index_status" => self.video_index_status(&c),
            "export_video_transcript" => self.export_video_transcript(&c),
            "suggest_moments" => self.suggest_moments(&c),
            "suggest_photo_duplicates" => self.suggest_photo_duplicates(&c),
            "clear_video_index" => self.clear_video_index(&c),
            "revoke_device" => self.change_access(&c, true),
            "set_role" => self.change_access(&c, false),
            "rotate_keys" => self.change_access(&c, true),
            "leave_room" => self.leave_room(&c),
            "prepare_owner_transfer" => self.prepare_owner_transfer(&c),
            "accept_owner_transfer" => self.accept_owner_transfer(&c),
            "access_policy" => {
                Ok(json!({"changes":self.room(text(&c,"room_id")?)?.access_changes}))
            }
            "apply_access" => {
                let seed = self.state.signing_seed.clone();
                let changes: Vec<AccessChange> = serde_json::from_value(c["changes"].clone())?;
                let room_id = text(&c, "room_id")?;
                let changed = access::apply(self.room_mut(room_id)?, changes, &seed)?;
                Ok(
                    json!({"changed":changed,"active":access::allowed_device(self.room(room_id)?,&self.device()?)}),
                )
            }
            "publish_album" => self.publish_album(
                text(&c, "room_id")?,
                text(&c, "album_id")?,
                c["new_audience"].as_bool().unwrap_or(false),
            ),
            "invite_album" => {
                let device = text(&c, "device_id")?;
                access::seal(device, b"device verification")?;
                let publication =
                    self.publish_album(text(&c, "room_id")?, text(&c, "album_id")?, true)?;
                let scope = text(&publication, "scope_id")?;
                let mut invitation = self.issue_invitation(&json!({"room_id":scope,
                    "device_id":device,"role":"viewer"}))?;
                invitation["scope_id"] = json!(scope);
                Ok(invitation)
            }
            "create_room" => {
                let room_id = id();
                let device = self.device()?;
                let room = Room {
                    id: room_id.clone(),
                    name: title(text(&c, "name")?)?,
                    owner_id: device.clone(),
                    secret: hex::encode(crypto::key()),
                    grant: signing::grant(
                        &self.state.signing_seed,
                        &room_id,
                        &device,
                        Role::Owner,
                    )?,
                    operations: vec![],
                    album_only: false,
                    issued_grants: BTreeMap::new(),
                    access_changes: vec![],
                    epoch_secrets: BTreeMap::new(),
                    rejected_operations: vec![],
                };
                self.state.rooms.push(room);
                Ok(json!({"id":room_id}))
            }
            "import" => self.import_file(
                text(&c, "room_id")?,
                Path::new(text(&c, "path")?),
                c["captured_at"].as_i64(),
            ),
            "materialize" => {
                let room_id = text(&c, "room_id")?;
                let asset_id = text(&c, "asset_id")?;
                let asset = self
                    .assets(room_id)?
                    .into_iter()
                    .find(|a| a.id == asset_id)
                    .ok_or_else(|| invalid("Media not found"))?;
                let dest = PathBuf::from(text(&c, "destination")?);
                let result = crypto::decrypt_file(
                    &crypto::derive(
                        &access::key(self.room(room_id)?, asset.key_epoch)?,
                        b"media",
                    ),
                    &self.blob(room_id, asset_id)?,
                    &dest,
                    &format!("{room_id}:{asset_id}"),
                    &asset.sha256,
                );
                if matches!(result, Err(CoreError::Authentication))
                    && !self.state.damaged_originals.iter().any(|id| id == asset_id)
                {
                    self.state.damaged_originals.push(asset_id.into());
                }
                result?;
                Ok(json!({"path":dest}))
            }
            "tag_assets" => {
                let room_id = text(&c, "room_id")?;
                let person_id = text(&c, "person_id")?;
                let asset_ids: Vec<String> = serde_json::from_value(c["asset_ids"].clone())?;
                if asset_ids.is_empty()
                    || asset_ids.len() > 500
                    || asset_ids.iter().collect::<BTreeSet<_>>().len() != asset_ids.len()
                {
                    return Err(invalid("Select between 1 and 500 distinct memories"));
                }
                let person_key = format!("person:{person_id}");
                let person_value = self
                    .entities(room_id)?
                    .get(&person_key)
                    .cloned()
                    .ok_or_else(|| invalid("Person is not in this Room"))?;
                let person: PersonTag = serde_json::from_value(person_value)?;
                if person.room_id != room_id {
                    return Err(invalid("Person is not in this Room"));
                }

                let known = self.assets(room_id)?;
                let selected = asset_ids
                    .iter()
                    .map(|asset_id| {
                        known
                            .iter()
                            .find(|asset| &asset.id == asset_id)
                            .ok_or_else(|| invalid("Media not found in this Room"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                let remove = c["remove"].as_bool().unwrap_or(false);
                let mut changes = Vec::with_capacity(selected.len());
                for asset in selected {
                    let mut tags = asset.people.iter().cloned().collect::<BTreeSet<_>>();
                    if remove {
                        tags.remove(person_id);
                    } else {
                        tags.insert(person_id.to_owned());
                    }
                    let fields = json!({"people": tags.into_iter().collect::<Vec<_>>()});
                    self.validate_asset_patch(room_id, asset, fields.as_object().unwrap())?;
                    changes.push((asset.id.clone(), fields));
                }
                for (asset_id, fields) in &changes {
                    self.write_op(room_id, "asset_patch", asset_id, fields.clone())?;
                }
                Ok(json!({"saved": true, "updated": changes.len()}))
            }
            "patch_asset" => {
                let room_id = text(&c, "room_id")?;
                let asset_id = text(&c, "asset_id")?;
                let fields = c["fields"]
                    .as_object()
                    .ok_or_else(|| invalid("Missing fields"))?;
                let asset = self
                    .assets(room_id)?
                    .into_iter()
                    .find(|asset| asset.id == asset_id)
                    .ok_or_else(|| invalid("Media not found"))?;
                self.validate_asset_patch(room_id, &asset, fields)?;
                self.write_op(room_id, "asset_patch", asset_id, c["fields"].clone())?;
                Ok(json!({"saved": true}))
            }
            "patch_assets" => {
                let room_id = text(&c, "room_id")?;
                let asset_ids: Vec<String> = serde_json::from_value(c["asset_ids"].clone())?;
                if asset_ids.is_empty()
                    || asset_ids.len() > 500
                    || asset_ids.iter().collect::<BTreeSet<_>>().len() != asset_ids.len()
                {
                    return Err(invalid("Select between 1 and 500 distinct memories"));
                }
                let fields = c["fields"]
                    .as_object()
                    .ok_or_else(|| invalid("Missing fields"))?;
                if fields.is_empty() {
                    return Err(invalid("Media edit has no fields"));
                }
                let known = self.assets(room_id)?;
                let selected = asset_ids
                    .iter()
                    .map(|asset_id| {
                        known
                            .iter()
                            .find(|asset| &asset.id == asset_id)
                            .ok_or_else(|| invalid("Media not found in this Room"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                for asset in &selected {
                    self.validate_asset_patch(room_id, asset, fields)?;
                }
                for asset_id in &asset_ids {
                    self.write_op(room_id, "asset_patch", asset_id, c["fields"].clone())?;
                }
                Ok(json!({"saved": true, "updated": asset_ids.len()}))
            }
            "patch_asset_dates" => {
                let room_id = text(&c, "room_id")?;
                let updates = c["dates"]
                    .as_array()
                    .ok_or_else(|| invalid("Missing corrected capture dates"))?;
                if updates.is_empty() || updates.len() > 500 {
                    return Err(invalid("Correct between 1 and 500 capture dates"));
                }
                let known = self.assets(room_id)?;
                let mut seen = BTreeSet::new();
                let mut changes = Vec::with_capacity(updates.len());
                for update in updates {
                    let asset_id = text(update, "asset_id")?;
                    if !seen.insert(asset_id.to_owned()) {
                        return Err(invalid("A memory appears more than once"));
                    }
                    let captured_at = update["captured_at"]
                        .as_i64()
                        .ok_or_else(|| invalid("A corrected capture date is invalid"))?;
                    let asset = known
                        .iter()
                        .find(|asset| asset.id == asset_id && !asset.deleted)
                        .ok_or_else(|| invalid("Media not found in this Room"))?;
                    let fields = json!({"captured_at":captured_at});
                    self.validate_asset_patch(room_id, asset, fields.as_object().unwrap())?;
                    changes.push((asset.id.clone(), fields));
                }
                for (asset_id, fields) in &changes {
                    self.write_op(room_id, "asset_patch", asset_id, fields.clone())?;
                }
                Ok(json!({"saved":true,"updated":changes.len()}))
            }
            "save_album" => {
                let room_id = text(&c, "room_id")?;
                let album_id = c["id"].as_str().map(str::to_owned).unwrap_or_else(id);
                let asset_ids: Vec<String> = serde_json::from_value(c["asset_ids"].clone())?;
                if asset_ids.len() > 5_000
                    || asset_ids.iter().collect::<BTreeSet<_>>().len() != asset_ids.len()
                {
                    return Err(invalid("An album needs at most 5,000 distinct memories"));
                }
                let known = self.assets(room_id)?;
                if asset_ids.iter().any(|a| !known.iter().any(|x| &x.id == a)) {
                    return Err(invalid("Album contains media from another Room"));
                }
                let previous = self
                    .entities(room_id)?
                    .get(&format!("album:{album_id}"))
                    .map(|value| serde_json::from_value::<Album>(value.clone()))
                    .transpose()?;
                let previous_ids = previous
                    .as_ref()
                    .map(|album| album.asset_ids.as_slice())
                    .unwrap_or(&[]);
                let changed: Vec<_> = previous_ids
                    .iter()
                    .chain(asset_ids.iter())
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .filter(|id| previous_ids.contains(id) != asset_ids.contains(id))
                    .collect();
                let caption = match c.get("caption") {
                    Some(value) => value
                        .as_str()
                        .ok_or_else(|| invalid("Invalid album caption"))?
                        .trim()
                        .to_owned(),
                    None => previous
                        .as_ref()
                        .map(|album| album.caption.clone())
                        .unwrap_or_default(),
                };
                if caption.len() > 2_000 {
                    return Err(invalid("Album caption is too long"));
                }
                let cover_asset_id = match c.get("cover_asset_id") {
                    Some(Value::Null) => None,
                    Some(value) => Some(
                        value
                            .as_str()
                            .ok_or_else(|| invalid("Invalid album cover"))?
                            .to_owned(),
                    ),
                    None => previous
                        .as_ref()
                        .and_then(|album| album.cover_asset_id.clone()),
                };
                let cover_asset_id = cover_asset_id.filter(|cover| asset_ids.contains(cover));
                if c.get("cover_asset_id")
                    .is_some_and(|value| !value.is_null())
                    && cover_asset_id.is_none()
                {
                    return Err(invalid("Choose a cover from this album"));
                }
                let album = Album {
                    id: album_id.clone(),
                    room_id: room_id.into(),
                    title: title(text(&c, "title")?)?,
                    caption,
                    asset_ids: asset_ids.clone(),
                    cover_asset_id,
                    pinned: c
                        .get("pinned")
                        .map(|value| value.as_bool().ok_or_else(|| invalid("Invalid pin choice")))
                        .transpose()?
                        .unwrap_or_else(|| previous.as_ref().is_some_and(|album| album.pinned)),
                };
                self.write_op(room_id, "album", &album_id, serde_json::to_value(album)?)?;
                for asset_id in changed {
                    let entity_id = format!("{album_id}-{asset_id}");
                    self.write_op(room_id, "album_member", &entity_id,
                        json!({"album_id":album_id,"asset_id":asset_id,"included":asset_ids.contains(&asset_id)}))?;
                }
                Ok(json!({"id":album_id}))
            }
            "save_playlist" => {
                let room_id = text(&c, "room_id")?;
                self.require_story_editor(room_id)?;
                let playlist_id = c["id"].as_str().map(str::to_owned).unwrap_or_else(id);
                safe_id(&playlist_id)?;
                let story_ids: Vec<String> = serde_json::from_value(c["story_ids"].clone())?;
                if story_ids.len() > 500
                    || story_ids.iter().collect::<BTreeSet<_>>().len() != story_ids.len()
                {
                    return Err(invalid("A playlist needs at most 500 distinct films"));
                }
                let room = self.room(room_id)?;
                if story_ids.iter().any(|story_id| {
                    room.operations.iter().all(|op| {
                        op.kind != "story"
                            || op.value["id"] != story_id.as_str()
                            || !op.value["published_asset"].is_string()
                    })
                }) {
                    return Err(invalid(
                        "A playlist can contain only published films in this Room",
                    ));
                }
                let playlist = FilmPlaylist {
                    id: playlist_id.clone(),
                    room_id: room_id.into(),
                    title: title(text(&c, "title")?)?,
                    story_ids,
                    deleted: false,
                };
                self.write_op(
                    room_id,
                    "playlist",
                    &playlist_id,
                    serde_json::to_value(playlist)?,
                )?;
                Ok(json!({"id":playlist_id}))
            }
            "delete_playlist" => {
                let room_id = text(&c, "room_id")?;
                self.require_story_editor(room_id)?;
                let playlist_id = text(&c, "id")?;
                let value = self
                    .entities(room_id)?
                    .get(&format!("playlist:{playlist_id}"))
                    .cloned()
                    .ok_or_else(|| invalid("Playlist not found"))?;
                let mut playlist: FilmPlaylist = serde_json::from_value(value)?;
                if playlist.deleted {
                    return Err(invalid("Playlist already deleted"));
                }
                playlist.deleted = true;
                self.write_op(
                    room_id,
                    "playlist",
                    playlist_id,
                    serde_json::to_value(playlist)?,
                )?;
                Ok(json!({"deleted":true}))
            }
            "save_moment" => {
                let room_id = text(&c, "room_id")?;
                let asset_ids: Vec<String> = serde_json::from_value(c["asset_ids"].clone())?;
                let known = self.assets(room_id)?;
                if asset_ids.is_empty()
                    || asset_ids
                        .iter()
                        .any(|id| !known.iter().any(|a| a.id == *id && !a.deleted))
                {
                    return Err(invalid("Choose memories from this Room"));
                }
                let dates = known
                    .iter()
                    .filter(|a| asset_ids.contains(&a.id))
                    .map(|a| a.captured_at)
                    .collect::<Vec<_>>();
                let moment = Moment {
                    id: c["id"].as_str().map(str::to_owned).unwrap_or_else(id),
                    room_id: room_id.into(),
                    title: title(text(&c, "title")?)?,
                    start: *dates.iter().min().ok_or_else(|| invalid("Missing date"))?,
                    end: *dates.iter().max().ok_or_else(|| invalid("Missing date"))?,
                    asset_ids,
                    featured: c["featured"].as_bool().unwrap_or(false),
                };
                self.write_op(
                    room_id,
                    "moment",
                    &moment.id,
                    serde_json::to_value(&moment)?,
                )?;
                Ok(json!({"id":moment.id}))
            }
            "export_library" => {
                self.export_library(text(&c, "room_id")?, Path::new(text(&c, "destination")?))
            }
            "export_preserved" => {
                self.export_preserved(text(&c, "room_id")?, Path::new(text(&c, "path")?))
            }
            "save_person" => {
                let room_id = text(&c, "room_id")?;
                let person_id = c["id"].as_str().map(str::to_owned).unwrap_or_else(id);
                let person = PersonTag {
                    id: person_id.clone(),
                    room_id: room_id.into(),
                    name: title(text(&c, "name")?)?,
                };
                self.write_op(room_id, "person", &person_id, serde_json::to_value(person)?)?;
                Ok(json!({"id":person_id}))
            }
            "comment" => {
                let room_id = text(&c, "room_id")?;
                let asset_id = text(&c, "asset_id")?;
                if !self.assets(room_id)?.iter().any(|a| a.id == asset_id) {
                    return Err(invalid("Media not found"));
                }
                let body = text(&c, "body")?.trim();
                if body.is_empty() || body.len() > 10000 {
                    return Err(invalid("Write a comment under 10,000 bytes"));
                }
                let comment_id = id();
                self.write_op(room_id,"comment",&comment_id,json!({"id":comment_id,"asset_id":asset_id,"author":self.device()?,"body":body,"created_at":now()}))?;
                Ok(json!({"id":comment_id}))
            }
            "set_like" => {
                let room_id = text(&c, "room_id")?;
                let asset_id = text(&c, "asset_id")?;
                if !self
                    .assets(room_id)?
                    .iter()
                    .any(|a| a.id == asset_id && !a.deleted)
                {
                    return Err(invalid("Memory not found"));
                }
                let liked = c["liked"]
                    .as_bool()
                    .ok_or_else(|| invalid("Choose liked or unliked"))?;
                let author = self.device()?;
                if !access::allowed_device(self.room(room_id)?, &author) {
                    return Err(CoreError::AccessDenied);
                }
                let reaction_id = format!("like-{asset_id}-{author}");
                if self
                    .entities(room_id)?
                    .get(&format!("reaction:{reaction_id}"))
                    .is_some_and(|reaction| reaction["liked"] == liked)
                {
                    return Ok(json!({"id":reaction_id,"liked":liked}));
                }
                self.write_op(
                    room_id,
                    "reaction",
                    &reaction_id,
                    json!({"id":reaction_id,"asset_id":asset_id,"author":author,"liked":liked}),
                )?;
                Ok(json!({"id":reaction_id,"liked":liked}))
            }
            "save_story" => self.save_story(&c),
            "skip_auto_reel" => self.skip_auto_reel(&c),
            "create_story" => {
                let room_id = text(&c, "room_id")?;
                self.require_story_editor(room_id)?;
                let story = StoryProject {
                    format_version: crate::timeline::FORMAT_VERSION,
                    id: id(),
                    room_id: room_id.into(),
                    title: title(c["title"].as_str().unwrap_or("Our film"))?,
                    version_id: id(),
                    editor: self.device()?,
                    width: 1920,
                    height: 1080,
                    clips: vec![],
                    chapters: vec![],
                    published_asset: None,
                    created_at: now(),
                    auto_generated: false,
                    archived: false,
                };
                validate_story(&story)?;
                self.state.drafts.push(story.clone());
                let mut value = serde_json::to_value(&story)?;
                value["private_draft"] = json!(true);
                Ok(json!({"project":value}))
            }
            "split_story_clip" => {
                let mut story: StoryProject = serde_json::from_value(c["project"].clone())?;
                validate_story(&story)?;
                self.require_story_editor(&story.room_id)?;
                let known = self.assets(&story.room_id)?;
                if story
                    .clips
                    .iter()
                    .any(|clip| !known.iter().any(|a| a.id == clip.asset_id))
                {
                    return Err(invalid("A film clip is not in this Room"));
                }
                let source_time = c["source_time"]
                    .as_f64()
                    .ok_or_else(|| invalid("Choose a split time"))?;
                crate::timeline::split(&mut story, text(&c, "clip_id")?, source_time)?;
                validate_story(&story)?;
                let mut value = serde_json::to_value(story)?;
                value["private_draft"] = c["project"]["private_draft"].clone();
                Ok(json!({"project":value}))
            }
            "asset_history" => {
                let room = self.room(text(&c, "room_id")?)?;
                let asset_id = text(&c, "asset_id")?;
                let mut history = room
                    .operations
                    .iter()
                    .filter(|operation| {
                        operation.entity_id == asset_id
                            && operation.kind == "asset_patch"
                            && operation.value.get("caption").is_some()
                    })
                    .map(|operation| {
                        json!({
                            "id": operation.id,
                            "author": operation.author,
                            "caption": operation.value["caption"],
                            "clock": operation.clock
                        })
                    })
                    .collect::<Vec<_>>();
                history.sort_by(|left, right| {
                    left["clock"]
                        .as_u64()
                        .cmp(&right["clock"].as_u64())
                        .then_with(|| left["author"].as_str().cmp(&right["author"].as_str()))
                        .then_with(|| left["id"].as_str().cmp(&right["id"].as_str()))
                });
                Ok(json!({"history":history}))
            }
            "invite" => self.issue_invitation(&c),
            "preview_invitation" => self.preview_invitation(&c),
            "stage_invitation" => self.stage_invitation(&c),
            "accept_invitation" => self.accept_invitation(&c),
            "decline_invitation" => self.decline_invitation(&c),
            "join" => self.join_invitation(text(&c, "invitation")?),
            "export_room" => {
                self.export_room(text(&c, "room_id")?, Path::new(text(&c, "path")?))?;
                Ok(json!({"exported":true}))
            }
            "import_room" => {
                self.import_room(text(&c, "room_id")?, Path::new(text(&c, "path")?))?;
                Ok(json!({"imported":true}))
            }
            "backup" => {
                self.backup(Path::new(text(&c, "path")?))?;
                Ok(json!({"exported":true}))
            }
            "restore" => {
                self.restore(
                    Path::new(text(&c, "path")?),
                    &crypto::parse_key(text(&c, "recovery_key")?)?,
                )?;
                Ok(json!({"restored":true}))
            }
            "add_source" => {
                let room_id = text(&c, "room_id")?;
                self.room(room_id)?;
                let kind = text(&c, "kind")?;
                if ![
                    "local",
                    "webdav",
                    "google_drive",
                    "onedrive",
                    "gateway",
                    "s3",
                ]
                .contains(&kind)
                {
                    return Err(invalid("This provider is not available yet"));
                }
                let room = self.room(room_id)?;
                let token = if kind == "gateway" {
                    serde_json::to_string(
                        &json!({"owner_id":room.owner_id,"grant":room.grant,"signing_seed":self.state.signing_seed,"access_revision":access::revision(room)}),
                    )?
                } else {
                    c["token"].as_str().unwrap_or("").into()
                };
                let source = StorageSource {
                    id: id(),
                    room_id: room_id.into(),
                    name: title(text(&c, "name")?)?,
                    kind: kind.into(),
                    endpoint: text(&c, "endpoint")?.into(),
                    token,
                    paused: false,
                    preferred: !self.state.sources.iter().any(|s| s.room_id == room_id),
                    overflow: false,
                    network_policy: "any".into(),
                    budget: c["budget"].as_u64(),
                };
                crate::providers::validate(&source)?;
                if kind == "gateway" && room.grant.role == Role::Owner {
                    crate::providers::register_gateway(&source, &room.access_changes)?;
                }
                let source_id = source.id.clone();
                self.state.sources.push(source);
                Ok(json!({"id":source_id}))
            }
            "pause_source" => {
                let source = self
                    .state
                    .sources
                    .iter_mut()
                    .find(|s| Some(s.id.as_str()) == c["id"].as_str())
                    .ok_or_else(|| invalid("Source not found"))?;
                source.paused = c["paused"].as_bool().unwrap_or(true);
                Ok(json!({"saved":true}))
            }
            "set_source_network_policy" => {
                let policy = text(&c, "policy")?;
                if !["any", "unmetered_only"].contains(&policy) {
                    return Err(invalid("Unknown network policy"));
                }
                let source = self
                    .state
                    .sources
                    .iter_mut()
                    .find(|s| Some(s.id.as_str()) == c["id"].as_str())
                    .ok_or_else(|| invalid("Source not found"))?;
                source.network_policy = policy.into();
                Ok(json!({"saved":true}))
            }
            "prefer_source" => {
                let selected = self.source(text(&c, "id")?)?;
                for source in &mut self.state.sources {
                    if source.room_id == selected.room_id {
                        source.preferred = source.id == selected.id;
                        if source.preferred {
                            source.overflow = false;
                        }
                    }
                }
                Ok(json!({"saved":true}))
            }
            "set_overflow" => {
                let room_id = text(&c, "room_id")?;
                self.room(room_id)?;
                let selected = c["id"].as_str();
                if let Some(id) = selected {
                    let source = self.source(id)?;
                    if source.room_id != room_id || self.is_preferred(&source) {
                        return Err(invalid("Overflow must be a different source in this Room"));
                    }
                } else if !c["id"].is_null() {
                    return Err(invalid("Invalid overflow source"));
                }
                for source in &mut self.state.sources {
                    if source.room_id == room_id {
                        source.overflow = selected == Some(source.id.as_str());
                    }
                }
                Ok(json!({"saved":true}))
            }
            "quota" => {
                let s = self.source(text(&c, "id")?)?;
                crate::providers::quota(&s)
            }
            "replicate" => self.replicate(text(&c, "source_id")?, text(&c, "asset_id")?),
            "replicate_step" => {
                self.replicate_mode(text(&c, "source_id")?, text(&c, "asset_id")?, true)
            }
            "retry_transfer" => {
                let job = self
                    .state
                    .transfers
                    .iter()
                    .find(|j| Some(j.id.as_str()) == c["id"].as_str())
                    .cloned()
                    .ok_or_else(|| invalid("Transfer not found"))?;
                self.replicate(&job.source_id, &job.asset_id)
            }
            "cancel_transfer" => {
                let position = self
                    .state
                    .transfers
                    .iter()
                    .position(|j| Some(j.id.as_str()) == c["id"].as_str())
                    .ok_or_else(|| invalid("Transfer not found"))?;
                let job = self.state.transfers[position].clone();
                if job.state == "complete" {
                    return Err(invalid("This transfer is already complete"));
                }
                if let Some(source) = self.state.sources.iter().find(|s| s.id == job.source_id) {
                    if source.kind == "s3" {
                        if let Some(session) = &job.session {
                            let name = format!("{}-{}.frblob", job.room_id, job.asset_id);
                            match crate::s3::abort(source, &name, session) {
                                Ok(()) => {
                                    self.state.transfers[position].session = None;
                                    self.state.transfers[position].transferred = 0;
                                    self.state.transfers[position].error = None;
                                },
                                Err(_) => self.state.transfers[position].error = Some(
                                    "Cancelled locally; reconnect and cancel again to release unfinished S3 parts".into()),
                            }
                        }
                    }
                }
                self.state.transfers[position].state = "cancelled".into();
                Ok(json!({"saved":true}))
            }
            "sync_source" => self.sync_source(text(&c, "id")?),
            "refresh_access" => Ok(json!({"active":self.refresh_access(text(&c,"id")?)?})),
            "sync_all" => {
                let room_id = text(&c, "room_id")?;
                let mut sources = self
                    .state
                    .sources
                    .iter()
                    .filter(|s| s.room_id == room_id && !s.paused)
                    .cloned()
                    .collect::<Vec<_>>();
                // Update other accessible sources before choosing where to upload.
                sources.sort_by_key(|source| self.is_preferred(source));
                let mut errors = vec![];
                let mut synced_sources = HashSet::new();
                for source in sources {
                    let preferred = self.is_preferred(&source);
                    if self.network_state == "offline" && source.kind != "local" {
                        errors.push("Waiting for a network".into());
                        let can_contribute = self.room(room_id)?.grant.role != Role::Viewer
                            && access::allowed_device(self.room(room_id)?, &self.device()?);
                        if can_contribute && preferred {
                            for asset in self.assets(room_id)?.into_iter().filter(|a| !a.deleted) {
                                if self.blob(room_id, &asset.id)?.is_file()
                                    && !self.state.replicas.iter().any(|replica| {
                                        replica.asset_id == asset.id
                                            && replica.source_id == source.id
                                            && replica.verified
                                    })
                                    && !self.state.transfers.iter().any(|job| {
                                        job.asset_id == asset.id
                                            && job.source_id == source.id
                                            && ["complete", "cancelled"]
                                                .contains(&job.state.as_str())
                                    })
                                {
                                    self.replicate(&source.id, &asset.id)?;
                                }
                            }
                        }
                        continue;
                    }
                    let preferred_catalog_full = match self.sync_source(&source.id) {
                        Ok(_) => {
                            synced_sources.insert(source.id.clone());
                            false
                        }
                        Err(e) => {
                            let full =
                                preferred && e.to_string().contains("Upload paused: storage full");
                            errors.push(e.to_string());
                            if !full {
                                continue;
                            }
                            true
                        }
                    };
                    // The sync may have learned a removal or a viewer downgrade.
                    let can_contribute = self.room(room_id)?.grant.role != Role::Viewer
                        && access::allowed_device(self.room(room_id)?, &self.device()?);
                    if can_contribute && preferred {
                        let overflow = self
                            .state
                            .sources
                            .iter()
                            .find(|candidate| {
                                candidate.room_id == room_id
                                    && candidate.overflow
                                    && !candidate.paused
                                    && synced_sources.contains(&candidate.id)
                            })
                            .cloned();
                        for asset in self.assets(room_id)?.into_iter().filter(|a| !a.deleted) {
                            if !self.blob(room_id, &asset.id)?.is_file()
                                || self.state.transfers.iter().any(|j| {
                                    j.source_id == source.id
                                        && j.asset_id == asset.id
                                        && ["complete", "cancelled"].contains(&j.state.as_str())
                                })
                                || overflow.as_ref().is_some_and(|candidate| {
                                    self.state.replicas.iter().any(|replica| {
                                        replica.asset_id == asset.id
                                            && replica.source_id == candidate.id
                                            && replica.verified
                                    })
                                })
                            {
                                continue;
                            }
                            if preferred_catalog_full {
                                if let Some(candidate) = &overflow {
                                    if let Err(e) = self.replicate(&candidate.id, &asset.id) {
                                        errors.push(e.to_string());
                                    }
                                }
                                continue;
                            }
                            match self.replicate(&source.id, &asset.id) {
                                Ok(job)
                                    if job["state"] == "waiting"
                                        && job["error"] == "Upload paused: storage full" =>
                                {
                                    if let Some(candidate) = &overflow {
                                        if let Err(e) = self.replicate(&candidate.id, &asset.id) {
                                            errors.push(e.to_string());
                                        }
                                    }
                                }
                                Ok(_) => {}
                                Err(e) => errors.push(e.to_string()),
                            }
                        }
                    }
                }
                let cache_cleanup = self.enforce_device_cache_limit(room_id)?;
                Ok(json!({"errors":errors,"cache_cleanup":cache_cleanup}))
            }
            "fetch" => self.fetch(text(&c, "room_id")?, text(&c, "asset_id")?),
            "pin_original" => self.pin_original(text(&c, "room_id")?, text(&c, "asset_id")?),
            "unpin_original" => self.unpin_original(text(&c, "room_id")?, text(&c, "asset_id")?),
            "evict_original" => self.evict_original(text(&c, "room_id")?, text(&c, "asset_id")?),
            "free_device_storage" => self.free_device_storage(
                text(&c, "room_id")?,
                c["bytes_to_free"]
                    .as_u64()
                    .ok_or_else(|| invalid("Choose a non-negative amount of device storage"))?,
            ),
            "set_device_cache_limit" => {
                let max_bytes = if c["max_bytes"].is_null() {
                    None
                } else {
                    Some(c["max_bytes"].as_u64().ok_or_else(|| {
                        invalid("Choose a valid local cache limit or clear the limit")
                    })?)
                };
                self.set_device_cache_limit(text(&c, "room_id")?, max_bytes)
            }
            "enforce_device_cache_limit" => self.enforce_device_cache_limit(text(&c, "room_id")?),
            "watch_folder" => {
                let room_id = text(&c, "room_id")?;
                if self.room(room_id)?.grant.role == Role::Viewer
                    || !access::allowed_device(self.room(room_id)?, &self.device()?)
                {
                    return Err(CoreError::AccessDenied);
                }
                let path = PathBuf::from(text(&c, "path")?);
                if !path.is_dir() {
                    return Err(invalid("Choose an existing folder"));
                }
                let seen = if c["include_existing"].as_bool().unwrap_or(false) {
                    vec![]
                } else {
                    self.folder_files(&path)?
                        .into_iter()
                        .map(|p| self.fingerprint(&p))
                        .collect::<Result<Vec<_>>>()?
                };
                let rule = WatchRule {
                    id: id(),
                    room_id: room_id.into(),
                    path: path.to_string_lossy().into(),
                    paused: false,
                    excluded_extensions: exclusions(c.get("excluded_extensions"))?,
                    seen,
                };
                let watch_id = rule.id.clone();
                self.state.watches.push(rule);
                Ok(json!({"id":watch_id}))
            }
            "watch_content_tree" => {
                let room_id = text(&c, "room_id")?;
                if self.room(room_id)?.grant.role == Role::Viewer
                    || !access::allowed_device(self.room(room_id)?, &self.device()?)
                {
                    return Err(CoreError::AccessDenied);
                }
                let tree_uri = text(&c, "tree_uri")?.trim();
                let has_authority = tree_uri
                    .strip_prefix("content://")
                    .and_then(|path| path.split('/').next())
                    .is_some_and(|authority| !authority.is_empty());
                if !has_authority || tree_uri.len() > 8_192 {
                    return Err(invalid("Choose an Android shared folder"));
                }
                let include_existing = c["include_existing"].as_bool().unwrap_or(false);
                let mut seen = Vec::new();
                if !include_existing {
                    let identifiers = c["initial_seen"]
                        .as_array()
                        .ok_or_else(|| invalid("Missing folder snapshot"))?;
                    if identifiers.len() > 20_000 {
                        return Err(invalid(
                            "This folder has too many existing items to skip individually. Choose a smaller folder or import existing media.",
                        ));
                    }
                    for identifier in identifiers {
                        let identifier = identifier
                            .as_str()
                            .filter(|value| value.starts_with("content:") && value.len() <= 8_192)
                            .ok_or_else(|| invalid("Invalid Android folder item identifier"))?;
                        if !seen.iter().any(|existing| existing == identifier) {
                            seen.push(identifier.to_owned());
                        }
                    }
                }
                let excluded_extensions = exclusions(c.get("excluded_extensions"))?;
                if let Some(existing) = self
                    .state
                    .watches
                    .iter_mut()
                    .find(|watch| watch.room_id == room_id && watch.path == tree_uri)
                {
                    existing.paused = false;
                    existing.excluded_extensions = excluded_extensions;
                    if include_existing {
                        existing.seen.clear();
                    }
                    return Ok(json!({"id":existing.id,"updated":true}));
                }
                let rule = WatchRule {
                    id: id(),
                    room_id: room_id.into(),
                    path: tree_uri.into(),
                    paused: false,
                    excluded_extensions,
                    seen,
                };
                let watch_id = rule.id.clone();
                self.state.watches.push(rule);
                Ok(json!({"id":watch_id,"updated":false}))
            }
            "pause_watch" => {
                let index = self
                    .state
                    .watches
                    .iter()
                    .position(|watch| Some(watch.id.as_str()) == c["id"].as_str())
                    .ok_or_else(|| invalid("Upload rule not found"))?;
                let room_id = self.state.watches[index].room_id.clone();
                if self.room(&room_id)?.grant.role == Role::Viewer
                    || !access::allowed_device(self.room(&room_id)?, &self.device()?)
                {
                    return Err(CoreError::AccessDenied);
                }
                self.state.watches[index].paused = c["paused"].as_bool().unwrap_or(true);
                Ok(json!({"saved":true}))
            }
            "set_watch_exclusions" => {
                let excluded = exclusions(c.get("excluded_extensions"))?;
                let index = self
                    .state
                    .watches
                    .iter()
                    .position(|watch| Some(watch.id.as_str()) == c["id"].as_str())
                    .ok_or_else(|| invalid("Upload rule not found"))?;
                let room_id = self.state.watches[index].room_id.clone();
                if self.room(&room_id)?.grant.role == Role::Viewer
                    || !access::allowed_device(self.room(&room_id)?, &self.device()?)
                {
                    return Err(CoreError::AccessDenied);
                }
                self.state.watches[index].excluded_extensions = excluded;
                Ok(json!({"saved":true}))
            }
            "remove_watch" => {
                let index = self
                    .state
                    .watches
                    .iter()
                    .position(|watch| Some(watch.id.as_str()) == c["id"].as_str())
                    .ok_or_else(|| invalid("Upload rule not found"))?;
                let room_id = self.state.watches[index].room_id.clone();
                if self.room(&room_id)?.grant.role == Role::Viewer
                    || !access::allowed_device(self.room(&room_id)?, &self.device()?)
                {
                    return Err(CoreError::AccessDenied);
                }
                self.state.watches.remove(index);
                Ok(json!({"removed":true}))
            }
            "acknowledge_watch_item" => {
                let watch_id = text(&c, "id")?;
                let identifier = text(&c, "identifier")?;
                let index = self
                    .state
                    .watches
                    .iter()
                    .position(|watch| watch.id == watch_id)
                    .ok_or_else(|| invalid("Upload rule not found"))?;
                let room_id = self.state.watches[index].room_id.clone();
                if !self.state.watches[index].path.starts_with("content://")
                    || !identifier.starts_with("content:")
                    || identifier.len() > 8_192
                {
                    return Err(invalid("Invalid Android folder item identifier"));
                }
                if self.room(&room_id)?.grant.role == Role::Viewer
                    || !access::allowed_device(self.room(&room_id)?, &self.device()?)
                {
                    return Err(CoreError::AccessDenied);
                }
                if !self.state.watches[index]
                    .seen
                    .iter()
                    .any(|seen| seen == identifier)
                {
                    self.state.watches[index].seen.push(identifier.into());
                }
                Ok(json!({"saved":true}))
            }
            "watch_photos" => {
                let room_id = text(&c, "room_id")?;
                if self.room(room_id)?.grant.role == Role::Viewer
                    || !access::allowed_device(self.room(room_id)?, &self.device()?)
                {
                    return Err(CoreError::AccessDenied);
                }
                self.state.watches.retain(|w| w.path != "photos://library");
                let rule = WatchRule {
                    id: id(),
                    room_id: room_id.into(),
                    path: "photos://library".into(),
                    paused: false,
                    excluded_extensions: exclusions(c.get("excluded_extensions"))?,
                    seen: serde_json::from_value(c["seen"].clone())?,
                };
                let watch_id = rule.id.clone();
                self.state.watches.push(rule);
                Ok(json!({"id":watch_id}))
            }
            "import_photo" => self.import_photo(&c),
            "acknowledge_photo" => {
                let rule = self
                    .state
                    .watches
                    .iter_mut()
                    .find(|w| {
                        Some(w.id.as_str()) == c["id"].as_str() && w.path == "photos://library"
                    })
                    .ok_or_else(|| invalid("Photo upload rule not found"))?;
                let identifier = text(&c, "identifier")?;
                if !rule.seen.iter().any(|s| s == identifier) {
                    rule.seen.push(identifier.into());
                }
                Ok(json!({"saved":true}))
            }
            "scan_watches" => self.scan_watches(),
            _ => Err(invalid("Unknown command")),
        }
    }

    fn members(&self, room_id: &str) -> Result<BTreeMap<String, Grant>> {
        let room = self.room(room_id)?;
        if let Some(policy) = room.access_changes.last() {
            return Ok(policy.members.clone());
        }
        let mut members = BTreeMap::new();
        let mut ops: Vec<_> = room.operations.iter().collect();
        ops.sort_by_key(|o| o.clock);
        for op in ops {
            members.insert(op.author.clone(), op.grant.clone());
        }
        members.extend(room.issued_grants.clone());
        members.insert(room.grant.device_id.clone(), room.grant.clone());
        Ok(members)
    }

    fn prepare_owner_transfer(&self, c: &Value) -> Result<Value> {
        let room_id = text(c, "room_id")?;
        let next_owner_id = text(c, "device_id")?;
        let seed = self.state.signing_seed.clone();
        let offer = access::make_transfer_offer(
            self.room(room_id)?,
            self.members(room_id)?,
            next_owner_id,
            &seed,
        )?;
        Ok(json!({
            "offer": URL_SAFE_NO_PAD.encode(serde_json::to_vec(&offer)?),
            "next_owner_id": next_owner_id,
            "revision": offer.revision,
        }))
    }

    fn leave_room(&mut self, c: &Value) -> Result<Value> {
        let room_id = text(c, "room_id")?;
        let device_id = self.device()?;
        let room = self.room(room_id)?;
        if room.owner_id == device_id {
            return Err(invalid("Transfer ownership before leaving this Room"));
        }
        let asset_ids: BTreeSet<_> = self
            .assets(room_id)?
            .into_iter()
            .map(|asset| asset.id)
            .collect();

        // Disconnect this device's Room-scoped jobs and credentials before
        // forgetting the only local copy of the Room keys.
        for source in self
            .state
            .sources
            .iter_mut()
            .filter(|source| source.room_id == room_id)
        {
            source.token.zeroize();
        }
        self.state
            .sources
            .retain(|source| source.room_id != room_id);
        self.state.watches.retain(|watch| watch.room_id != room_id);
        self.state.transfers.retain(|job| job.room_id != room_id);
        self.state
            .replicas
            .retain(|replica| !asset_ids.contains(&replica.asset_id));
        self.state
            .damaged_originals
            .retain(|asset_id| !asset_ids.contains(asset_id));
        self.state.offline_pins.retain(|pin| pin.room_id != room_id);
        self.state
            .device_cache_limits
            .retain(|limit| limit.room_id != room_id);
        self.state
            .video_indexes
            .retain(|index| index.room_id != room_id);
        self.state
            .photo_fingerprints
            .retain(|fingerprint| fingerprint.room_id != room_id);
        self.state
            .album_shares
            .retain(|share| share.room_id != room_id);
        if let Some(room) = self.state.rooms.iter_mut().find(|room| room.id == room_id) {
            room.secret.zeroize();
            for secret in room.epoch_secrets.values_mut() {
                secret.zeroize();
            }
        }
        self.state.rooms.retain(|room| room.id != room_id);
        self.video_search_cache = None;

        Ok(json!({
            "room_id": room_id,
            "local_access_removed": true,
            "shared_roster_changed": false,
        }))
    }

    fn accept_owner_transfer(&mut self, c: &Value) -> Result<Value> {
        let room_id = text(c, "room_id")?;
        let encoded = text(c, "offer")?.trim();
        if encoded.len() > 512 * 1024 {
            return Err(invalid("Ownership transfer offer is too large"));
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| invalid("Invalid ownership transfer offer"))?;
        let offer: OwnershipTransfer = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("Invalid ownership transfer offer"))?;
        let room = self.room(room_id)?.clone();
        let seed = self.state.signing_seed.clone();
        let change = access::build_transfer(&room, offer, &seed)
            .map_err(|_| invalid("This offer is expired or does not match this Room"))?;
        let mut changes = room.access_changes;
        changes.push(change);
        access::apply(self.room_mut(room_id)?, changes.clone(), &seed)?;
        let members = self.members(room_id)?;
        self.room_mut(room_id)?.issued_grants = members;
        Ok(json!({
            "owner_id": self.room(room_id)?.owner_id,
            "revision": access::revision(self.room(room_id)?),
            "key_epoch": access::epoch(self.room(room_id)?),
            "changes": changes,
        }))
    }

    fn change_access(&mut self, c: &Value, rotate: bool) -> Result<Value> {
        let room_id = text(c, "room_id")?;
        if self.device()? != self.room(room_id)?.owner_id {
            return Err(CoreError::AccessDenied);
        }
        let mut members = self.members(room_id)?;
        match text(c, "action")? {
            "revoke_device" => {
                let device = text(c, "device_id")?;
                if device == self.room(room_id)?.owner_id {
                    return Err(invalid("Transfer ownership before removing the owner"));
                }
                if members.remove(device).is_none() {
                    return Err(invalid("Member not found"));
                }
            }
            "set_role" => {
                let device = text(c, "device_id")?;
                let role: Role = serde_json::from_value(c["role"].clone())?;
                if device == self.room(room_id)?.owner_id || role == Role::Owner {
                    return Err(invalid("Use ownership transfer to change the owner"));
                }
                if !members.contains_key(device) {
                    return Err(invalid("Member not found"));
                }
                if self.room(room_id)?.album_only && role != Role::Viewer {
                    return Err(invalid("Album-only members have viewer access"));
                }
                members.insert(
                    device.into(),
                    signing::grant(&self.state.signing_seed, room_id, device, role)?,
                );
            }
            "rotate_keys" => {}
            _ => return Err(invalid("Invalid access change")),
        }
        let change = access::build(
            self.room(room_id)?,
            members,
            &self.state.signing_seed,
            rotate,
        )?;
        let mut chain = self.room(room_id)?.access_changes.clone();
        chain.push(change);
        let seed = self.state.signing_seed.clone();
        access::apply(self.room_mut(room_id)?, chain, &seed)?;
        self.room_mut(room_id)?.issued_grants = self.members(room_id)?;
        Ok(
            json!({"revision":access::revision(self.room(room_id)?),"key_epoch":access::epoch(self.room(room_id)?)}),
        )
    }
    fn decode_invitation(&self, encoded: &str) -> Result<Invitation> {
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded.trim())
            .map_err(|_| invalid("Invalid invitation"))?;
        let value: Value = serde_json::from_slice(&bytes)?;
        let invite: Invitation = if value.get("ciphertext").is_some() {
            let mut envelope: InvitationEnvelope = serde_json::from_value(value)?;
            if envelope.version != 2 || envelope.device_id != self.device()? {
                return Err(CoreError::AccessDenied);
            }
            let signature = std::mem::take(&mut envelope.signature);
            signing::verify(
                &envelope.owner_id,
                &serde_json::to_vec(&envelope)?,
                &signature,
            )?;
            let invitation: Invitation = serde_json::from_slice(&access::unseal(
                &self.state.signing_seed,
                &envelope.ciphertext,
            )?)?;
            if invitation.owner_id != envelope.owner_id {
                return Err(CoreError::Authentication);
            }
            invitation
        } else {
            serde_json::from_value(value)?
        };
        if invite.version != VERSION
            || invite.grant.device_id != self.device()?
            || invite.grant.room_id != invite.room_id
        {
            return Err(CoreError::AccessDenied);
        }
        signing::verify_grant(&invite.grant, &invite.owner_id)?;
        crypto::parse_key(&invite.secret)?;
        let root_owner = access::root_owner_id(&invite.access_changes, &invite.owner_id);
        access::verify_chain(
            &invite.access_changes,
            &invite.room_id,
            root_owner,
            &invite.owner_id,
        )?;
        if invite.access_changes.last().is_some_and(|policy| {
            !policy
                .members
                .contains_key(&self.device().unwrap_or_default())
        }) {
            return Err(CoreError::AccessDenied);
        }
        for key in invite.epoch_secrets.values() {
            crypto::parse_key(key)?;
        }
        safe_id(&invite.room_id)?;
        Ok(invite)
    }

    fn preview_invitation(&self, c: &Value) -> Result<Value> {
        let invite = self.decode_invitation(text(c, "invitation")?)?;
        Ok(json!({
            "room_id": invite.room_id,
            "name": invite.name,
            "owner_id": invite.owner_id,
            "role": invite.grant.role,
            "album_only": invite.album_only
        }))
    }

    fn stage_invitation(&mut self, c: &Value) -> Result<Value> {
        const MAX_PENDING_INVITATIONS: usize = 100;
        let encoded = text(c, "invitation")?.trim().to_owned();
        let invite = self.decode_invitation(&encoded)?;
        if self
            .state
            .rooms
            .iter()
            .any(|room| room.id == invite.room_id)
        {
            return Err(invalid("You already belong to this Room"));
        }
        if let Some(existing) = self
            .state
            .pending_invitations
            .iter()
            .find(|pending| pending.invitation == encoded)
        {
            return Ok(json!({"id": existing.id, "staged": true}));
        }
        if self.state.pending_invitations.len() >= MAX_PENDING_INVITATIONS
            && !self
                .state
                .pending_invitations
                .iter()
                .any(|pending| pending.room_id == invite.room_id)
        {
            return Err(invalid("This device already has 100 pending invitations"));
        }
        self.state.pending_invitations.retain_mut(|pending| {
            if pending.room_id == invite.room_id {
                pending.invitation.zeroize();
                false
            } else {
                true
            }
        });
        let pending_id = id();
        self.state.pending_invitations.push(PendingInvitation {
            id: pending_id.clone(),
            invitation: encoded,
            room_id: invite.room_id,
            name: invite.name,
            owner_id: invite.owner_id,
            role: invite.grant.role,
            album_only: invite.album_only,
            created_at: now(),
        });
        Ok(json!({"id": pending_id, "staged": true}))
    }

    fn join_invitation(&mut self, encoded: &str) -> Result<Value> {
        let invite = self.decode_invitation(encoded)?;
        if self
            .state
            .rooms
            .iter()
            .any(|room| room.id == invite.room_id)
        {
            return Err(invalid("You already belong to this Room"));
        }
        let room_id = invite.room_id.clone();
        self.state.rooms.push(Room {
            id: invite.room_id,
            name: invite.name,
            owner_id: invite.owner_id,
            secret: invite.secret,
            grant: invite.grant,
            album_only: invite.album_only,
            issued_grants: BTreeMap::new(),
            access_changes: invite.access_changes,
            epoch_secrets: invite.epoch_secrets,
            rejected_operations: vec![],
            operations: vec![],
        });
        Ok(json!({"id": room_id}))
    }

    fn accept_invitation(&mut self, c: &Value) -> Result<Value> {
        let pending_id = text(c, "id")?;
        let encoded = self
            .state
            .pending_invitations
            .iter()
            .find(|pending| pending.id == pending_id)
            .map(|pending| pending.invitation.clone())
            .ok_or_else(|| invalid("Pending invitation not found"))?;
        let result = self.join_invitation(&encoded)?;
        if let Some(index) = self
            .state
            .pending_invitations
            .iter()
            .position(|pending| pending.id == pending_id)
        {
            let mut accepted = self.state.pending_invitations.remove(index);
            accepted.invitation.zeroize();
        }
        Ok(result)
    }

    fn decline_invitation(&mut self, c: &Value) -> Result<Value> {
        let pending_id = text(c, "id")?;
        let index = self
            .state
            .pending_invitations
            .iter()
            .position(|pending| pending.id == pending_id)
            .ok_or_else(|| invalid("Pending invitation not found"))?;
        let mut declined = self.state.pending_invitations.remove(index);
        declined.invitation.zeroize();
        Ok(json!({"id": pending_id, "declined": true, "notified_owner": false}))
    }

    fn issue_invitation(&mut self, c: &Value) -> Result<Value> {
        let room_id = text(c, "room_id")?;
        let device = text(c, "device_id")?;
        let room = self.room(room_id)?;
        if self.device()? != room.owner_id {
            return Err(CoreError::AccessDenied);
        }
        let role: Role = serde_json::from_value(c["role"].clone())?;
        if role == Role::Owner {
            return Err(invalid("Ownership transfer is not an invitation"));
        }
        if room.album_only && role != Role::Viewer {
            return Err(invalid("Album-only invitations grant viewer access"));
        }
        if room.album_only && !self.members(room_id)?.contains_key(device) {
            let current_operations: BTreeSet<_> =
                room.operations.iter().map(|op| op.id.clone()).collect();
            let snapshot_only = self.state.album_shares.iter().any(|share| {
                share.scope_id == room_id
                    && !share.snapshot_operations.is_empty()
                    && share.snapshot_operations == current_operations
            });
            if !snapshot_only {
                return Err(invalid(
                    "Publish a fresh album for this new viewer from the original Room's album sharing screen",
                ));
            }
        }
        let grant = signing::grant(&self.state.signing_seed, room_id, device, role)?;
        // Validate recipient identity before issuing or storing a grant.
        access::seal(device, b"device verification")?;
        if !room.access_changes.is_empty() {
            let mut members = self.members(room_id)?;
            members.insert(device.into(), grant.clone());
            let change = access::build(room, members, &self.state.signing_seed, false)?;
            let mut chain = room.access_changes.clone();
            chain.push(change);
            let seed = self.state.signing_seed.clone();
            access::apply(self.room_mut(room_id)?, chain, &seed)?;
        }
        self.room_mut(room_id)?
            .issued_grants
            .insert(device.into(), grant.clone());
        let room = self.room(room_id)?;
        let invite = Invitation {
            version: VERSION,
            room_id: room.id.clone(),
            name: room.name.clone(),
            owner_id: room.owner_id.clone(),
            secret: room.secret.clone(),
            grant,
            album_only: room.album_only,
            access_changes: room.access_changes.clone(),
            epoch_secrets: room.epoch_secrets.clone(),
        };
        let plain = zeroize::Zeroizing::new(serde_json::to_vec(&invite)?);
        let mut envelope = InvitationEnvelope {
            version: 2,
            owner_id: room.owner_id.clone(),
            device_id: device.into(),
            ciphertext: access::seal(device, &plain)?,
            signature: String::new(),
        };
        envelope.signature =
            signing::sign_message(&self.state.signing_seed, &serde_json::to_vec(&envelope)?)?;
        Ok(json!({"invitation":URL_SAFE_NO_PAD.encode(serde_json::to_vec(&envelope)?)}))
    }

    /// Publish selected originals into a separate encryption scope. The parent
    /// Room journal, key and credentials never enter this scope.
    fn publish_album(
        &mut self,
        room_id: &str,
        album_id: &str,
        new_audience: bool,
    ) -> Result<Value> {
        let scopes: Vec<_> = self
            .state
            .album_shares
            .iter()
            .filter(|share| !new_audience && share.room_id == room_id && share.album_id == album_id)
            .map(|share| share.scope_id.clone())
            .collect();
        if scopes.is_empty() {
            return self.publish_album_scope(room_id, album_id, None);
        }
        let mut first = None;
        for scope in &scopes {
            let result = self.publish_album_scope(room_id, album_id, Some(scope.clone()))?;
            first.get_or_insert(result);
        }
        let mut result = first.ok_or_else(|| invalid("Album publication missing"))?;
        result["updated_scope_ids"] = json!(scopes);
        Ok(result)
    }

    fn publish_album_scope(
        &mut self,
        room_id: &str,
        album_id: &str,
        existing_scope: Option<String>,
    ) -> Result<Value> {
        let parent = self.room(room_id)?;
        if parent.album_only
            || !access::allowed_device(parent, &self.device()?)
            || !matches!(parent.grant.role, Role::Owner | Role::Admin)
        {
            return Err(CoreError::AccessDenied);
        }
        let album: Album = serde_json::from_value(
            self.entities(room_id)?
                .get(&format!("album:{album_id}"))
                .cloned()
                .ok_or_else(|| invalid("Album not found"))?,
        )?;
        let available = self.assets(room_id)?;
        let selected: Vec<_> = available
            .iter()
            .filter(|a| album.asset_ids.contains(&a.id) && !a.deleted)
            .cloned()
            .collect();
        let mut selected_ids: BTreeSet<_> = selected.iter().map(|a| a.id.clone()).collect();
        for asset in &selected {
            for component in &asset.components {
                if available.iter().any(|a| &a.id == component && !a.deleted) {
                    selected_ids.insert(component.clone());
                }
            }
        }
        let selected: Vec<_> = available
            .into_iter()
            .filter(|a| selected_ids.contains(&a.id))
            .collect();
        if selected
            .iter()
            .any(|a| !self.blob(room_id, &a.id).is_ok_and(|p| p.is_file()))
        {
            return Err(invalid("Download the album originals before publishing"));
        }
        let fresh = existing_scope.is_none();
        let scope_id = match existing_scope {
            Some(scope_id) => scope_id,
            None => {
                let scope_id = id();
                let device = self.device()?;
                self.state.rooms.push(Room {
                    id: scope_id.clone(),
                    name: album.title.clone(),
                    owner_id: device.clone(),
                    secret: hex::encode(crypto::key()),
                    grant: signing::grant(
                        &self.state.signing_seed,
                        &scope_id,
                        &device,
                        Role::Owner,
                    )?,
                    operations: vec![],
                    album_only: true,
                    issued_grants: BTreeMap::new(),
                    access_changes: vec![],
                    epoch_secrets: BTreeMap::new(),
                    rejected_operations: vec![],
                });
                self.state.album_shares.push(AlbumShare {
                    room_id: room_id.into(),
                    album_id: album_id.into(),
                    scope_id: scope_id.clone(),
                    snapshot_operations: BTreeSet::new(),
                });
                scope_id
            }
        };
        let scope_secret = crypto::parse_key(&self.room(&scope_id)?.secret)?;
        let mut mapping = BTreeMap::new();
        for asset in &selected {
            mapping.insert(
                asset.id.clone(),
                crypto::object_id(&scope_secret, &asset.sha256)?,
            );
        }
        let temporary = tempfile::tempdir_in(&self.root)?;
        for asset in &selected {
            let filename = Path::new(&asset.filename)
                .file_name()
                .ok_or_else(|| invalid("Invalid original filename"))?;
            let original = temporary.path().join(filename);
            crypto::decrypt_file(
                &crypto::derive(
                    &access::key(self.room(room_id)?, asset.key_epoch)?,
                    b"media",
                ),
                &self.blob(room_id, &asset.id)?,
                &original,
                &format!("{room_id}:{}", asset.id),
                &asset.sha256,
            )?;
            self.import_file(&scope_id, &original, Some(asset.captured_at))?;
            let scope_asset = &mapping[&asset.id];
            let components: Vec<_> = asset
                .components
                .iter()
                .filter_map(|id| mapping.get(id))
                .collect();
            self.write_op(
                &scope_id,
                "asset_patch",
                scope_asset,
                json!({"caption":asset.caption,"favorite":asset.favorite,
                    "captured_at":asset.captured_at,"components":components,"deleted":false}),
            )?;
            fs::remove_file(original)?;
        }
        for previous in self.assets(&scope_id)? {
            if !mapping.values().any(|id| id == &previous.id) && !previous.deleted {
                self.write_op(
                    &scope_id,
                    "asset_patch",
                    &previous.id,
                    json!({"deleted":true}),
                )?;
            }
        }
        let asset_ids: Vec<_> = album
            .asset_ids
            .iter()
            .filter_map(|id| mapping.get(id))
            .cloned()
            .collect();
        self.dispatch(
            json!({"action":"save_album","room_id":scope_id,"id":album_id,
            "title":album.title,"caption":album.caption,"asset_ids":asset_ids,
            "cover_asset_id":album.cover_asset_id.as_ref().and_then(|id| mapping.get(id)),"pinned":false}),
        )?;
        self.room_mut(&scope_id)?.name = album.title;
        if fresh {
            let initial = self
                .room(&scope_id)?
                .operations
                .iter()
                .map(|op| op.id.clone())
                .collect();
            if let Some(share) = self
                .state
                .album_shares
                .iter_mut()
                .find(|share| share.scope_id == scope_id)
            {
                share.snapshot_operations = initial;
            }
        }
        Ok(json!({"scope_id":scope_id,"album_id":album_id,"count":asset_ids.len()}))
    }

    pub fn import_file(
        &mut self,
        room_id: &str,
        path: &Path,
        captured: Option<i64>,
    ) -> Result<Value> {
        if self.room(room_id)?.grant.role == Role::Viewer {
            return Err(CoreError::AccessDenied);
        }
        if !path.is_file() {
            return Err(invalid("Choose a media file"));
        }
        let (hash, size) = crypto::digest(path)?;
        let secret = crypto::parse_key(&self.room(room_id)?.secret)?;
        let asset_id = crypto::object_id(&secret, &hash)?;
        let existing = self.assets(room_id)?.into_iter().find(|a| a.id == asset_id);
        let duplicate = existing.is_some();
        let key_epoch = existing
            .as_ref()
            .map_or(access::epoch(self.room(room_id)?), |a| a.key_epoch);
        if duplicate
            && self.blob(room_id, &asset_id)?.is_file()
            && !self.state.damaged_originals.contains(&asset_id)
        {
            return Ok(json!({"id":asset_id,"duplicate":true}));
        }
        let target = self.blob(room_id, &asset_id)?;
        let parent = target
            .parent()
            .ok_or_else(|| invalid("Missing object directory"))?;
        fs::create_dir_all(parent)?;
        let tmp = tempfile::NamedTempFile::new_in(parent)?;
        let media_key = crypto::derive(&access::key(self.room(room_id)?, key_epoch)?, b"media");
        crypto::encrypt_file(
            &media_key,
            path,
            tmp.path(),
            &format!("{room_id}:{asset_id}"),
        )?;
        let verified = tempfile::NamedTempFile::new_in(parent)?;
        crypto::decrypt_file(
            &media_key,
            tmp.path(),
            verified.path(),
            &format!("{room_id}:{asset_id}"),
            &hash,
        )?;
        tmp.persist(&target).map_err(|e| CoreError::from(e.error))?;
        self.state.damaged_originals.retain(|id| id != &asset_id);
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let kind = if ["mp4", "mov", "m4v", "mkv", "avi", "webm"].contains(&extension.as_str()) {
            "video"
        } else if ["mp3", "wav", "m4a", "aac", "flac"].contains(&extension.as_str()) {
            "audio"
        } else if [
            "jpg", "jpeg", "png", "heic", "heif", "tiff", "gif", "webp", "avif", "dng",
        ]
        .contains(&extension.as_str())
        {
            "photo"
        } else {
            "file"
        };
        let perceptual_hash = if kind == "photo" {
            crate::photo_duplicates::perceptual_hash(path)
        } else {
            None
        };
        let asset = MediaAsset {
            id: asset_id.clone(),
            room_id: room_id.into(),
            filename: path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled".into()),
            kind: kind.into(),
            sha256: hash,
            size,
            captured_at: captured.unwrap_or(now()),
            contributed_at: now(),
            contributor: self.device()?,
            favorite: false,
            caption: String::new(),
            deleted: false,
            people: vec![],
            components: vec![],
            key_epoch,
        };
        if !duplicate {
            self.write_op(room_id, "asset", &asset_id, serde_json::to_value(&asset)?)?;
            if let Some(difference_hash) = perceptual_hash {
                self.state.photo_fingerprints.retain(|fingerprint| {
                    fingerprint.room_id != room_id || fingerprint.asset_id != asset_id
                });
                self.state.photo_fingerprints.push(PhotoFingerprint {
                    room_id: room_id.into(),
                    asset_id: asset_id.clone(),
                    asset_sha256: asset.sha256.clone(),
                    difference_hash,
                });
            }
        }
        Ok(json!({"id":asset_id,"duplicate":duplicate}))
    }
    fn save_story(&mut self, c: &Value) -> Result<Value> {
        let mut story: StoryProject = serde_json::from_value(c["project"].clone())?;
        validate_story(&story)?;
        self.require_story_editor(&story.room_id)?;
        let known = self.assets(&story.room_id)?;
        if story
            .clips
            .iter()
            .any(|clip| !known.iter().any(|a| a.id == clip.asset_id))
        {
            return Err(invalid("A film clip is not in this Room"));
        }
        story.editor = self.device()?;
        story.version_id = id();
        story.archived = false;
        if story.created_at == 0 {
            story.created_at = now();
        }
        // Drafts stay on this device. Only an explicit publication enters the shared journal.
        if c["publish"].as_bool().unwrap_or(false) {
            if !story
                .published_asset
                .as_ref()
                .is_some_and(|id| known.iter().any(|a| &a.id == id && a.kind == "video"))
            {
                return Err(invalid("Publish a rendered movie first"));
            }
            self.write_op(
                &story.room_id,
                "story",
                &story.version_id,
                serde_json::to_value(&story)?,
            )?;
            for previous in &mut self.state.drafts {
                if previous.id == story.id && previous.room_id == story.room_id {
                    previous.archived = true;
                }
            }
            return Ok(json!({"id":story.id,"version_id":story.version_id}));
        }
        self.state
            .drafts
            .retain(|p| !p.auto_generated || p.created_at >= now() - 14 * 86400);
        for previous in &mut self.state.drafts {
            if previous.room_id == story.room_id
                && previous.id == story.id
                && previous.auto_generated
            {
                previous.archived = true;
            }
        }
        self.state.drafts.push(story.clone());
        Ok(json!({"id":story.id,"version_id":story.version_id}))
    }

    fn skip_auto_reel(&mut self, c: &Value) -> Result<Value> {
        let room_id = text(c, "room_id")?;
        let story_id = text(c, "id")?;
        self.room(room_id)?;
        let device_id = self.device()?;
        let active_reel_exists = self.state.drafts.iter().any(|story| {
            story.room_id == room_id
                && story.id == story_id
                && story.auto_generated
                && !story.archived
                && story.published_asset.is_none()
                && story.editor == device_id
        });
        if !active_reel_exists {
            return Err(invalid("Private weekly reel not found"));
        }
        let before = self.state.drafts.len();
        self.state.drafts.retain(|story| {
            !(story.room_id == room_id
                && story.id == story_id
                && story.auto_generated
                && story.published_asset.is_none()
                && story.editor == device_id)
        });
        let removed = before - self.state.drafts.len();
        Ok(json!({"id":story_id,"skipped":true,"versions_removed":removed}))
    }

    fn require_story_editor(&self, room_id: &str) -> Result<()> {
        let room = self.room(room_id)?;
        if room.grant.role == Role::Viewer || !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        Ok(())
    }
    pub fn merge(&mut self, room_id: &str, operations: Vec<Operation>) -> Result<usize> {
        let room = self.room(room_id)?;
        let known: BTreeSet<_> = room.operations.iter().map(|o| o.id.clone()).collect();
        let mut added = vec![];
        let mut seen = known;
        for op in operations {
            let grant_owner = access::grant_issuer(room, &op.grant)
                .unwrap_or_else(|| access::root_owner_id(&room.access_changes, &room.owner_id));
            signing::verify_operation(&op, grant_owner, room_id)?;
            if !access::allowed_operation(room, &op)? {
                return Err(CoreError::AccessDenied);
            }
            safe_id(&op.id)?;
            safe_id(&op.entity_id)?;
            if op.clock > i64::MAX as u64 {
                return Err(invalid("Invalid synchronization clock"));
            }
            if ![
                "asset",
                "album",
                "album_member",
                "playlist",
                "moment",
                "person",
                "person_merge",
                "comment",
                "reaction",
                "story",
                "settings",
                "asset_patch",
            ]
            .contains(&op.kind.as_str())
            {
                return Err(invalid("Unknown operation type"));
            }
            if let Some(existing) = room.operations.iter().find(|o| o.id == op.id) {
                if serde_json::to_vec(existing)? != serde_json::to_vec(&op)? {
                    return Err(CoreError::Authentication);
                }
            }
            if seen.insert(op.id.clone()) {
                added.push(op);
            }
        }
        let count = added.len();
        let old_len = self.room(room_id)?.operations.len();
        self.room_mut(room_id)?.operations.extend(added.clone());
        for op in &added {
            if let Err(error) = self.validate_operation_payload(op) {
                self.room_mut(room_id)?.operations.truncate(old_len);
                return Err(error);
            }
        }
        Ok(count)
    }
    fn validate_operation_payload(&self, op: &Operation) -> Result<()> {
        match op.kind.as_str() {
            "asset" => {
                let asset: MediaAsset = serde_json::from_value(op.value.clone())?;
                if asset.id != op.entity_id
                    || asset.room_id != op.room_id
                    || asset.contributor != op.author
                    || asset.deleted
                    || asset.sha256.len() != 64
                    || asset.id
                        != crypto::object_id(
                            &crypto::parse_key(&self.room(&op.room_id)?.secret)?,
                            &asset.sha256,
                        )?
                {
                    return Err(CoreError::AccessDenied);
                }
            }
            "asset_patch" => {
                let fields = op
                    .value
                    .as_object()
                    .ok_or_else(|| invalid("Invalid media edit"))?;
                for (key, value) in fields {
                    let valid = match key.as_str() {
                        "caption" => value.as_str().is_some_and(|s| s.len() <= 10000),
                        "favorite" | "deleted" => value.is_boolean(),
                        "captured_at" => value.is_i64(),
                        "people" | "components" => value
                            .as_array()
                            .is_some_and(|a| a.iter().all(Value::is_string)),
                        _ => false,
                    };
                    if !valid {
                        return Err(CoreError::AccessDenied);
                    }
                }
                if fields.contains_key("deleted")
                    && !matches!(op.grant.role, Role::Owner | Role::Admin)
                {
                    // Find the original signed contribution; contributors cannot delete others' media.
                    let author = self
                        .room(&op.room_id)?
                        .operations
                        .iter()
                        .find(|o| o.kind == "asset" && o.entity_id == op.entity_id)
                        .map(|o| &o.author);
                    if author != Some(&op.author) {
                        return Err(CoreError::AccessDenied);
                    }
                }
            }
            "album" => {
                let a: Album = serde_json::from_value(op.value.clone())?;
                if a.id != op.entity_id
                    || a.room_id != op.room_id
                    || a.asset_ids.len() > 5_000
                    || a.asset_ids.iter().collect::<BTreeSet<_>>().len() != a.asset_ids.len()
                    || a.caption.len() > 2_000
                    || a.cover_asset_id
                        .as_ref()
                        .is_some_and(|cover| !a.asset_ids.contains(cover))
                {
                    return Err(CoreError::AccessDenied);
                }
                title(&a.title)?;
            }
            "playlist" => {
                let p: FilmPlaylist = serde_json::from_value(op.value.clone())?;
                if p.id != op.entity_id
                    || p.room_id != op.room_id
                    || p.story_ids.len() > 500
                    || p.story_ids.iter().collect::<BTreeSet<_>>().len() != p.story_ids.len()
                {
                    return Err(CoreError::AccessDenied);
                }
                safe_id(&p.id)?;
                title(&p.title)?;
                for story_id in &p.story_ids {
                    safe_id(story_id)?;
                }
            }
            "album_member" => {
                let album_id = text(&op.value, "album_id")?;
                let asset_id = text(&op.value, "asset_id")?;
                if op.entity_id != format!("{album_id}-{asset_id}")
                    || !op.value["included"].is_boolean()
                    || !self
                        .entities(&op.room_id)?
                        .contains_key(&format!("album:{album_id}"))
                    || !self
                        .assets(&op.room_id)?
                        .iter()
                        .any(|asset| asset.id == asset_id)
                {
                    return Err(CoreError::AccessDenied);
                }
            }
            "moment" => {
                let m: Moment = serde_json::from_value(op.value.clone())?;
                if m.id != op.entity_id
                    || m.room_id != op.room_id
                    || m.start > m.end
                    || m.asset_ids.is_empty()
                {
                    return Err(CoreError::AccessDenied);
                }
                title(&m.title)?;
                let assets = self.assets(&op.room_id)?;
                if m.asset_ids
                    .iter()
                    .any(|id| !assets.iter().any(|a| a.id == *id))
                {
                    return Err(CoreError::AccessDenied);
                }
            }
            "person" => {
                let p: PersonTag = serde_json::from_value(op.value.clone())?;
                if p.id != op.entity_id || p.room_id != op.room_id {
                    return Err(CoreError::AccessDenied);
                }
                title(&p.name)?;
            }
            "person_merge" => {
                let merge: PersonMerge = serde_json::from_value(op.value.clone())?;
                if merge.room_id != op.room_id
                    || merge.source_person_id != op.entity_id
                    || merge.source_person_id == merge.target_person_id
                    || op.grant.role == Role::Viewer
                {
                    return Err(CoreError::AccessDenied);
                }
                safe_id(&merge.source_person_id)?;
                safe_id(&merge.target_person_id)?;
                for person_id in [&merge.source_person_id, &merge.target_person_id] {
                    let person: PersonTag = serde_json::from_value(
                        self.entities(&op.room_id)?
                            .get(&format!("person:{person_id}"))
                            .cloned()
                            .ok_or(CoreError::AccessDenied)?,
                    )?;
                    if person.room_id != op.room_id {
                        return Err(CoreError::AccessDenied);
                    }
                }
            }
            "story" => {
                let p: StoryProject = serde_json::from_value(op.value.clone())?;
                validate_story(&p)?;
                if p.version_id != op.entity_id || p.room_id != op.room_id || p.editor != op.author
                {
                    return Err(CoreError::AccessDenied);
                }
            }
            "comment" => {
                if op.value["author"] != op.author
                    || op.value["id"] != op.entity_id
                    || !op.value["body"]
                        .as_str()
                        .is_some_and(|s| !s.trim().is_empty() && s.len() <= 10000)
                {
                    return Err(CoreError::AccessDenied);
                }
            }
            "settings" => {
                if !matches!(op.grant.role, Role::Owner | Role::Admin) {
                    return Err(CoreError::AccessDenied);
                }
            }
            "reaction" => {
                let asset_id = text(&op.value, "asset_id")?;
                if op.value.as_object().is_none_or(|value| value.len() != 4)
                    || op.value["author"] != op.author
                    || op.value["id"] != op.entity_id
                    || op.entity_id != format!("like-{asset_id}-{}", op.author)
                    || !op.value["liked"].is_boolean()
                    || !self
                        .assets(&op.room_id)?
                        .iter()
                        .any(|asset| asset.id == asset_id)
                {
                    return Err(CoreError::AccessDenied);
                }
            }
            _ => return Err(CoreError::AccessDenied),
        }
        Ok(())
    }
    pub fn packet(&self, room_id: &str) -> Result<Vec<u8>> {
        let r = self.room(room_id)?;
        if !access::allowed_device(r, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let plain = serde_json::to_vec(
            &json!({"version":VERSION,"room_id":room_id,"operations":r.operations}),
        )?;
        let epoch = access::epoch(r);
        if epoch == 0 {
            return crypto::encrypt(
                &access::key(r, 0)?,
                &plain,
                format!("room:{room_id}:v1").as_bytes(),
            );
        }
        let ciphertext = crypto::encrypt(
            &access::key(r, epoch)?,
            &plain,
            format!("room:{room_id}:epoch:{epoch}:v2").as_bytes(),
        )?;
        Ok(serde_json::to_vec(
            &json!({"protocol":2,"room_id":room_id,"key_epoch":epoch,
            "access_changes":r.access_changes,"ciphertext":URL_SAFE_NO_PAD.encode(ciphertext)}),
        )?)
    }
    pub fn merge_packet(&mut self, room_id: &str, packet: &[u8]) -> Result<usize> {
        let plain = if packet.starts_with(b"FRE1") {
            crypto::decrypt(
                &access::key(self.room(room_id)?, 0)?,
                packet,
                format!("room:{room_id}:v1").as_bytes(),
            )?
        } else {
            let envelope: Value = serde_json::from_slice(packet)?;
            if envelope["protocol"] != 2 || envelope["room_id"] != room_id {
                return Err(CoreError::Authentication);
            }
            let changes: Vec<AccessChange> =
                serde_json::from_value(envelope["access_changes"].clone())?;
            let packet_epoch = changes
                .last()
                .map(|change| change.epoch)
                .ok_or(CoreError::Authentication)?;
            let seed = self.state.signing_seed.clone();
            access::apply(self.room_mut(room_id)?, changes, &seed)?;
            let epoch = envelope["key_epoch"]
                .as_u64()
                .ok_or(CoreError::Authentication)?;
            if epoch != packet_epoch {
                return Err(CoreError::Authentication);
            }
            let ciphertext = URL_SAFE_NO_PAD
                .decode(text(&envelope, "ciphertext")?)
                .map_err(|_| CoreError::Authentication)?;
            crypto::decrypt(
                &access::key(self.room(room_id)?, epoch)?,
                &ciphertext,
                format!("room:{room_id}:epoch:{epoch}:v2").as_bytes(),
            )?
        };
        let value: Value = serde_json::from_slice(&plain)?;
        if value["version"] != VERSION || value["room_id"] != room_id {
            return Err(invalid("Wrong Room or protocol version"));
        }
        self.merge(
            room_id,
            serde_json::from_value(value["operations"].clone())?,
        )
    }
    fn export_room(&self, room_id: &str, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| invalid("Choose a destination"))?;
        let temp = tempfile::NamedTempFile::new_in(parent)?;
        let mut zip = ZipWriter::new(temp);
        zip.start_file("manifest.fr", SimpleFileOptions::default())?;
        zip.write_all(&self.packet(room_id)?)?;
        for a in self.assets(room_id)? {
            let blob = self.blob(room_id, &a.id)?;
            if blob.is_file() {
                zip.start_file(
                    format!("objects/{}.frblob", a.id),
                    SimpleFileOptions::default(),
                )?;
                std::io::copy(&mut fs::File::open(blob)?, &mut zip)?;
            }
        }
        let temp = zip.finish()?;
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|e| CoreError::from(e.error))?;
        Ok(())
    }
    fn import_room(&mut self, room_id: &str, path: &Path) -> Result<()> {
        let mut archive = ZipArchive::new(fs::File::open(path)?)?;
        let mut manifest = vec![];
        archive
            .by_name("manifest.fr")?
            .take(32 * 1024 * 1024)
            .read_to_end(&mut manifest)?;
        self.merge_packet(room_id, &manifest)?;
        for a in self.assets(room_id)? {
            let entry = archive.by_name(&format!("objects/{}.frblob", a.id));
            if let Ok(mut entry) = entry {
                let dest = self.blob(room_id, &a.id)?;
                let parent = dest.parent().ok_or_else(|| invalid("Missing directory"))?;
                fs::create_dir_all(parent)?;
                let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
                std::io::copy(&mut entry, &mut tmp)?;
                let clear = tempfile::NamedTempFile::new_in(parent)?;
                crypto::decrypt_file(
                    &crypto::derive(&access::key(self.room(room_id)?, a.key_epoch)?, b"media"),
                    tmp.path(),
                    clear.path(),
                    &format!("{room_id}:{}", a.id),
                    &a.sha256,
                )?;
                tmp.persist(dest).map_err(|e| CoreError::from(e.error))?;
            }
        }
        Ok(())
    }
    fn backup(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| invalid("Choose a destination"))?;
        let temp = tempfile::NamedTempFile::new_in(parent)?;
        let mut zip = ZipWriter::new(temp);
        zip.start_file("catalog.fr", SimpleFileOptions::default())?;
        zip.write_all(&crypto::encrypt(
            &self.master,
            &serde_json::to_vec(&self.state)?,
            b"family-room-catalog-v1",
        )?)?;
        for r in &self.state.rooms {
            for a in self.recoverable_assets(&r.id)? {
                let blob = self.blob(&r.id, &a.id)?;
                if blob.is_file() {
                    zip.start_file(
                        format!("objects/{}/{}.frblob", r.id, a.id),
                        SimpleFileOptions::default(),
                    )?;
                    std::io::copy(&mut fs::File::open(blob)?, &mut zip)?;
                }
            }
        }
        let temp = zip.finish()?;
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|e| CoreError::from(e.error))?;
        Ok(())
    }
    fn restore(&mut self, path: &Path, recovery: &[u8; 32]) -> Result<()> {
        if !self.state.rooms.is_empty() || !self.state.pending_invitations.is_empty() {
            return Err(invalid(
                "Restore into an empty library without pending invitations to protect existing data",
            ));
        }
        let mut archive = ZipArchive::new(fs::File::open(path)?)?;
        let mut bytes = vec![];
        archive
            .by_name("catalog.fr")?
            .take(32 * 1024 * 1024)
            .read_to_end(&mut bytes)?;
        let mut state: State = serde_json::from_slice(&crypto::decrypt(
            recovery,
            &bytes,
            b"family-room-catalog-v1",
        )?)?;
        state.replicas.clear();
        state.transfers.clear();
        for s in &mut state.sources {
            s.paused = true;
        }
        for w in &mut state.watches {
            w.paused = true;
        }
        self.state = state;
        self.validate_state()?;
        for room in self.state.rooms.clone() {
            for a in self.recoverable_assets(&room.id)? {
                if let Ok(mut entry) =
                    archive.by_name(&format!("objects/{}/{}.frblob", room.id, a.id))
                {
                    let dest = self.blob(&room.id, &a.id)?;
                    let parent = dest.parent().ok_or_else(|| invalid("Missing directory"))?;
                    fs::create_dir_all(parent)?;
                    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
                    std::io::copy(&mut entry, &mut temp)?;
                    let clear = tempfile::NamedTempFile::new_in(parent)?;
                    crypto::decrypt_file(
                        &crypto::derive(&access::key(&room, a.key_epoch)?, b"media"),
                        temp.path(),
                        clear.path(),
                        &format!("{}:{}", room.id, a.id),
                        &a.sha256,
                    )?;
                    temp.persist(dest).map_err(|e| CoreError::from(e.error))?;
                }
            }
        }
        Ok(())
    }
    fn network_wait_reason(&self, source: &StorageSource, direction: &str) -> Option<String> {
        if source.kind == "local" {
            return None;
        }
        if self.network_state == "offline" {
            return Some(format!("{direction} waiting for a network"));
        }
        if source.network_policy == "unmetered_only" && self.network_state != "unmetered" {
            return Some(format!("{direction} waiting for an unmetered network"));
        }
        None
    }
    fn source(&self, source_id: &str) -> Result<StorageSource> {
        self.state
            .sources
            .iter()
            .find(|s| s.id == source_id)
            .cloned()
            .ok_or_else(|| invalid("Storage source not found"))
    }
    fn is_preferred(&self, source: &StorageSource) -> bool {
        source.preferred
            || (!self
                .state
                .sources
                .iter()
                .any(|s| s.room_id == source.room_id && s.preferred)
                && self
                    .state
                    .sources
                    .iter()
                    .find(|s| s.room_id == source.room_id)
                    .is_some_and(|s| s.id == source.id))
    }
    fn export_library(&self, room_id: &str, destination: &Path) -> Result<Value> {
        if destination.exists() {
            return Err(invalid("Choose a new export folder"));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| invalid("Missing export folder"))?;
        fs::create_dir_all(parent)?;
        let staging = tempfile::TempDir::new_in(parent)?;
        let originals = staging.path().join("originals");
        fs::create_dir(&originals)?;
        let room = self.room(room_id)?;
        let snapshot = self.snapshot()?;
        let mut metadata = snapshot["rooms"]
            .as_array()
            .and_then(|a| a.iter().find(|r| r["id"] == room_id))
            .ok_or_else(|| invalid("Room not found"))?
            .clone();
        let mut count = 0;
        for asset in self.assets(room_id)?.iter().filter(|a| !a.deleted) {
            let name = original_export_filename(asset);
            let target = originals.join(&name);
            crypto::decrypt_file(
                &crypto::derive(&access::key(room, asset.key_epoch)?, b"media"),
                &self.blob(room_id, &asset.id)?,
                &target,
                &format!("{}:{}", room_id, asset.id),
                &asset.sha256,
            )?;
            if let Some(exported) = metadata["assets"]
                .as_array_mut()
                .and_then(|values| values.iter_mut().find(|a| a["id"] == asset.id))
            {
                exported["export_file"] = json!(format!("originals/{name}"));
            }
            count += 1;
        }
        fs::write(
            staging.path().join("library.json"),
            serde_json::to_vec_pretty(
                &json!({"format":"family-room-export","version":1,"room":metadata}),
            )?,
        )?;
        fs::rename(staging.path(), destination)?;
        Ok(json!({"exported":count,"destination":destination}))
    }
    fn export_preserved(&self, room_id: &str, path: &Path) -> Result<Value> {
        // This explicit export is plaintext; the device vault stays encrypted.
        let room = self.room(room_id)?;
        if path.exists() {
            return Err(invalid("Choose a new recovery export file"));
        }
        let parent = path
            .parent()
            .ok_or_else(|| invalid("Choose a destination"))?;
        fs::create_dir_all(parent)?;
        let temp = tempfile::NamedTempFile::new_in(parent)?;
        let mut archive = ZipWriter::new(temp);
        let projects: Vec<_> = self
            .state
            .drafts
            .iter()
            .filter(|p| p.room_id == room_id && !p.archived)
            .collect();
        fn references(value: &Value, ids: &mut BTreeSet<String>) {
            match value {
                Value::Object(fields) => {
                    for (name, value) in fields {
                        if name == "asset_id" {
                            if let Some(id) = value.as_str() {
                                ids.insert(id.into());
                            }
                        } else if name == "asset_ids" || name == "components" {
                            if let Some(values) = value.as_array() {
                                ids.extend(
                                    values.iter().filter_map(Value::as_str).map(str::to_owned),
                                );
                            }
                        }
                        references(value, ids);
                    }
                }
                Value::Array(values) => {
                    for value in values {
                        references(value, ids);
                    }
                }
                _ => {}
            }
        }
        let mut ids = BTreeSet::new();
        for op in &room.rejected_operations {
            if op.kind == "asset" || op.kind == "asset_patch" {
                ids.insert(op.entity_id.clone());
            }
            references(&op.value, &mut ids);
        }
        for project in &projects {
            references(&serde_json::to_value(project)?, &mut ids);
        }
        let assets = self.recoverable_assets(room_id)?;
        loop {
            let previous = ids.len();
            for asset in &assets {
                if ids.contains(&asset.id) {
                    ids.extend(asset.components.clone());
                }
            }
            if previous == ids.len() {
                break;
            }
        }
        let mut originals = vec![];
        let mut exported = 0;
        let mut missing = vec![];
        let referenced_assets: Vec<_> = assets.iter().filter(|a| ids.contains(&a.id)).collect();
        for asset in referenced_assets {
            let blob = self.blob(room_id, &asset.id)?;
            let mut value = serde_json::to_value(asset)?;
            if blob.is_file() {
                let scratch = tempfile::NamedTempFile::new_in(parent)?;
                crypto::decrypt_file(
                    &crypto::derive(&access::key(room, asset.key_epoch)?, b"media"),
                    &blob,
                    scratch.path(),
                    &format!("{room_id}:{}", asset.id),
                    &asset.sha256,
                )?;
                let filename = format!("originals/{}", original_export_filename(asset));
                archive.start_file(&filename, SimpleFileOptions::default())?;
                std::io::copy(&mut fs::File::open(scratch.path())?, &mut archive)?;
                value["export_file"] = json!(filename);
                exported += 1;
            } else {
                value["export_file"] = Value::Null;
                missing.push(asset.id.clone());
            }
            ids.remove(&asset.id);
            originals.push(value);
        }
        missing.extend(ids);
        archive.start_file("preserved-changes.json", SimpleFileOptions::default())?;
        archive.write_all(&serde_json::to_vec_pretty(&json!({
            "format":"family-room-preserved-changes","version":1,
            "room_id":room_id,"room_name":room.name,"access_revision":access::revision(room),
            "created_at":now(),"shared":false,"operations":room.rejected_operations,
            "projects":projects,"originals":originals,"missing_original_ids":missing
        }))?)?;
        archive.start_file("README.txt", SimpleFileOptions::default())?;
        archive.write_all(b"Family Room preserved changes\n\nThis ZIP is not encrypted. Keep it private.\nIt contains offline changes that were not accepted into the shared Room,\nprivate film drafts, and available originals referenced by that work.\nThe JSON manifest preserves captions, organization, project versions and signed edits.\nMissing original IDs identify files that were not on this device.\nExporting does not publish changes or restore membership.\n")?;
        let temp = archive.finish()?;
        temp.as_file().sync_all()?;
        temp.persist_noclobber(path)
            .map_err(|e| CoreError::from(e.error))?;
        Ok(
            json!({"path":path,"exported":exported,"missing":missing.len(),
            "changes":room.rejected_operations.len(),"projects":projects.len()}),
        )
    }
    fn replicate(&mut self, source_id: &str, asset_id: &str) -> Result<Value> {
        self.replicate_mode(source_id, asset_id, false)
    }
    fn replicate_mode(
        &mut self,
        source_id: &str,
        asset_id: &str,
        single_step: bool,
    ) -> Result<Value> {
        let source = self.source(source_id)?;
        if source.paused {
            return Err(invalid("Storage source is paused"));
        }
        if self.room(&source.room_id)?.grant.role == Role::Viewer {
            return Err(CoreError::AccessDenied);
        }
        let waiting = self.network_wait_reason(&source, "Upload");
        if waiting.is_none() && !self.refresh_access(source_id)? {
            return Err(CoreError::AccessDenied);
        }
        // Access refresh may rotate a gateway credential. Use the updated source.
        let source = self.source(source_id)?;
        let input = self.blob(&source.room_id, asset_id)?;
        let total = fs::metadata(&input)?.len();
        let existing = self.state.transfers.iter().position(|j| {
            j.source_id == source_id && j.asset_id == asset_id && j.state != "cancelled"
        });
        let index = match existing {
            Some(i) => i,
            None => {
                self.state.transfers.push(TransferJob {
                    id: id(),
                    room_id: source.room_id.clone(),
                    asset_id: asset_id.into(),
                    source_id: source_id.into(),
                    state: "queued".into(),
                    transferred: 0,
                    total,
                    error: None,
                    session: None,
                });
                self.state.transfers.len() - 1
            }
        };
        let filename = format!("{}-{asset_id}.frblob", source.room_id);
        if let Some(reason) = waiting {
            let job = &mut self.state.transfers[index];
            job.state = "waiting".into();
            job.error = Some(reason);
            return Ok(serde_json::to_value(job)?);
        }
        self.state.transfers[index].state = "transferring".into();
        self.state.transfers[index].error = None;
        self.persist()?;
        let quota = crate::providers::quota(&source)?;
        let used = self
            .state
            .replicas
            .iter()
            .filter(|r| r.source_id == source_id && r.verified && r.asset_id != asset_id)
            .filter_map(|r| {
                self.assets(&source.room_id)
                    .ok()?
                    .into_iter()
                    .find(|asset| asset.id == r.asset_id)
                    .map(|asset| {
                        asset
                            .size
                            .saturating_add(12)
                            .saturating_add(asset.size.div_ceil(1024 * 1024).saturating_mul(48))
                    })
            })
            .sum::<u64>();
        if source
            .budget
            .is_some_and(|b| used.saturating_add(total) > b)
            || quota["remaining"].as_u64().is_some_and(|free| {
                free < total.saturating_sub(self.state.transfers[index].transferred)
            })
        {
            let job = &mut self.state.transfers[index];
            job.state = "waiting".into();
            job.error = Some("Upload paused: storage full".into());
            return Ok(serde_json::to_value(job)?);
        }
        let mut job = self.state.transfers[index].clone();
        let result = (|| -> Result<()> {
            let asset = self
                .assets(&source.room_id)?
                .into_iter()
                .find(|asset| asset.id == asset_id)
                .ok_or_else(|| invalid("Original not found in this Room"))?;
            let key = crypto::derive(
                &access::key(self.room(&source.room_id)?, asset.key_epoch)?,
                b"media",
            );
            let context = format!("{}:{asset_id}", source.room_id);
            match crypto::verify_file(&key, &input, &context, &asset.sha256) {
                Ok(size) if size == asset.size => {
                    self.state.damaged_originals.retain(|id| id != asset_id);
                }
                result => {
                    if !self.state.damaged_originals.iter().any(|id| id == asset_id) {
                        self.state.damaged_originals.push(asset_id.into());
                    }
                    return Err(result.err().unwrap_or(CoreError::Authentication));
                }
            }
            // A write or failed readback invalidates the previous verification.
            // Local corruption alone must not erase evidence of a good remote copy.
            self.state
                .replicas
                .retain(|r| !(r.asset_id == asset_id && r.source_id == source_id));
            self.persist()?;
            while job.transferred < job.total {
                crate::providers::upload_step(&source, &filename, &input, &mut job)?;
                self.state.transfers[index] = job.clone();
                self.persist()?;
                if single_step && job.transferred < job.total {
                    return Ok(());
                }
            }
            let verified = tempfile::NamedTempFile::new_in(&self.root)?;
            crate::providers::download(&source, &filename, verified.path())?;
            if !crypto::verify_file(&key, verified.path(), &context, &asset.sha256)
                .is_ok_and(|size| size == asset.size)
            {
                // A corrupt completed object needs a new upload, not another
                // attempt to resume at its already-complete byte offset.
                if source.kind == "s3" {
                    if let Some(session) = &job.session {
                        crate::s3::abort(&source, &filename, session)?;
                    }
                }
                job.transferred = 0;
                job.session = None;
                return Err(CoreError::Authentication);
            }
            Ok(())
        })();
        if let Err(e) = result {
            job.state = "waiting".into();
            job.error = Some(e.to_string());
        } else if job.transferred < job.total {
            job.state = "queued".into();
            job.error = None;
        } else {
            job.state = "complete".into();
            job.error = None;
            self.state
                .replicas
                .retain(|r| !(r.asset_id == asset_id && r.source_id == source_id));
            self.state.replicas.push(Replica {
                asset_id: asset_id.into(),
                source_id: source_id.into(),
                verified: true,
            });
        }
        self.state.transfers[index] = job.clone();
        Ok(serde_json::to_value(job)?)
    }

    fn refresh_access(&mut self, source_id: &str) -> Result<bool> {
        let mut source = self.source(source_id)?.clone();
        if source.paused {
            return Err(invalid("Storage source is paused"));
        }
        if self.network_state == "offline" && source.kind != "local" {
            return Err(invalid("Waiting for network"));
        }
        let incoming = crate::providers::read_access(&source)?;
        let incoming_len = incoming.len();
        let seed = self.state.signing_seed.clone();
        access::apply(self.room_mut(&source.room_id)?, incoming, &seed)?;
        let device = self.device()?;
        let (owner_id, grant, revision, changes, active) = {
            let room = self.room(&source.room_id)?;
            (
                room.owner_id.clone(),
                room.grant.clone(),
                access::revision(room),
                room.access_changes.clone(),
                access::allowed_device(room, &device),
            )
        };
        if source.kind == "gateway" {
            let token = serde_json::to_string(&json!({
                "owner_id": owner_id,
                "grant": grant,
                "signing_seed": self.state.signing_seed,
                "access_revision": revision,
            }))?;
            source.token = token.clone();
            if let Some(target) = self
                .state
                .sources
                .iter_mut()
                .find(|item| item.id == source_id)
            {
                target.token = token;
            }
        }
        // A successor can receive an offer-created access revision before the
        // gateway knows about it. Publish with the newly issued Owner grant.
        if device == owner_id && changes.len() > incoming_len {
            crate::providers::publish_access(&source, &changes)?;
        }
        Ok(active)
    }

    fn index_video(&mut self, request: &Value) -> Result<Value> {
        let room_id = text(request, "room_id")?;
        let asset_id = text(request, "asset_id")?;
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let asset = self
            .assets(room_id)?
            .into_iter()
            .find(|asset| asset.id == asset_id && asset.kind == "video" && !asset.deleted)
            .ok_or_else(|| invalid("Video not found in this Room"))?;
        if !self.blob(room_id, asset_id)?.is_file() {
            return Err(invalid("Download the original before indexing this video"));
        }
        let mut audio_indexed = request["audio_indexed"].as_bool().unwrap_or(false);
        let mut frames_indexed = request["frames_indexed"].as_bool().unwrap_or(false);
        let mut sound_events_indexed = request["sound_events_indexed"].as_bool().unwrap_or(false);
        if !audio_indexed && !frames_indexed && !sound_events_indexed {
            return Err(invalid("No video analysis was completed"));
        }
        let mut frame_interval_ms = request["frame_interval_ms"].as_u64();
        if frames_indexed && !frame_interval_ms.is_some_and(|value| (1..=600_000).contains(&value))
        {
            return Err(invalid("Invalid frame sampling interval"));
        }
        let mut observations: Vec<VideoObservation> =
            serde_json::from_value(request["observations"].clone())?;
        let has_sound_events = observations
            .iter()
            .any(|observation| observation.modality == "sound_event");
        sound_events_indexed |= has_sound_events;
        let has_visual_labels = observations
            .iter()
            .any(|observation| observation.modality == "visual_label");
        let mut visual_labels_indexed = request["visual_labels_indexed"]
            .as_bool()
            .unwrap_or(has_visual_labels)
            || has_visual_labels;
        if request["preserve_existing"].as_bool() == Some(true) {
            if let Some(previous) = self.state.video_indexes.iter().find(|index| {
                index.room_id == room_id
                    && index.asset_id == asset_id
                    && index.asset_sha256 == asset.sha256
            }) {
                if !audio_indexed && previous.audio_indexed {
                    audio_indexed = true;
                    observations.extend(
                        previous
                            .observations
                            .iter()
                            .filter(|item| item.modality == "speech")
                            .cloned(),
                    );
                }
                if !sound_events_indexed && previous.sound_events_indexed {
                    sound_events_indexed = true;
                    observations.extend(
                        previous
                            .observations
                            .iter()
                            .filter(|item| item.modality == "sound_event")
                            .cloned(),
                    );
                }
                if !frames_indexed && previous.frames_indexed {
                    frames_indexed = true;
                    frame_interval_ms = previous.frame_interval_ms;
                    observations.extend(
                        previous
                            .observations
                            .iter()
                            .filter(|item| item.modality == "screen_text")
                            .cloned(),
                    );
                }
                let previous_has_visual_labels = previous.visual_labels_indexed
                    || previous
                        .observations
                        .iter()
                        .any(|item| item.modality == "visual_label");
                if !visual_labels_indexed && previous_has_visual_labels {
                    visual_labels_indexed = true;
                    observations.extend(
                        previous
                            .observations
                            .iter()
                            .filter(|item| item.modality == "visual_label")
                            .cloned(),
                    );
                }
            }
        }
        if visual_labels_indexed && !frames_indexed {
            return Err(invalid("Visual labels require a completed frame index"));
        }
        if observations.len() > 15_000 {
            return Err(invalid("Video index contains too many observations"));
        }
        let mut total_bytes = 0usize;
        for observation in &mut observations {
            observation.text = observation.text.trim().to_owned();
            total_bytes = total_bytes.saturating_add(observation.text.len());
            if observation.text.is_empty()
                || observation.text.len() > 1_024
                || total_bytes > 4 * 1024 * 1024
                || observation.start_ms > observation.end_ms
                || observation.end_ms.saturating_sub(observation.start_ms) > 120_000
                || observation.end_ms > 7 * 24 * 60 * 60 * 1_000
                || observation.confidence_percent > 100
            {
                return Err(invalid("Invalid video observation"));
            }
            let permitted = match observation.modality.as_str() {
                "speech" => audio_indexed,
                "sound_event" => sound_events_indexed,
                "screen_text" | "visual_label" => frames_indexed,
                _ => false,
            };
            if !permitted {
                return Err(invalid("Invalid video evidence type"));
            }
        }
        observations.sort_by_key(|entry| entry.start_ms);
        self.state
            .video_indexes
            .retain(|index| index.room_id != room_id || index.asset_id != asset_id);
        self.state.video_indexes.push(VideoIndex {
            room_id: room_id.into(),
            asset_id: asset_id.into(),
            asset_sha256: asset.sha256,
            audio_indexed,
            sound_events_indexed,
            frames_indexed,
            visual_labels_indexed,
            frame_interval_ms,
            observations,
        });
        Ok(
            json!({"indexed":true,"observations":self.state.video_indexes.last().map_or(0, |index| index.observations.len())}),
        )
    }

    fn suggest_moments(&self, request: &Value) -> Result<Value> {
        let room_id = text(request, "room_id")?;
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let assets = self.assets(room_id)?;
        let moments: Vec<Moment> = self
            .entities(room_id)?
            .into_iter()
            .filter(|(key, _)| key.starts_with("moment:"))
            .map(|(_, value)| serde_json::from_value(value).map_err(CoreError::from))
            .collect::<Result<_>>()?;
        let suggestions = crate::moment_suggestions::suggest_moments(room_id, &assets, &moments);
        Ok(json!({"suggestions":suggestions}))
    }

    fn suggest_photo_duplicates(&mut self, request: &Value) -> Result<Value> {
        let room_id = text(request, "room_id")?;
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let assets = self.assets(room_id)?;
        let mut newly_indexed = 0usize;
        let mut unavailable_locally = 0usize;
        let mut unsupported_or_oversized = 0usize;
        for asset in assets
            .iter()
            .filter(|asset| asset.kind == "photo" && !asset.deleted)
        {
            let is_indexed = self.state.photo_fingerprints.iter().any(|fingerprint| {
                fingerprint.room_id == room_id
                    && fingerprint.asset_id == asset.id
                    && fingerprint.asset_sha256 == asset.sha256
            });
            if is_indexed {
                continue;
            }

            let path = self.blob(room_id, &asset.id)?;
            if !path.is_file() {
                unavailable_locally += 1;
                continue;
            }
            if asset.size == 0 || asset.size > crate::photo_duplicates::MAX_ENCRYPTED_SOURCE_BYTES {
                unsupported_or_oversized += 1;
                continue;
            }

            let media_key = crypto::derive(
                &access::key(self.room(room_id)?, asset.key_epoch)?,
                b"media",
            );
            let context = format!("{room_id}:{}", asset.id);
            let bytes = match crypto::decrypt_file_bounded(
                &media_key,
                &path,
                &context,
                &asset.sha256,
                asset
                    .size
                    .min(crate::photo_duplicates::MAX_ENCRYPTED_SOURCE_BYTES)
                    as usize,
            ) {
                Ok(bytes) => bytes,
                Err(_) => {
                    if !self.state.damaged_originals.contains(&asset.id) {
                        self.state.damaged_originals.push(asset.id.clone());
                    }
                    continue;
                }
            };
            if let Some(difference_hash) = crate::photo_duplicates::perceptual_hash_bytes(&bytes) {
                self.state.photo_fingerprints.push(PhotoFingerprint {
                    room_id: room_id.into(),
                    asset_id: asset.id.clone(),
                    asset_sha256: asset.sha256.clone(),
                    difference_hash,
                });
                newly_indexed += 1;
            } else {
                unsupported_or_oversized += 1;
            }
        }

        let analysis = crate::photo_duplicates::suggest_photo_duplicates(
            room_id,
            &assets,
            &self.state.photo_fingerprints,
        );
        let indexed_photos = self
            .state
            .photo_fingerprints
            .iter()
            .filter(|fingerprint| {
                fingerprint.room_id == room_id
                    && assets.iter().any(|asset| {
                        asset.id == fingerprint.asset_id
                            && asset.sha256 == fingerprint.asset_sha256
                            && asset.kind == "photo"
                            && !asset.deleted
                    })
            })
            .count();
        Ok(json!({
            "suggestions":analysis.suggestions,
            "basis":"perceptual_difference_hash",
            "newly_indexed":newly_indexed,
            "indexed_photos":indexed_photos,
            "unavailable_locally":unavailable_locally,
            "unsupported_or_oversized":unsupported_or_oversized,
            "comparison_limit_reached":analysis.comparison_limit_reached,
            "result_limit_reached":analysis.result_limit_reached
        }))
    }

    fn search_video(&mut self, request: &Value) -> Result<Value> {
        let room_id = text(request, "room_id")?;
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let query = text(request, "query")?.trim();
        if query.is_empty() || query.len() > 200 {
            return Err(invalid("Describe what to find in 200 characters or fewer"));
        }
        let context_query = request["context_query"]
            .as_str()
            .filter(|value| !value.trim().is_empty());
        if context_query.is_some_and(|context| context.len() > 200) {
            return Err(invalid(
                "Previous video-search context must be 200 characters or fewer",
            ));
        }
        let filter = request["modality"].as_str().unwrap_or("all");
        if !["all", "audio", "frame"].contains(&filter) {
            return Err(invalid("Invalid video search filter"));
        }
        let assets = self.assets(room_id)?;
        let indexes: Vec<_> = self
            .state
            .video_indexes
            .iter()
            .filter(|index| index.room_id == room_id)
            .collect();
        if self
            .video_search_cache
            .as_ref()
            .is_none_or(|(cached_room, _)| cached_room != room_id)
        {
            self.video_search_cache = Some((
                room_id.into(),
                crate::video_index::VideoSearchCache::build(&indexes),
            ));
        }
        Ok(
            match self
                .video_search_cache
                .as_ref()
                .and_then(|(_, cache)| cache.as_ref())
            {
                Some(cache) => match context_query {
                    Some(context) => {
                        cache.search_with_context(&indexes, &assets, query, filter, Some(context))
                    }
                    None => cache.search(&indexes, &assets, query, filter),
                },
                None => match context_query {
                    Some(context) => crate::video_index::search_with_context(
                        &indexes,
                        &assets,
                        query,
                        filter,
                        Some(context),
                    ),
                    None => crate::video_index::search(&indexes, &assets, query, filter),
                },
            },
        )
    }

    fn video_index_status(&self, request: &Value) -> Result<Value> {
        let room_id = text(request, "room_id")?;
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let videos: Vec<_> = self
            .assets(room_id)?
            .into_iter()
            .filter(|asset| asset.kind == "video" && !asset.deleted)
            .map(|asset| {
                let index = self.state.video_indexes.iter().find(|index| {
                    index.room_id == room_id
                        && index.asset_id == asset.id
                        && index.asset_sha256 == asset.sha256
                });
                json!({"asset_id":asset.id,
                    "local":self.blob(room_id,&asset.id).is_ok_and(|path| path.is_file()),
                    "indexed":index.is_some(),
                "audio_indexed":index.is_some_and(|index| index.audio_indexed),
                "sound_events_indexed":index.is_some_and(|index| index.sound_events_indexed),
                "frames_indexed":index.is_some_and(|index| index.frames_indexed),
                    "visual_labels_indexed":index.is_some_and(|index| index.visual_labels_indexed),
                    "frame_interval_ms":index.and_then(|index| index.frame_interval_ms)})
            })
            .collect();
        Ok(json!({"videos":videos}))
    }

    fn export_video_transcript(&self, request: &Value) -> Result<Value> {
        let room_id = text(request, "room_id")?;
        let asset_id = text(request, "asset_id")?;
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let asset = self
            .assets(room_id)?
            .into_iter()
            .find(|asset| asset.id == asset_id && asset.kind == "video" && !asset.deleted)
            .ok_or_else(|| invalid("Video not found in this Room"))?;
        let index = self
            .state
            .video_indexes
            .iter()
            .find(|index| {
                index.room_id == room_id
                    && index.asset_id == asset_id
                    && index.asset_sha256 == asset.sha256
            })
            .ok_or_else(|| invalid("Index speech on this device before exporting subtitles"))?;
        let (webvtt, cues) = crate::video_index::speech_webvtt(index);
        let cue_count = cues.len();
        Ok(json!({"asset_id":asset_id,"webvtt":webvtt,"cue_count":cue_count,"cues":cues}))
    }

    fn clear_video_index(&mut self, request: &Value) -> Result<Value> {
        let room_id = text(request, "room_id")?;
        self.room(room_id)?;
        let asset_id = request["asset_id"].as_str();
        let before = self.state.video_indexes.len();
        self.state.video_indexes.retain(|index| {
            index.room_id != room_id || asset_id.is_some_and(|asset| index.asset_id != asset)
        });
        Ok(json!({"removed":before - self.state.video_indexes.len()}))
    }

    fn sync_source(&mut self, source_id: &str) -> Result<Value> {
        if !self.refresh_access(source_id)? {
            return Ok(json!({"active":false,"merged":0}));
        }
        let source = self.source(source_id)?;
        if source.paused {
            return Err(invalid("Storage source is paused"));
        }
        let prefix = format!("{}-", source.room_id);
        let mut merged = 0;
        for name in crate::providers::list(&source)? {
            if name.starts_with(&prefix) && name.ends_with(".frindex") {
                let temp = tempfile::NamedTempFile::new_in(&self.root)?;
                crate::providers::download(&source, &name, temp.path())?;
                let bytes = fs::read(temp.path())?;
                if bytes.len() > 32 * 1024 * 1024 {
                    return Err(invalid("Room index is too large"));
                }
                merged += self.merge_packet(&source.room_id, &bytes)?;
            }
        }
        {
            let temp = tempfile::NamedTempFile::new_in(&self.root)?;
            fs::write(temp.path(), self.packet(&source.room_id)?)?;
            let size = fs::metadata(temp.path())?.len();
            let mut job = TransferJob {
                id: id(),
                room_id: source.room_id.clone(),
                asset_id: String::new(),
                source_id: source_id.into(),
                state: "transferring".into(),
                transferred: 0,
                total: size,
                error: None,
                session: None,
            };
            let filename = format!("{}-{}.frindex", source.room_id, self.device()?);
            while job.transferred < job.total {
                crate::providers::upload_step(&source, &filename, temp.path(), &mut job)?;
            }
        }
        Ok(json!({"merged":merged}))
    }
    fn pin_original(&mut self, room_id: &str, asset_id: &str) -> Result<Value> {
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let asset = self
            .assets(room_id)?
            .into_iter()
            .find(|asset| asset.id == asset_id && !asset.deleted)
            .ok_or_else(|| invalid("Media not found"))?;
        let original_path = self.blob(room_id, asset_id)?;
        let local_original = original_path.is_file()
            && !self.state.damaged_originals.contains(&asset.id)
            && crypto::verify_file(
                &crypto::derive(
                    &access::key(self.room(room_id)?, asset.key_epoch)?,
                    b"media",
                ),
                &original_path,
                &format!("{room_id}:{asset_id}"),
                &asset.sha256,
            )
            .is_ok_and(|size| size == asset.size);
        if original_path.is_file()
            && !local_original
            && !self.state.damaged_originals.iter().any(|id| id == asset_id)
        {
            self.state.damaged_originals.push(asset_id.into());
        }
        if !local_original {
            self.fetch(room_id, asset_id)?;
        }
        if !self
            .state
            .offline_pins
            .iter()
            .any(|pin| pin.room_id == room_id && pin.asset_id == asset_id)
        {
            self.state.offline_pins.push(crate::model::OfflinePin {
                room_id: room_id.into(),
                asset_id: asset_id.into(),
            });
        }
        Ok(json!({"pinned_offline":true,"available":true}))
    }

    fn unpin_original(&mut self, room_id: &str, asset_id: &str) -> Result<Value> {
        self.state
            .offline_pins
            .retain(|pin| pin.room_id != room_id || pin.asset_id != asset_id);
        Ok(json!({"pinned_offline":false}))
    }

    fn evict_original(&mut self, room_id: &str, asset_id: &str) -> Result<Value> {
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        if self
            .state
            .offline_pins
            .iter()
            .any(|pin| pin.room_id == room_id && pin.asset_id == asset_id)
        {
            return Err(invalid("Unpin this original before freeing device storage"));
        }
        let asset = self
            .assets(room_id)?
            .into_iter()
            .find(|asset| asset.id == asset_id)
            .ok_or_else(|| invalid("Media not found"))?;
        let destination = self.blob(room_id, asset_id)?;
        if !destination.is_file() {
            return Ok(json!({"evicted":false,"already_local":false}));
        }
        if self.state.damaged_originals.contains(&asset.id) {
            return Err(invalid(
                "Local original needs repair before it can be evicted",
            ));
        }

        let media_key = crypto::derive(
            &access::key(self.room(room_id)?, asset.key_epoch)?,
            b"media",
        );
        let local_size = crypto::verify_file(
            &media_key,
            &destination,
            &format!("{room_id}:{asset_id}"),
            &asset.sha256,
        );
        if !local_size.is_ok_and(|size| size == asset.size) {
            self.state.damaged_originals.push(asset.id.clone());
            return Err(invalid("Local original failed integrity verification"));
        }

        let candidates: Vec<_> = self
            .state
            .replicas
            .iter()
            .filter(|replica| replica.asset_id == asset_id && replica.verified)
            .filter_map(|replica| {
                self.state
                    .sources
                    .iter()
                    .find(|source| source.id == replica.source_id && source.room_id == room_id)
                    .cloned()
            })
            .collect();
        if candidates.is_empty() {
            return Err(invalid("Cannot evict the only verified original"));
        }

        let parent = destination
            .parent()
            .ok_or_else(|| invalid("Missing media directory"))?;
        let mut errors = Vec::new();
        for source in candidates {
            if source.paused {
                errors.push(format!("{} is paused", source.name));
                continue;
            }
            if let Some(reason) = self.network_wait_reason(&source, "Eviction") {
                errors.push(reason);
                continue;
            }
            let temporary = tempfile::NamedTempFile::new_in(parent)?;
            if crate::providers::download(
                &source,
                &format!("{room_id}-{asset_id}.frblob"),
                temporary.path(),
            )
            .is_err()
            {
                errors.push(format!("Could not verify the copy in {}", source.name));
                continue;
            }
            let verified = crypto::verify_file(
                &media_key,
                temporary.path(),
                &format!("{room_id}:{asset_id}"),
                &asset.sha256,
            );
            if !verified.is_ok_and(|size| size == asset.size) {
                for replica in &mut self.state.replicas {
                    if replica.asset_id == asset_id && replica.source_id == source.id {
                        replica.verified = false;
                    }
                }
                errors.push(format!(
                    "The copy in {} failed integrity verification",
                    source.name
                ));
                continue;
            }
            fs::remove_file(&destination)?;
            self.state.damaged_originals.retain(|id| id != asset_id);
            return Ok(json!({"evicted":true,"verified_source_id":source.id}));
        }
        Err(invalid(if errors.is_empty() {
            "No verified storage copy is available".into()
        } else {
            errors.join("; ")
        }))
    }

    fn free_device_storage(&mut self, room_id: &str, requested_bytes: u64) -> Result<Value> {
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }

        let local_assets: Vec<_> = self
            .assets(room_id)?
            .into_iter()
            .filter_map(|asset| {
                let path = self.blob(room_id, &asset.id).ok()?;
                let bytes = fs::metadata(path)
                    .ok()
                    .filter(|metadata| metadata.is_file())?
                    .len();
                Some((asset, bytes))
            })
            .collect();
        let local_bytes = local_assets
            .iter()
            .map(|(_, bytes)| *bytes)
            .fold(0_u64, u64::saturating_add);

        let mut candidates: Vec<_> = local_assets
            .iter()
            .filter(|(asset, _)| {
                !self.state.damaged_originals.contains(&asset.id)
                    && !self
                        .state
                        .offline_pins
                        .iter()
                        .any(|pin| pin.room_id == room_id && pin.asset_id == asset.id)
                    && self.state.replicas.iter().any(|replica| {
                        replica.asset_id == asset.id
                            && replica.verified
                            && self.state.sources.iter().any(|source| {
                                source.id == replica.source_id && source.room_id == room_id
                            })
                    })
            })
            .map(|(asset, bytes)| (asset.clone(), *bytes))
            .collect();
        // Free trash and older, non-favorite memories first. Pinned originals
        // are never candidates; every remaining copy is verified again below.
        candidates.sort_by(|(left, _), (right, _)| {
            let left_deleted = !left.deleted;
            let right_deleted = !right.deleted;
            left_deleted
                .cmp(&right_deleted)
                .then_with(|| left.favorite.cmp(&right.favorite))
                .then_with(|| left.captured_at.cmp(&right.captured_at))
                .then_with(|| right.size.cmp(&left.size))
                .then_with(|| left.id.cmp(&right.id))
        });

        let pinned_local = local_assets
            .iter()
            .filter(|(asset, _)| {
                self.state
                    .offline_pins
                    .iter()
                    .any(|pin| pin.room_id == room_id && pin.asset_id == asset.id)
            })
            .count();
        let mut bytes_freed = 0_u64;
        let mut evicted_asset_ids = Vec::new();
        let mut errors = Vec::new();
        for (asset, local_size) in candidates {
            if bytes_freed >= requested_bytes {
                break;
            }
            match self.evict_original(room_id, &asset.id) {
                Ok(result) if result["evicted"] == true => {
                    bytes_freed = bytes_freed.saturating_add(local_size);
                    evicted_asset_ids.push(asset.id);
                }
                Ok(_) => {}
                Err(error) => errors.push(json!({
                    "asset_id":asset.id,
                    "message":error.to_string()
                })),
            }
        }

        let bytes_after = local_bytes.saturating_sub(bytes_freed);
        Ok(json!({
            "requested_bytes":requested_bytes,
            "bytes_freed":bytes_freed,
            "local_bytes_before":local_bytes,
            "local_bytes_after":bytes_after,
            "complete":bytes_freed >= requested_bytes,
            "evicted_asset_ids":evicted_asset_ids,
            "pinned_originals_preserved":pinned_local,
            "errors":errors
        }))
    }

    fn set_device_cache_limit(&mut self, room_id: &str, max_bytes: Option<u64>) -> Result<Value> {
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        if let Some(max_bytes) = max_bytes {
            if let Some(policy) = self
                .state
                .device_cache_limits
                .iter_mut()
                .find(|policy| policy.room_id == room_id)
            {
                policy.max_bytes = max_bytes;
            } else {
                self.state
                    .device_cache_limits
                    .push(crate::model::DeviceCacheLimit {
                        room_id: room_id.into(),
                        max_bytes,
                    });
            }
        } else {
            self.state
                .device_cache_limits
                .retain(|policy| policy.room_id != room_id);
        }
        let cleanup = self.enforce_device_cache_limit(room_id)?;
        Ok(json!({"enabled":max_bytes.is_some(),"max_bytes":max_bytes,"cleanup":cleanup}))
    }

    fn enforce_device_cache_limit(&mut self, room_id: &str) -> Result<Value> {
        let Some(max_bytes) = self
            .state
            .device_cache_limits
            .iter()
            .find(|policy| policy.room_id == room_id)
            .map(|policy| policy.max_bytes)
        else {
            return Ok(json!({"enabled":false,"complete":true,"bytes_freed":0}));
        };
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Ok(
                json!({"enabled":true,"limit_bytes":max_bytes,"complete":false,"deferred":true,"reason":"Room access is unavailable"}),
            );
        }
        let local_bytes = self
            .assets(room_id)?
            .iter()
            .filter_map(|asset| {
                let path = self.blob(room_id, &asset.id).ok()?;
                fs::metadata(path)
                    .ok()
                    .filter(|metadata| metadata.is_file())
                    .map(|metadata| metadata.len())
            })
            .fold(0_u64, u64::saturating_add);
        if local_bytes <= max_bytes {
            return Ok(json!({
                "enabled":true,
                "limit_bytes":max_bytes,
                "local_bytes_before":local_bytes,
                "local_bytes_after":local_bytes,
                "requested_bytes":0,
                "bytes_freed":0,
                "complete":true,
                "deferred":false
            }));
        }
        let mut result = self.free_device_storage(room_id, local_bytes - max_bytes)?;
        result["enabled"] = json!(true);
        result["limit_bytes"] = json!(max_bytes);
        result["deferred"] = json!(false);
        Ok(result)
    }

    fn fetch(&mut self, room_id: &str, asset_id: &str) -> Result<Value> {
        let room = self.room(room_id)?;
        if !access::allowed_device(room, &self.device()?) {
            return Err(CoreError::AccessDenied);
        }
        let dest = self.blob(room_id, asset_id)?;
        let asset = self
            .assets(room_id)?
            .into_iter()
            .find(|a| a.id == asset_id)
            .ok_or_else(|| invalid("Media not found"))?;
        let parent = dest.parent().ok_or_else(|| invalid("Missing directory"))?;
        fs::create_dir_all(parent)?;
        let sources: Vec<_> = self
            .state
            .sources
            .iter()
            .filter(|s| s.room_id == room_id && !s.paused)
            .cloned()
            .collect();
        let mut network_waiting = None;
        for source in sources {
            if let Some(reason) = self.network_wait_reason(&source, "Download") {
                network_waiting = Some(reason);
                continue;
            }
            let tmp = tempfile::NamedTempFile::new_in(parent)?;
            if crate::providers::download(
                &source,
                &format!("{room_id}-{asset_id}.frblob"),
                tmp.path(),
            )
            .is_ok()
            {
                let verified = crypto::verify_file(
                    &crypto::derive(
                        &access::key(self.room(room_id)?, asset.key_epoch)?,
                        b"media",
                    ),
                    tmp.path(),
                    &format!("{room_id}:{asset_id}"),
                    &asset.sha256,
                );
                if !verified.is_ok_and(|size| size == asset.size) {
                    for replica in &mut self.state.replicas {
                        if replica.asset_id == asset_id && replica.source_id == source.id {
                            replica.verified = false;
                        }
                    }
                    continue;
                }
                tmp.persist(&dest).map_err(|e| CoreError::from(e.error))?;
                self.state.damaged_originals.retain(|id| id != asset_id);
                return Ok(json!({"available":true}));
            }
        }
        Err(invalid(network_waiting.unwrap_or_else(|| {
            "Waiting for an available original".into()
        })))
    }
    fn import_photo(&mut self, c: &Value) -> Result<Value> {
        let watch_id = text(c, "id")?;
        let room_id = text(c, "room_id")?;
        let Some(rule) = self
            .state
            .watches
            .iter()
            .find(|w| w.id == watch_id && w.path == "photos://library" && w.room_id == room_id)
            .cloned()
        else {
            return Ok(json!({"imported":false,"reason":"Source or destination changed"}));
        };
        if rule.paused {
            return Ok(json!({"imported":false,"reason":"Automatic uploads paused"}));
        }
        if self.room(room_id)?.grant.role == Role::Viewer
            || !access::allowed_device(self.room(room_id)?, &self.device()?)
        {
            return Err(CoreError::AccessDenied);
        }
        let identifier = text(c, "identifier")?;
        if identifier.is_empty() || identifier.len() > 4096 {
            return Err(invalid("Invalid Photos identifier"));
        }
        if rule.seen.iter().any(|s| s == identifier) {
            return Ok(json!({"imported":false,"reason":"Already shared"}));
        }
        let paths: Vec<PathBuf> = serde_json::from_value(c["paths"].clone())?;
        if paths.is_empty() || paths.len() > 32 {
            return Err(invalid("Choose the photo and its original components"));
        }
        if paths.iter().any(|path| {
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            rule.excluded_extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(extension))
        }) {
            // A Photos item is one contribution, including its Live Photo
            // components. Do not acknowledge or partially publish excluded items.
            return Ok(json!({"imported":false,"reason":"Excluded by source settings"}));
        }
        let captured = c["captured_at"].as_i64();
        let mut imported = vec![];
        for path in paths {
            let result = self.import_file(room_id, &path, captured)?;
            let asset_id = text(&result, "id")?.to_owned();
            if !imported.contains(&asset_id) {
                imported.push(asset_id);
            }
        }
        if imported.is_empty() {
            return Ok(json!({"imported":false,"reason":"Excluded by source settings"}));
        }
        if imported.len() > 1 {
            self.dispatch(json!({"action":"patch_asset","room_id":room_id,
                "asset_id":imported[0],"fields":{"components":imported[1..]}}))?;
        }
        // Components, sharing and the source checkpoint commit together. A failed
        // batch leaves the identifier unseen so it can safely retry.
        self.dispatch(json!({"action":"acknowledge_photo","id":watch_id,"identifier":identifier}))?;
        Ok(json!({"imported":true,"asset_ids":imported}))
    }

    fn folder_files(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut result = vec![];
        for e in fs::read_dir(path)? {
            let e = e?;
            if e.file_type()?.is_file() {
                result.push(e.path());
            }
        }
        Ok(result)
    }
    fn fingerprint(&self, path: &Path) -> Result<String> {
        let meta = fs::metadata(path)?;
        Ok(format!(
            "{}:{}:{}",
            path.to_string_lossy(),
            meta.len(),
            meta.modified()?
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }
    fn scan_watches(&mut self) -> Result<Value> {
        let mut imported = 0;
        let mut errors = vec![];
        for index in 0..self.state.watches.len() {
            let rule = self.state.watches[index].clone();
            if rule.paused
                || rule.path.starts_with("photos://")
                || rule.path.starts_with("content://")
            {
                continue;
            }
            let files = match self.folder_files(Path::new(&rule.path)) {
                Ok(f) => f,
                Err(e) => {
                    errors.push(e.to_string());
                    continue;
                }
            };
            for path in files {
                let ext = path
                    .extension()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if ![
                    "jpg", "jpeg", "png", "heic", "heif", "mov", "mp4", "m4v", "webm", "dng",
                    "gif", "tiff", "webp", "avif",
                ]
                .contains(&ext.as_str())
                    || rule
                        .excluded_extensions
                        .iter()
                        .any(|e| e.eq_ignore_ascii_case(&ext))
                {
                    continue;
                }
                let fingerprint = self.fingerprint(&path)?;
                if rule.seen.contains(&fingerprint) {
                    continue;
                }
                let recent = path
                    .metadata()?
                    .modified()?
                    .elapsed()
                    .map(|d| d.as_secs() < 2)
                    .unwrap_or(true);
                if recent {
                    continue;
                }
                match self.import_file(&rule.room_id, &path, None) {
                    Ok(value) => {
                        imported += usize::from(value["duplicate"] != true);
                        self.state.watches[index].seen.push(fingerprint);
                    }
                    Err(e) => errors.push(e.to_string()),
                }
            }
        }
        Ok(json!({"imported":imported,"errors":errors}))
    }
}
