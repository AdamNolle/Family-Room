use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: u32 = 1;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grant {
    pub room_id: String,
    pub device_id: String,
    pub role: Role,
    pub signature: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Admin,
    Contributor,
    Viewer,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub clock: u64,
    pub author: String,
    pub room_id: String,
    pub grant: Grant,
    pub kind: String,
    pub entity_id: String,
    pub value: Value,
    pub signature: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub secret: String,
    pub grant: Membership,
    pub operations: Vec<Operation>,
    #[serde(default)]
    pub album_only: bool,
    #[serde(default)]
    pub issued_grants: BTreeMap<String, Grant>,
    #[serde(default)]
    pub access_changes: Vec<AccessChange>,
    #[serde(default)]
    pub epoch_secrets: BTreeMap<u64, String>,
    #[serde(default)]
    pub rejected_operations: Vec<Operation>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessChange {
    pub room_id: String,
    pub owner_id: String,
    pub revision: u64,
    pub epoch: u64,
    pub previous: String,
    pub members: BTreeMap<String, Grant>,
    pub accepted_operations: BTreeMap<String, String>,
    pub envelopes: BTreeMap<String, String>,
    pub signature: String,
}
/// Kept in the owning device's encrypted catalog, never in a Room journal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlbumShare {
    pub room_id: String,
    pub album_id: String,
    pub scope_id: String,
    /// Initial publication operations: a new viewer must never inherit later history.
    #[serde(default)]
    pub snapshot_operations: BTreeSet<String>,
}
pub type Membership = Grant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Moment {
    pub id: String,
    pub room_id: String,
    pub title: String,
    pub start: i64,
    pub end: i64,
    pub asset_ids: Vec<String>,
    pub featured: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: String,
    pub room_id: String,
    pub filename: String,
    pub kind: String,
    pub sha256: String,
    pub size: u64,
    pub captured_at: i64,
    pub contributor: String,
    pub favorite: bool,
    pub caption: String,
    pub deleted: bool,
    pub people: Vec<String>,
    #[serde(default)]
    pub components: Vec<String>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub key_epoch: u64,
}
fn is_zero(value: &u64) -> bool {
    *value == 0
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub room_id: String,
    pub title: String,
    pub asset_ids: Vec<String>,
    pub pinned: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonTag {
    pub id: String,
    pub room_id: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f64,
    pub value: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub asset_id: String,
    pub track: u32,
    pub start: f64,
    pub trim_in: f64,
    pub duration: f64,
    pub speed: f64,
    pub volume: f64,
    pub opacity: f64,
    pub exposure: f64,
    pub saturation: f64,
    pub title: String,
    pub fade_in: f64,
    pub fade_out: f64,
    #[serde(default)]
    pub fade_in_offset: f64,
    #[serde(default)]
    pub fade_out_offset: f64,
    pub opacity_keyframes: Vec<Keyframe>,
    pub volume_keyframes: Vec<Keyframe>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoryProject {
    #[serde(default = "crate::timeline::default_format_version")]
    pub format_version: u32,
    pub id: String,
    pub room_id: String,
    pub title: String,
    pub version_id: String,
    pub editor: String,
    pub width: u32,
    pub height: u32,
    pub clips: Vec<Clip>,
    pub chapters: Vec<Value>,
    pub published_asset: Option<String>,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub auto_generated: bool,
    #[serde(default)]
    pub archived: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageSource {
    pub id: String,
    pub room_id: String,
    pub name: String,
    pub kind: String,
    pub endpoint: String,
    pub token: String,
    pub paused: bool,
    #[serde(default)]
    pub preferred: bool,
    pub budget: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchRule {
    pub id: String,
    pub room_id: String,
    pub path: String,
    pub paused: bool,
    pub excluded_extensions: Vec<String>,
    pub seen: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferJob {
    pub id: String,
    pub room_id: String,
    pub asset_id: String,
    pub source_id: String,
    pub state: String,
    pub transferred: u64,
    pub total: u64,
    pub error: Option<String>,
    pub session: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Replica {
    pub asset_id: String,
    pub source_id: String,
    pub verified: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub version: u32,
    pub signing_seed: String,
    pub rooms: Vec<Room>,
    pub sources: Vec<StorageSource>,
    pub watches: Vec<WatchRule>,
    pub transfers: Vec<TransferJob>,
    pub replicas: Vec<Replica>,
    #[serde(default)]
    pub drafts: Vec<StoryProject>,
    #[serde(default)]
    pub album_shares: Vec<AlbumShare>,
    /// Originals that failed an authenticated read; repaired imports/fetches clear these.
    #[serde(default)]
    pub damaged_originals: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invitation {
    pub version: u32,
    pub room_id: String,
    pub name: String,
    pub owner_id: String,
    pub secret: String,
    pub grant: Grant,
    #[serde(default)]
    pub album_only: bool,
    #[serde(default)]
    pub access_changes: Vec<AccessChange>,
    #[serde(default)]
    pub epoch_secrets: BTreeMap<u64, String>,
}
#[derive(Serialize, Deserialize)]
pub struct InvitationEnvelope {
    pub version: u32,
    pub owner_id: String,
    pub device_id: String,
    pub ciphertext: String,
    pub signature: String,
}
