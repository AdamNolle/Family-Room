use crate::{
    access, crypto,
    error::{invalid, CoreError, Result},
    model::*,
    signing,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
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
                drafts: vec![],
                album_shares: vec![],
                damaged_originals: vec![],
            },
            Err(e) => return Err(e.into()),
        };
        let engine = Self {
            root,
            master,
            state,
            _lock: lock,
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
            signing::verify_grant(&r.grant, &r.owner_id)?;
            access::verify_chain(&r.access_changes, &r.id, &r.owner_id)?;
            for value in r.epoch_secrets.values() {
                crypto::parse_key(value)?;
            }
            for op in &r.operations {
                signing::verify_operation(op, &r.owner_id, &r.id)?;
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
                "album" | "album_member" | "moment" | "person" | "comment" | "reaction"
                | "story" | "settings" => {
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
    pub fn assets(&self, room_id: &str) -> Result<Vec<MediaAsset>> {
        self.entities(room_id)?
            .into_iter()
            .filter(|(k, _)| k.starts_with("asset:"))
            .map(|(_, v)| serde_json::from_value(v).map_err(CoreError::from))
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
        for r in &self.state.rooms {
            let entities = self.entities(&r.id)?;
            let collect = |prefix: &str| {
                entities
                    .iter()
                    .filter(|(k, _)| k.starts_with(prefix))
                    .map(|(_, v)| v.clone())
                    .collect::<Vec<_>>()
            };
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
                    value["verified_copies"] = json!(copies);
                    value["availability"] = json!(if damaged {
                        "Local original needs repair"
                    } else if local {
                        "Saved on this device"
                    } else if copies > 0 {
                        "Original available from storage"
                    } else {
                        "Waiting for an original"
                    });
                    value
                })
                .collect::<Vec<_>>();
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
            rooms.push(json!({"id":r.id,"name":r.name,"owner_id":r.owner_id,"role":if access::allowed_device(r,&self.device()?) { serde_json::to_value(r.grant.role)? } else { json!("removed") },"album_only":r.album_only,"access_revision":access::revision(r),"key_epoch":access::epoch(r),"rejected_edits":r.rejected_operations.len(),"assets":assets,"moments":collect("moment:"),"albums":collect("album:"),"people":collect("person:"),"stories":stories,"comments":collect("comment:"),"members":members.values().collect::<Vec<_>>()}));
        }
        let sources=self.state.sources.iter().map(|s|json!({"id":s.id,"room_id":s.room_id,"name":s.name,"kind":s.kind,"endpoint":s.endpoint,"paused":s.paused,"preferred":self.is_preferred(s),"budget":s.budget,"capabilities":crate::providers::capabilities(&s.kind)})).collect::<Vec<_>>();
        let transfers=self.state.transfers.iter().map(|t|json!({"id":t.id,"room_id":t.room_id,"asset_id":t.asset_id,"source_id":t.source_id,"state":t.state,"transferred":t.transferred,"total":t.total,"error":t.error})).collect::<Vec<_>>();
        Ok(
            json!({"version":VERSION,"timeline_formats":[crate::timeline::FORMAT_VERSION],"device_id":self.device()?,"rooms":rooms,"sources":sources,"watches":self.state.watches,"transfers":transfers}),
        )
    }
    pub fn execute(&mut self, command: Value) -> Result<Value> {
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
            "revoke_device" => self.change_access(&c, true),
            "set_role" => self.change_access(&c, false),
            "rotate_keys" => self.change_access(&c, true),
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
            "patch_asset" => {
                let room_id = text(&c, "room_id")?;
                let asset_id = text(&c, "asset_id")?;
                let fields = c["fields"]
                    .as_object()
                    .ok_or_else(|| invalid("Missing fields"))?;
                let asset = self
                    .assets(room_id)?
                    .into_iter()
                    .find(|a| a.id == asset_id)
                    .ok_or_else(|| invalid("Media not found"))?;
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
                        return Err(invalid("Invalid media edit"));
                    }
                    if key == "deleted"
                        && asset.contributor != self.device()?
                        && !matches!(self.room(room_id)?.grant.role, Role::Owner | Role::Admin)
                    {
                        return Err(CoreError::AccessDenied);
                    }
                }
                self.write_op(room_id, "asset_patch", asset_id, c["fields"].clone())?;
                Ok(json!({"saved":true}))
            }
            "save_album" => {
                let room_id = text(&c, "room_id")?;
                let album_id = c["id"].as_str().map(str::to_owned).unwrap_or_else(id);
                let asset_ids: Vec<String> = serde_json::from_value(c["asset_ids"].clone())?;
                let known = self.assets(room_id)?;
                if asset_ids.iter().any(|a| !known.iter().any(|x| &x.id == a)) {
                    return Err(invalid("Album contains media from another Room"));
                }
                let previous = self
                    .entities(room_id)?
                    .get(&format!("album:{album_id}"))
                    .map(|value| serde_json::from_value::<Album>(value.clone()))
                    .transpose()?
                    .map(|album| album.asset_ids)
                    .unwrap_or_default();
                let changed: Vec<_> = previous
                    .iter()
                    .chain(asset_ids.iter())
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .filter(|id| previous.contains(id) != asset_ids.contains(id))
                    .collect();
                let album = Album {
                    id: album_id.clone(),
                    room_id: room_id.into(),
                    title: title(text(&c, "title")?)?,
                    asset_ids: asset_ids.clone(),
                    pinned: c["pinned"].as_bool().unwrap_or(false),
                };
                self.write_op(room_id, "album", &album_id, serde_json::to_value(album)?)?;
                for asset_id in changed {
                    let entity_id = format!("{album_id}-{asset_id}");
                    self.write_op(room_id, "album_member", &entity_id,
                        json!({"album_id":album_id,"asset_id":asset_id,"included":asset_ids.contains(&asset_id)}))?;
                }
                Ok(json!({"id":album_id}))
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
            "save_story" => self.save_story(&c),
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
                let history=room.operations.iter().filter(|o|o.entity_id==asset_id&&o.kind=="asset_patch"&&o.value.get("caption").is_some()).map(|o|json!({"id":o.id,"author":o.author,"caption":o.value["caption"],"clock":o.clock})).collect::<Vec<_>>();
                Ok(json!({"history":history}))
            }
            "invite" => self.issue_invitation(&c),
            "join" => {
                let bytes = URL_SAFE_NO_PAD
                    .decode(text(&c, "invitation")?.trim())
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
                access::verify_chain(&invite.access_changes, &invite.room_id, &invite.owner_id)?;
                if invite
                    .access_changes
                    .last()
                    .is_some_and(|p| !p.members.contains_key(&self.device().unwrap_or_default()))
                {
                    return Err(CoreError::AccessDenied);
                }
                for key in invite.epoch_secrets.values() {
                    crypto::parse_key(key)?;
                }
                safe_id(&invite.room_id)?;
                if self.state.rooms.iter().any(|r| r.id == invite.room_id) {
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
                Ok(json!({"id":room_id}))
            }
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
                    budget: c["budget"].as_u64(),
                };
                crate::providers::validate(&source)?;
                if kind == "gateway" && room.grant.role == Role::Owner {
                    crate::providers::register_gateway(&source)?;
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
            "prefer_source" => {
                let selected = self.source(text(&c, "id")?)?;
                for source in &mut self.state.sources {
                    if source.room_id == selected.room_id {
                        source.preferred = source.id == selected.id;
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
                let sources = self
                    .state
                    .sources
                    .iter()
                    .filter(|s| s.room_id == room_id && !s.paused)
                    .cloned()
                    .collect::<Vec<_>>();
                let mut errors = vec![];
                for source in sources {
                    if let Err(e) = self.sync_source(&source.id) {
                        errors.push(e.to_string());
                        continue;
                    }
                    // The sync may have learned a removal or a viewer downgrade.
                    let can_contribute = self.room(room_id)?.grant.role != Role::Viewer
                        && access::allowed_device(self.room(room_id)?, &self.device()?);
                    if can_contribute && self.is_preferred(&source) {
                        for asset in self.assets(room_id)?.into_iter().filter(|a| !a.deleted) {
                            if !self.blob(room_id, &asset.id)?.is_file()
                                || self.state.transfers.iter().any(|j| {
                                    j.source_id == source.id
                                        && j.asset_id == asset.id
                                        && ["complete", "cancelled"].contains(&j.state.as_str())
                                })
                            {
                                continue;
                            }
                            if let Err(e) = self.replicate(&source.id, &asset.id) {
                                errors.push(e.to_string());
                            }
                        }
                    }
                }
                Ok(json!({"errors":errors}))
            }
            "fetch" => self.fetch(text(&c, "room_id")?, text(&c, "asset_id")?),
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
            "pause_watch" => {
                let rule = self
                    .state
                    .watches
                    .iter_mut()
                    .find(|w| Some(w.id.as_str()) == c["id"].as_str())
                    .ok_or_else(|| invalid("Upload rule not found"))?;
                rule.paused = c["paused"].as_bool().unwrap_or(true);
                Ok(json!({"saved":true}))
            }
            "set_watch_exclusions" => {
                let excluded = exclusions(c.get("excluded_extensions"))?;
                let rule = self
                    .state
                    .watches
                    .iter_mut()
                    .find(|w| Some(w.id.as_str()) == c["id"].as_str())
                    .ok_or_else(|| invalid("Upload rule not found"))?;
                rule.excluded_extensions = excluded;
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
            "title":album.title,"asset_ids":asset_ids,"pinned":false}),
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
            contributor: self.device()?,
            favorite: false,
            caption: String::new(),
            deleted: false,
            people: vec![],
            components: vec![],
            key_epoch,
        };
        if !duplicate {
            self.write_op(room_id, "asset", &asset_id, serde_json::to_value(asset)?)?;
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
        self.state.drafts.push(story.clone());
        Ok(json!({"id":story.id,"version_id":story.version_id}))
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
            signing::verify_operation(&op, &room.owner_id, room_id)?;
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
                "moment",
                "person",
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
                if a.id != op.entity_id || a.room_id != op.room_id {
                    return Err(CoreError::AccessDenied);
                }
                title(&a.title)?;
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
                if op.value["author"] != op.author {
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
        if !self.state.rooms.is_empty() {
            return Err(invalid(
                "Restore into an empty library to protect existing memories",
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
        if !self.refresh_access(source_id)? {
            return Err(CoreError::AccessDenied);
        }
        let source = self.source(source_id)?;
        if source.paused {
            return Err(invalid("Storage source is paused"));
        }
        if self.room(&source.room_id)?.grant.role == Role::Viewer {
            return Err(CoreError::AccessDenied);
        }
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
        let source = self.source(source_id)?;
        if source.paused {
            return Err(invalid("Storage source is paused"));
        }
        let incoming = crate::providers::read_access(&source)?;
        let seed = self.state.signing_seed.clone();
        access::apply(self.room_mut(&source.room_id)?, incoming.clone(), &seed)?;
        let room = self.room(&source.room_id)?;
        if self.device()? == room.owner_id && room.access_changes.len() > incoming.len() {
            crate::providers::publish_access(&source, &room.access_changes)?;
        }
        let active = access::allowed_device(room, &self.device()?);
        if source.kind == "gateway" {
            let token =
                serde_json::to_string(&json!({"owner_id":room.owner_id,"grant":room.grant,
                "signing_seed":self.state.signing_seed,"access_revision":access::revision(room)}))?;
            if let Some(target) = self.state.sources.iter_mut().find(|s| s.id == source_id) {
                target.token = token;
            }
        }
        Ok(active)
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
    fn fetch(&mut self, room_id: &str, asset_id: &str) -> Result<Value> {
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
        for source in sources {
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
        Err(invalid("Waiting for an available original"))
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
            if rule.paused || rule.path.starts_with("photos://") {
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
