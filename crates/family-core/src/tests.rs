use super::*;
use crate::{engine::Engine, model::Role};
use serde_json::json;
use std::fs;
fn vault() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path(), [7; 32]).unwrap();
    (dir, engine)
}

#[test]
fn failed_room_import_retains_learned_access_changes_after_catalog_reopen() {
    for remove in [false, true] {
        let (owner_dir, mut owner) = vault();
        let (member_dir, mut member) = vault();
        let r = room(&mut owner);
        invite(&mut owner, &mut member, &r, Role::Contributor);
        let offline = import(&mut member, &r, &media(&member_dir));
        owner
            .execute(
                json!({"action":if remove {"revoke_device"} else {"set_role"},
            "room_id":r,"device_id":member.device().unwrap(),"role":"viewer"}),
            )
            .unwrap();
        let source_file = owner_dir.path().join("new.jpg");
        fs::write(&source_file, b"owner original after access change").unwrap();
        let new = import(&mut owner, &r, &source_file);
        let path = owner_dir.path().join("damaged.frroom");
        let mut archive = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        archive
            .start_file("manifest.fr", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut archive, &owner.packet(&r).unwrap()).unwrap();
        archive
            .start_file(
                format!("objects/{new}.frblob"),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        std::io::Write::write_all(&mut archive, b"damaged encrypted original").unwrap();
        archive.finish().unwrap();
        assert!(member
            .execute(json!({"action":"import_room","room_id":r,"path":path}))
            .is_err());
        assert_eq!(crate::access::revision(member.room(&r).unwrap()), 1);
        drop(member);
        let mut reopened = Engine::open(member_dir.path(), [7; 32]).unwrap();
        assert_eq!(
            reopened.snapshot().unwrap()["rooms"][0]["role"],
            if remove { "removed" } else { "viewer" }
        );
        assert!(reopened
            .execute(json!({"action":"import","room_id":r,"path":source_file}))
            .is_err());
        // Permission changes survive failure, while unrelated shared mutations roll back.
        assert!(!reopened.assets(&r).unwrap().iter().any(|a| a.id == new));
        assert!(reopened.blob(&r, &offline).unwrap().is_file());
        assert_eq!(reopened.room(&r).unwrap().rejected_operations.len(), 1);
    }
}

#[test]
fn preserved_change_export_recovers_originals_projects_and_missing_file_manifest() {
    let (owner_dir, mut owner) = vault();
    let (member_dir, mut member) = vault();
    let r = room(&mut owner);
    let shared = import(&mut owner, &r, &media(&owner_dir));
    invite(&mut owner, &mut member, &r, Role::Contributor);
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    let original = member_dir.path().join(r"memory\..\..\escape.jpg");
    fs::write(&original, b"unpublished local photo").unwrap();
    let local = import(&mut member, &r, &original);
    let component_path = member_dir.path().join("live-photo.mov");
    fs::write(&component_path, b"live photo video component").unwrap();
    let component = import(&mut member, &r, &component_path);
    member
        .execute(json!({"action":"patch_asset","room_id":r,"asset_id":local,
        "fields":{"caption":"My unsent caption","components":[component]}}))
        .unwrap();
    member
        .execute(json!({"action":"patch_asset","room_id":r,"asset_id":shared,
        "fields":{"caption":"Unsent caption for an uncached original"}}))
        .unwrap();
    let project = json!({"id":engine::id(),"room_id":r,"title":"Private film",
        "version_id":"","editor":"","width":1920,"height":1080,
        "clips":[{"id":engine::id(),"asset_id":local,"track":0,"start":0,
        "trim_in":0,"duration":3,"speed":1,"volume":1,"opacity":1,
        "exposure":0,"saturation":1,"title":"Saved title","fade_in":0,"fade_out":0,
        "opacity_keyframes":[],"volume_keyframes":[]}],"chapters":[],"published_asset":null});
    member
        .execute(json!({"action":"save_story","project":project}))
        .unwrap();
    owner
        .execute(json!({"action":"revoke_device","room_id":r,
        "device_id":member.device().unwrap()}))
        .unwrap();
    member
        .execute(json!({"action":"apply_access","room_id":r,
        "changes":owner.room(&r).unwrap().access_changes}))
        .unwrap();
    let before = serde_json::to_value(&member.state).unwrap();
    let path = member_dir.path().join("preserved.zip");
    let result = member
        .execute(json!({"action":"export_preserved","room_id":r,"path":path}))
        .unwrap();
    assert_eq!(result["exported"], 2);
    assert_eq!(result["missing"], 1);
    assert_eq!(result["projects"], 1);
    assert_eq!(serde_json::to_value(&member.state).unwrap(), before);
    let exported = fs::read(&path).unwrap();
    let mut archive = zip::ZipArchive::new(fs::File::open(&path).unwrap()).unwrap();
    assert_eq!(archive.len(), 4);
    let mut manifest = String::new();
    std::io::Read::read_to_string(
        &mut archive.by_name("preserved-changes.json").unwrap(),
        &mut manifest,
    )
    .unwrap();
    assert!(!manifest.contains(&member.room(&r).unwrap().secret));
    assert!(!manifest.contains(&member.state.signing_seed));
    let metadata: serde_json::Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(metadata["shared"], false);
    assert_eq!(metadata["missing_original_ids"], json!([shared]));
    assert!(metadata["operations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op["value"]["caption"] == "My unsent caption"));
    assert_eq!(metadata["projects"][0]["clips"][0]["title"], "Saved title");
    let asset = metadata["originals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == local)
        .unwrap();
    assert_eq!(
        asset["export_file"],
        format!("originals/{local}-memory_.._.._escape.jpg")
    );
    let mut bytes = vec![];
    std::io::Read::read_to_end(
        &mut archive
            .by_name(asset["export_file"].as_str().unwrap())
            .unwrap(),
        &mut bytes,
    )
    .unwrap();
    assert_eq!(bytes, fs::read(&original).unwrap());
    assert!(member
        .execute(json!({"action":"export_preserved","room_id":r,"path":path}))
        .is_err());
    assert_eq!(fs::read(&path).unwrap(), exported);
    // The explicit export never republishes the quarantined work.
    assert!(!member.assets(&r).unwrap().iter().any(|a| a.id == local));
    let mut encrypted = fs::read(member.blob(&r, &local).unwrap()).unwrap();
    let last = encrypted.len() - 1;
    encrypted[last] ^= 1;
    fs::write(member.blob(&r, &local).unwrap(), encrypted).unwrap();
    let corrupt = member_dir.path().join("corrupt-export.zip");
    assert!(member
        .execute(json!({"action":"export_preserved","room_id":r,"path":corrupt}))
        .is_err());
    assert!(!corrupt.exists());
    assert_eq!(member.room(&r).unwrap().rejected_operations.len(), 4);
}

#[test]
fn removed_device_backup_preserves_quarantined_local_originals() {
    let (owner_dir, mut owner) = vault();
    let (member_dir, mut member) = vault();
    let r = room(&mut owner);
    invite(&mut owner, &mut member, &r, Role::Contributor);
    let original = media(&member_dir);
    let a = import(&mut member, &r, &original);
    owner
        .execute(json!({"action":"revoke_device","room_id":r,"device_id":member.device().unwrap()}))
        .unwrap();
    member
        .execute(json!({"action":"apply_access","room_id":r,
        "changes":owner.room(&r).unwrap().access_changes}))
        .unwrap();
    assert!(member.assets(&r).unwrap().is_empty());
    assert!(member.blob(&r, &a).unwrap().is_file());
    let backup = owner_dir.path().join("removed-device.frbackup");
    member
        .execute(json!({"action":"backup","path":backup}))
        .unwrap();
    let (_restored_dir, mut restored) = vault();
    restored
        .execute(json!({"action":"restore","path":backup,"recovery_key":hex::encode([7;32])}))
        .unwrap();
    assert!(restored.blob(&r, &a).unwrap().is_file());
    assert_eq!(restored.room(&r).unwrap().rejected_operations.len(), 1);
    let output = member_dir.path().join("backup-original.jpg");
    crypto::decrypt_file(
        &crypto::derive(
            &crate::access::key(restored.room(&r).unwrap(), 0).unwrap(),
            b"media",
        ),
        &restored.blob(&r, &a).unwrap(),
        &output,
        &format!("{r}:{a}"),
        &crypto::digest(&original).unwrap().0,
    )
    .unwrap();
    assert_eq!(fs::read(output).unwrap(), fs::read(original).unwrap());
}

#[cfg(feature = "gateway")]
#[test]
fn gateway_enforces_revocation_and_restart_preserves_access_policy() {
    let directory = tempfile::tempdir().unwrap();
    fn server(
        path: &std::path::Path,
    ) -> (
        String,
        tokio::sync::oneshot::Sender<()>,
        std::thread::JoinHandle<()>,
    ) {
        let app = crate::gateway::router(path.to_owned(), None).unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let (shutdown, stop) = tokio::sync::oneshot::channel();
        let thread = std::thread::spawn(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                    sender.send(listener.local_addr().unwrap()).unwrap();
                    axum::serve(listener, app)
                        .with_graceful_shutdown(async {
                            let _ = stop.await;
                        })
                        .await
                        .unwrap();
                });
        });
        (
            format!("http://{}", receiver.recv().unwrap()),
            shutdown,
            thread,
        )
    }
    let (endpoint, shutdown, thread) = server(directory.path());
    let (owner_dir, mut owner) = vault();
    let (_member_dir, mut member) = vault();
    let (_removed_dir, mut removed) = vault();
    let r = room(&mut owner);
    invite(&mut owner, &mut member, &r, Role::Contributor);
    invite(&mut owner, &mut removed, &r, Role::Contributor);
    let source = |core: &mut Engine| -> String {
        core.execute(json!({"action":"add_source","room_id":r,"name":"Gateway","kind":"gateway","endpoint":endpoint}))
            .unwrap()["id"].as_str().unwrap().to_string()
    };
    let owner_source = source(&mut owner);
    let member_source = source(&mut member);
    let removed_source = source(&mut removed);
    let stale_configuration = removed.state.sources[0].clone();
    let old = import(&mut owner, &r, &media(&owner_dir));
    owner
        .execute(json!({"action":"replicate","source_id":owner_source,"asset_id":old}))
        .unwrap();
    owner
        .execute(json!({"action":"sync_source","id":owner_source}))
        .unwrap();
    removed
        .execute(json!({"action":"sync_source","id":removed_source}))
        .unwrap();
    member
        .execute(json!({"action":"sync_source","id":member_source}))
        .unwrap();
    assert_eq!(removed.assets(&r).unwrap().len(), 1);
    removed
        .execute(json!({"action":"fetch","room_id":r,"asset_id":old}))
        .unwrap();
    owner
        .execute(
            json!({"action":"revoke_device","room_id":r,"device_id":removed.device().unwrap()}),
        )
        .unwrap();
    let new_file = owner_dir.path().join("future.jpg");
    fs::write(&new_file, b"new private memory").unwrap();
    let new = import(&mut owner, &r, &new_file);
    owner
        .execute(json!({"action":"replicate","source_id":owner_source,"asset_id":new}))
        .unwrap();
    owner
        .execute(json!({"action":"sync_source","id":owner_source}))
        .unwrap();
    // The server rejects the old credential even before the removed client syncs.
    assert!(crate::providers::list(&stale_configuration).is_err());
    let synced = removed
        .execute(json!({"action":"sync_all","room_id":r}))
        .unwrap();
    assert!(synced["errors"].as_array().unwrap().is_empty());
    assert!(removed.state.transfers.is_empty());
    let active = removed
        .execute(json!({"action":"sync_source","id":removed_source}))
        .unwrap();
    assert_eq!(active["active"], false);
    assert_eq!(removed.snapshot().unwrap()["rooms"][0]["role"], "removed");
    assert!(removed
        .execute(json!({"action":"comment","room_id":r,"asset_id":old,"body":"revoked comment"}))
        .is_err());
    member
        .execute(json!({"action":"sync_source","id":member_source}))
        .unwrap();
    member
        .execute(json!({"action":"fetch","room_id":r,"asset_id":new}))
        .unwrap();
    let restored = owner_dir.path().join("fetched.jpg");
    member
        .execute(json!({"action":"materialize","room_id":r,"asset_id":new,"destination":restored}))
        .unwrap();
    assert_eq!(fs::read(restored).unwrap(), b"new private memory");
    // The gateway stores only recipient-sealed keys and signed permission data.
    let policy = fs::read_to_string(directory.path().join("access.json")).unwrap();
    assert!(!policy.contains(owner.room(&r).unwrap().epoch_secrets.get(&1).unwrap()));
    shutdown.send(()).unwrap();
    thread.join().unwrap();
    let (new_endpoint, shutdown, thread) = server(directory.path());
    let mut stale = stale_configuration.clone();
    stale.endpoint = new_endpoint;
    assert!(crate::providers::list(&stale).is_err());
    shutdown.send(()).unwrap();
    thread.join().unwrap();
}

#[test]
fn revocation_rotates_future_media_and_freezes_confirmed_history() {
    let (dir, mut owner) = vault();
    let (_remaining_dir, mut remaining) = vault();
    let (_removed_dir, mut removed) = vault();
    let r = room(&mut owner);
    invite(&mut owner, &mut remaining, &r, Role::Contributor);
    invite(&mut owner, &mut removed, &r, Role::Contributor);
    let old = import(&mut owner, &r, &media(&dir));
    remaining
        .merge_packet(&r, &owner.packet(&r).unwrap())
        .unwrap();
    removed
        .merge_packet(&r, &owner.packet(&r).unwrap())
        .unwrap();
    removed
        .execute(json!({"action":"patch_asset","room_id":r,"asset_id":old,
        "fields":{"caption":"Confirmed caption before removal"}}))
        .unwrap();
    owner
        .merge_packet(&r, &removed.packet(&r).unwrap())
        .unwrap();
    let removed_id = removed.device().unwrap();
    owner
        .execute(json!({"action":"revoke_device","room_id":r,"device_id":removed_id}))
        .unwrap();
    assert_eq!(crate::access::epoch(owner.room(&r).unwrap()), 1);
    let new_file = dir.path().join("new.mp4");
    fs::write(&new_file, b"future private video").unwrap();
    let new = import(&mut owner, &r, &new_file);
    let packet = owner.packet(&r).unwrap();
    remaining.merge_packet(&r, &packet).unwrap();
    assert!(removed.merge_packet(&r, &packet).is_err());
    assert!(!crate::access::allowed_device(
        removed.room(&r).unwrap(),
        &removed_id
    ));
    assert!(removed
        .execute(json!({"action":"patch_asset","room_id":r,"asset_id":old,
        "fields":{"caption":"Forged after removal"}}))
        .is_err());
    // The removed device knows the old content key but cannot open new media.
    let root_key = crypto::derive(
        &crypto::parse_key(&removed.room(&r).unwrap().secret).unwrap(),
        b"media",
    );
    assert!(crypto::decrypt_file(
        &root_key,
        &owner.blob(&r, &new).unwrap(),
        &dir.path().join("forbidden.mp4"),
        &format!("{r}:{new}"),
        &crypto::digest(&new_file).unwrap().0
    )
    .is_err());
    let package = dir.path().join("current.frroom");
    owner
        .execute(json!({"action":"export_room","room_id":r,"path":package}))
        .unwrap();
    remaining
        .execute(json!({"action":"import_room","room_id":r,"path":package}))
        .unwrap();
    for (asset, path) in [
        (old.clone(), dir.path().join("old-restored.jpg")),
        (new.clone(), dir.path().join("new-restored.mp4")),
    ] {
        remaining
            .execute(
                json!({"action":"materialize","room_id":r,"asset_id":asset,"destination":path}),
            )
            .unwrap();
    }
    assert_eq!(
        fs::read(dir.path().join("new-restored.mp4")).unwrap(),
        b"future private video"
    );
    // Re-signing an already confirmed ID does not rewrite frozen history.
    let mut forged = removed
        .room(&r)
        .unwrap()
        .operations
        .iter()
        .find(|o| o.author == removed_id && o.kind == "asset_patch")
        .unwrap()
        .clone();
    forged.value["caption"] = json!("Malicious replacement with the same ID");
    signing::sign_operation(&mut forged, &removed.state.signing_seed).unwrap();
    assert!(remaining.merge(&r, vec![forged]).is_err());
    assert!(owner
        .assets(&r)
        .unwrap()
        .iter()
        .find(|a| a.id == old)
        .unwrap()
        .caption
        .starts_with("Confirmed"));
    // A forged or conflicting policy never advances a trusted revision.
    let mut changes = owner.room(&r).unwrap().access_changes.clone();
    changes[0]
        .members
        .insert(removed_id, removed.room(&r).unwrap().grant.clone());
    assert!(remaining
        .execute(json!({"action":"apply_access","room_id":r,"changes":changes}))
        .is_err());
    assert_eq!(crate::access::revision(remaining.room(&r).unwrap()), 1);
    // Rotation preserves identity and reimport repairs an old-epoch original.
    fs::remove_file(owner.blob(&r, &old).unwrap()).unwrap();
    assert_eq!(import(&mut owner, &r, &media(&dir)), old);
    owner.execute(json!({"action":"materialize","room_id":r,"asset_id":old,"destination":dir.path().join("reimported.jpg")})).unwrap();
}
#[test]
fn access_history_survives_backup_and_offline_versions_are_quarantined() {
    let (dir, mut owner) = vault();
    let (member_dir, mut member) = vault();
    let r = room(&mut owner);
    invite(&mut owner, &mut member, &r, Role::Contributor);
    let old = import(&mut owner, &r, &media(&dir));
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    member.execute(json!({"action":"patch_asset","room_id":r,"asset_id":old,"fields":{"caption":"Unsynced offline caption"}})).unwrap();
    owner
        .execute(json!({"action":"revoke_device","room_id":r,"device_id":member.device().unwrap()}))
        .unwrap();
    let policy = owner.room(&r).unwrap().access_changes.clone();
    member
        .execute(json!({"action":"apply_access","room_id":r,"changes":policy}))
        .unwrap();
    assert!(member
        .room(&r)
        .unwrap()
        .rejected_operations
        .iter()
        .any(|o| o.value["caption"] == "Unsynced offline caption"));
    assert_eq!(member.snapshot().unwrap()["rooms"][0]["role"], "removed");
    drop(member);
    let reopened = Engine::open(member_dir.path(), [7; 32]).unwrap();
    assert_eq!(reopened.room(&r).unwrap().rejected_operations.len(), 1);
    assert!(!crate::access::allowed_device(
        reopened.room(&r).unwrap(),
        &reopened.device().unwrap()
    ));
    let backup = dir.path().join("access.frbackup");
    owner
        .execute(json!({"action":"backup","path":backup}))
        .unwrap();
    let (restore_dir, mut restored) = vault();
    restored
        .execute(json!({"action":"restore","path":backup,"recovery_key":hex::encode([7;32])}))
        .unwrap();
    assert_eq!(crate::access::revision(restored.room(&r).unwrap()), 1);
    assert_eq!(
        restored.room(&r).unwrap().epoch_secrets,
        owner.room(&r).unwrap().epoch_secrets
    );
    restored.execute(json!({"action":"materialize","room_id":r,"asset_id":old,"destination":restore_dir.path().join("old.jpg")})).unwrap();
}
#[test]
fn role_changes_require_current_grants_and_invitations_are_sealed() {
    let (_dir, mut owner) = vault();
    let (_member_dir, mut member) = vault();
    let (_later_dir, mut later) = vault();
    let r = room(&mut owner);
    invite(&mut owner, &mut member, &r, Role::Contributor);
    let before = member.room(&r).unwrap().grant.clone();
    let changed = owner.execute(json!({"action":"set_role","room_id":r,"device_id":member.device().unwrap(),"role":"viewer"})).unwrap();
    assert_eq!(changed["key_epoch"], 1);
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    assert_eq!(member.room(&r).unwrap().grant.role, Role::Viewer);
    assert_ne!(before.signature, member.room(&r).unwrap().grant.signature);
    let token = owner.execute(json!({"action":"invite","room_id":r,"device_id":later.device().unwrap(),"role":"viewer"})).unwrap()["invitation"].as_str().unwrap().to_string();
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let encoded = URL_SAFE_NO_PAD.decode(&token).unwrap();
    assert!(!String::from_utf8_lossy(&encoded).contains(&owner.room(&r).unwrap().secret));
    later
        .execute(json!({"action":"join","invitation":token}))
        .unwrap();
    later.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    assert_eq!(crate::access::revision(later.room(&r).unwrap()), 2);
    assert_eq!(crate::access::epoch(later.room(&r).unwrap()), 1);
    owner
        .execute(json!({"action":"rotate_keys","room_id":r}))
        .unwrap();
    let previous_packet = later.packet(&r).unwrap();
    later.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    owner.merge_packet(&r, &previous_packet).unwrap();
    assert_eq!(crate::access::epoch(later.room(&r).unwrap()), 2);
    // Older signed policies cannot downgrade local permissions.
    let older = owner.room(&r).unwrap().access_changes[..1].to_vec();
    later
        .execute(json!({"action":"apply_access","room_id":r,"changes":older}))
        .unwrap();
    assert_eq!(crate::access::revision(later.room(&r).unwrap()), 3);
}

fn room(engine: &mut Engine) -> String {
    engine
        .execute(json!({"action":"create_room","name":"Family"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn media(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let p = dir.path().join("memory.jpg");
    fs::write(&p, b"private-photo-original").unwrap();
    p
}
fn import(engine: &mut Engine, room: &str, path: &std::path::Path) -> String {
    engine
        .execute(json!({"action":"import","room_id":room,"path":path}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn invite(owner: &mut Engine, member: &mut Engine, room: &str, role: Role) {
    let token=owner.execute(json!({"action":"invite","room_id":room,"device_id":member.device().unwrap(),"role":role})).unwrap()["invitation"].clone();
    member
        .execute(json!({"action":"join","invitation":token}))
        .unwrap();
}
#[test]
fn album_only_invitation_has_independent_keys_and_republishes_removals() {
    let (dir, mut owner) = vault();
    let (_recipient_dir, mut recipient) = vault();
    let parent = room(&mut owner);
    let first = import(&mut owner, &parent, &media(&dir));
    let other_file = dir.path().join("private.jpg");
    fs::write(&other_file, b"unshared private memory").unwrap();
    let other = import(&mut owner, &parent, &other_file);
    owner
        .execute(
            json!({"action":"patch_asset","room_id":parent,"asset_id":first,
        "fields":{"caption":"Visible caption"}}),
        )
        .unwrap();
    owner
        .execute(
            json!({"action":"patch_asset","room_id":parent,"asset_id":other,
        "fields":{"caption":"SECRET UNRELATED CAPTION"}}),
        )
        .unwrap();
    let album = owner
        .execute(json!({"action":"save_album","room_id":parent,
        "title":"Graduation","asset_ids":[first]}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let scope = owner
        .execute(json!({"action":"publish_album","room_id":parent,
        "album_id":album}))
        .unwrap()["scope_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(
        owner.room(&scope).unwrap().secret,
        owner.room(&parent).unwrap().secret
    );
    assert!(owner
        .execute(json!({"action":"invite","room_id":scope,
        "device_id":recipient.device().unwrap(),"role":"contributor"}))
        .is_err());
    invite(&mut owner, &mut recipient, &scope, Role::Viewer);
    assert!(recipient.room(&parent).is_err());
    assert!(recipient
        .merge_packet(&scope, &owner.packet(&parent).unwrap())
        .is_err());
    recipient
        .merge_packet(&scope, &owner.packet(&scope).unwrap())
        .unwrap();
    let shared = recipient.assets(&scope).unwrap();
    assert_eq!(shared.len(), 1);
    assert_eq!(shared[0].caption, "Visible caption");
    assert_ne!(shared[0].id, first);
    assert!(!serde_json::to_string(&recipient.snapshot().unwrap())
        .unwrap()
        .contains("SECRET UNRELATED"));
    let package = dir.path().join("album.frroom");
    owner
        .execute(json!({"action":"export_room","room_id":scope,"path":package}))
        .unwrap();
    recipient
        .execute(json!({"action":"import_room","room_id":scope,"path":package}))
        .unwrap();
    let original = dir.path().join("received.jpg");
    recipient
        .execute(json!({"action":"materialize","room_id":scope,
        "asset_id":shared[0].id,"destination":original}))
        .unwrap();
    assert_eq!(fs::read(original).unwrap(), b"private-photo-original");
    assert!(recipient
        .execute(json!({"action":"import","room_id":scope,"path":other_file}))
        .is_err());
    // Existing membership and source originals survive republishing an empty album.
    owner
        .execute(json!({"action":"save_album","room_id":parent,"id":album,
        "title":"Graduation","asset_ids":[]}))
        .unwrap();
    let refreshed = owner
        .execute(json!({"action":"publish_album","room_id":parent,"album_id":album}))
        .unwrap();
    assert_eq!(refreshed["scope_id"], scope);
    recipient
        .merge_packet(&scope, &owner.packet(&scope).unwrap())
        .unwrap();
    assert!(recipient.assets(&scope).unwrap()[0].deleted);
    // A later audience cannot receive the old scope's retained originals/history.
    let (_later_dir, mut later) = vault();
    assert!(owner
        .execute(json!({"action":"invite","room_id":scope,
        "device_id":later.device().unwrap(),"role":"viewer"}))
        .is_err());
    let fresh = owner
        .execute(json!({"action":"invite_album","room_id":parent,
        "album_id":album,"device_id":later.device().unwrap()}))
        .unwrap();
    let new_scope = fresh["scope_id"].as_str().unwrap().to_owned();
    assert_ne!(new_scope, scope);
    later
        .execute(json!({"action":"join","invitation":fresh["invitation"]}))
        .unwrap();
    later
        .merge_packet(&new_scope, &owner.packet(&new_scope).unwrap())
        .unwrap();
    assert!(later.assets(&new_scope).unwrap().is_empty());
    assert!(later
        .merge_packet(&new_scope, &owner.packet(&scope).unwrap())
        .is_err());
    assert!(!serde_json::to_string(&later.snapshot().unwrap())
        .unwrap()
        .contains("Visible caption"));
    let old_original = owner.assets(&scope).unwrap()[0].clone();
    assert!(crypto::verify_file(
        &crypto::derive(
            &crypto::parse_key(&later.room(&new_scope).unwrap().secret).unwrap(),
            b"media"
        ),
        &owner.blob(&scope, &old_original.id).unwrap(),
        &format!("{scope}:{}", old_original.id),
        &old_original.sha256,
    )
    .is_err());
    // Republishing sends the current selection to all established audiences.
    owner
        .execute(json!({"action":"save_album","room_id":parent,"id":album,
        "title":"Graduation","asset_ids":[other]}))
        .unwrap();
    let update = owner
        .execute(json!({"action":"publish_album","room_id":parent,"album_id":album}))
        .unwrap();
    assert_eq!(update["updated_scope_ids"].as_array().unwrap().len(), 2);
    later
        .merge_packet(&new_scope, &owner.packet(&new_scope).unwrap())
        .unwrap();
    recipient
        .merge_packet(&scope, &owner.packet(&scope).unwrap())
        .unwrap();
    assert_eq!(later.assets(&new_scope).unwrap().len(), 1);
    assert_eq!(
        later.assets(&new_scope).unwrap()[0].caption,
        "SECRET UNRELATED CAPTION"
    );
    assert_eq!(
        recipient
            .assets(&scope)
            .unwrap()
            .iter()
            .filter(|asset| !asset.deleted)
            .count(),
        1
    );
    assert!(owner.blob(&parent, &first).unwrap().is_file());
    assert!(
        !owner
            .assets(&parent)
            .unwrap()
            .iter()
            .find(|a| a.id == first)
            .unwrap()
            .deleted
    );
}

#[test]
fn reimport_restores_evicted_original_without_rewriting_shared_identity() {
    let (dir, mut engine) = vault();
    let r = room(&mut engine);
    let path = media(&dir);
    let asset = import(&mut engine, &r, &path);
    let before = engine.room(&r).unwrap().operations.len();
    fs::remove_file(engine.blob(&r, &asset).unwrap()).unwrap();
    assert!(engine
        .execute(json!({"action":"import","room_id":r,"path":path}))
        .unwrap()["duplicate"]
        .as_bool()
        .unwrap());
    assert!(engine.blob(&r, &asset).unwrap().is_file());
    assert_eq!(before, engine.room(&r).unwrap().operations.len());
}
#[test]
fn originals_and_catalog_are_encrypted_and_reopen() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let a = import(&mut e, &r, &media(&dir));
    let blob = fs::read(e.blob(&r, &a).unwrap()).unwrap();
    assert!(!blob.windows(22).any(|w| w == b"private-photo-original"));
    let catalog = fs::read(dir.path().join("catalog.fr")).unwrap();
    assert!(!String::from_utf8_lossy(&catalog).contains("memory.jpg"));
    let target = dir.path().join("restored.jpg");
    e.execute(json!({"action":"materialize","room_id":r,"asset_id":a,"destination":target}))
        .unwrap();
    assert_eq!(fs::read(target).unwrap(), b"private-photo-original");
    drop(e);
    assert_eq!(
        Engine::open(dir.path(), [7; 32])
            .unwrap()
            .assets(&r)
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn corrupt_or_wrong_key_catalog_is_never_replaced() {
    let (dir, mut e) = vault();
    room(&mut e);
    drop(e);
    let before = fs::read(dir.path().join("catalog.fr")).unwrap();
    assert!(Engine::open(dir.path(), [9; 32]).is_err());
    assert_eq!(fs::read(dir.path().join("catalog.fr")).unwrap(), before);
    fs::write(dir.path().join("catalog.fr"), [0xff, 0xff]).unwrap();
    assert!(Engine::open(dir.path(), [7; 32]).is_err());
    assert_eq!(
        fs::read(dir.path().join("catalog.fr")).unwrap(),
        vec![0xff, 0xff]
    );
}
#[test]
fn second_writer_cannot_open_same_catalog() {
    let (dir, _e) = vault();
    assert!(Engine::open(dir.path(), [7; 32]).is_err());
}
#[test]
fn concurrent_duplicate_imports_do_not_reset_existing_metadata() {
    let (owner_dir, mut owner) = vault();
    let (member_dir, mut member) = vault();
    let r = room(&mut owner);
    invite(&mut owner, &mut member, &r, Role::Contributor);
    let path = media(&owner_dir);
    let a = owner
        .execute(json!({"action":"import","room_id":r,"path":path,
        "captured_at":42}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    owner
        .execute(json!({"action":"patch_asset","room_id":r,"asset_id":a,
        "fields":{"caption":"Keep our caption","favorite":true}}))
        .unwrap();
    // This member has not seen the original or caption. Their duplicate creation
    // sorts after those edits, which used to reset both fields.
    for index in 0..4 {
        let path = member_dir.path().join(format!("other-{index}.jpg"));
        fs::write(&path, format!("different original {index}")).unwrap();
        import(&mut member, &r, &path);
    }
    let received = member_dir.path().join("received.jpg");
    fs::write(&received, b"private-photo-original").unwrap();
    assert_eq!(
        member
            .execute(json!({"action":"import","room_id":r,
        "path":received,"captured_at":99}))
            .unwrap()["id"],
        a
    );
    let member_packet = member.packet(&r).unwrap();
    let owner_packet = owner.packet(&r).unwrap();
    owner.merge_packet(&r, &member_packet).unwrap();
    member.merge_packet(&r, &owner_packet).unwrap();
    assert_eq!(owner.entities(&r).unwrap(), member.entities(&r).unwrap());
    for engine in [&mut owner, &mut member] {
        let asset = engine
            .assets(&r)
            .unwrap()
            .into_iter()
            .find(|v| v.id == a)
            .unwrap();
        assert_eq!(asset.caption, "Keep our caption");
        assert!(asset.favorite);
        assert_eq!(asset.captured_at, 42);
        assert_eq!(asset.filename, "memory.jpg");
        let output = engine.root.join("duplicate-restored.jpg");
        engine
            .execute(json!({"action":"materialize","room_id":r,"asset_id":a,
            "destination":output}))
            .unwrap();
        assert_eq!(fs::read(output).unwrap(), b"private-photo-original");
    }
}

#[test]
fn duplicate_import_and_room_isolation() {
    let (dir, mut e) = vault();
    let first = room(&mut e);
    let second = room(&mut e);
    let path = media(&dir);
    let a = import(&mut e, &first, &path);
    assert_eq!(a, import(&mut e, &first, &path));
    assert_ne!(a, import(&mut e, &second, &path));
    assert_eq!(e.assets(&first).unwrap().len(), 1);
}
#[test]
fn tampered_original_never_overwrites_destination() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let a = import(&mut e, &r, &media(&dir));
    let blob = e.blob(&r, &a).unwrap();
    let mut bytes = fs::read(&blob).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    fs::write(blob, bytes).unwrap();
    let dest = dir.path().join("important.jpg");
    fs::write(&dest, b"keep").unwrap();
    assert!(e
        .execute(json!({"action":"materialize","room_id":r,"asset_id":a,"destination":dest}))
        .is_err());
    assert_eq!(fs::read(dest).unwrap(), b"keep");
}
#[test]
fn viewer_cannot_contribute_and_tags_do_not_grant_access() {
    let (dir, mut owner) = vault();
    let (_other, mut member) = vault();
    let r = room(&mut owner);
    invite(&mut owner, &mut member, &r, Role::Viewer);
    let members_before = owner.snapshot().unwrap()["rooms"][0]["members"]
        .as_array()
        .unwrap()
        .len();
    assert!(member
        .execute(json!({"action":"import","room_id":r,"path":media(&dir)}))
        .is_err());
    owner
        .execute(json!({"action":"save_person","room_id":r,"name":"Grandma"}))
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["members"]
            .as_array()
            .unwrap()
            .len(),
        members_before
    );
}
#[test]
fn invitations_are_bound_to_devices() {
    let (_a, mut owner) = vault();
    let (_b, member) = vault();
    let (_c, mut stranger) = vault();
    let r = room(&mut owner);
    let token=owner.execute(json!({"action":"invite","room_id":r,"device_id":member.device().unwrap(),"role":"contributor"})).unwrap()["invitation"].clone();
    assert!(stranger
        .execute(json!({"action":"join","invitation":token}))
        .is_err());
}
#[test]
fn replay_is_idempotent_and_signature_tampering_rejected() {
    let (dir, mut owner) = vault();
    let (_b, mut member) = vault();
    let r = room(&mut owner);
    import(&mut owner, &r, &media(&dir));
    invite(&mut owner, &mut member, &r, Role::Contributor);
    let packet = owner.packet(&r).unwrap();
    assert_eq!(member.merge_packet(&r, &packet).unwrap(), 1);
    assert_eq!(member.merge_packet(&r, &packet).unwrap(), 0);
    let mut ops = owner.room(&r).unwrap().operations.clone();
    ops[0].value["filename"] = json!("forged");
    assert!(member.merge(&r, ops).is_err());
}
#[test]
fn signed_immutable_metadata_edits_are_rejected() {
    let (dir, mut owner) = vault();
    let (_b, mut member) = vault();
    let r = room(&mut owner);
    let a = import(&mut owner, &r, &media(&dir));
    invite(&mut owner, &mut member, &r, Role::Contributor);
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    let mut op = member.room(&r).unwrap().operations[0].clone();
    op.id = engine::id();
    op.author = member.device().unwrap();
    op.grant = member.room(&r).unwrap().grant.clone();
    op.kind = "asset_patch".into();
    op.entity_id = a;
    op.value = json!({"sha256":"forged"});
    signing::sign_operation(&mut op, &member.state.signing_seed).unwrap();
    assert!(owner.merge(&r, vec![op]).is_err());
}
#[test]
fn concurrent_different_fields_and_comments_converge() {
    let (dir, mut owner) = vault();
    let (_b, mut member) = vault();
    let r = room(&mut owner);
    let a = import(&mut owner, &r, &media(&dir));
    invite(&mut owner, &mut member, &r, Role::Contributor);
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    owner.execute(json!({"action":"patch_asset","room_id":r,"asset_id":a,"fields":{"caption":"Birthday"}})).unwrap();
    member
        .execute(
            json!({"action":"patch_asset","room_id":r,"asset_id":a,"fields":{"favorite":true}}),
        )
        .unwrap();
    member
        .execute(json!({"action":"comment","room_id":r,"asset_id":a,"body":"Wonderful!"}))
        .unwrap();
    let packet = owner.packet(&r).unwrap();
    owner.merge_packet(&r, &member.packet(&r).unwrap()).unwrap();
    member.merge_packet(&r, &packet).unwrap();
    assert_eq!(
        serde_json::to_value(owner.assets(&r).unwrap()).unwrap(),
        serde_json::to_value(member.assets(&r).unwrap()).unwrap()
    );
    assert_eq!(owner.assets(&r).unwrap()[0].caption, "Birthday");
    assert!(owner.assets(&r).unwrap()[0].favorite);
}
#[test]
fn room_bundle_moves_encrypted_originals_between_devices() {
    let (dir, mut owner) = vault();
    let (other, mut member) = vault();
    let r = room(&mut owner);
    let a = import(&mut owner, &r, &media(&dir));
    invite(&mut owner, &mut member, &r, Role::Contributor);
    let path = dir.path().join("room.frroom");
    owner
        .execute(json!({"action":"export_room","room_id":r,"path":path}))
        .unwrap();
    member
        .execute(json!({"action":"import_room","room_id":r,"path":path}))
        .unwrap();
    let dest = other.path().join("photo.jpg");
    member
        .execute(json!({"action":"materialize","room_id":r,"asset_id":a,"destination":dest}))
        .unwrap();
    assert_eq!(fs::read(dest).unwrap(), b"private-photo-original");
}
#[test]
fn backup_recovers_identity_media_and_albums_with_a_new_local_key() {
    let (dir, mut owner) = vault();
    let (other, mut restored) = vault();
    let r = room(&mut owner);
    let a = import(&mut owner, &r, &media(&dir));
    owner
        .execute(json!({"action":"save_album","room_id":r,"title":"Summer","asset_ids":[a]}))
        .unwrap();
    let backup = dir.path().join("backup.frbackup");
    owner
        .execute(json!({"action":"backup","path":backup}))
        .unwrap();
    let identity = owner.device().unwrap();
    assert!(restored
        .execute(json!({"action":"restore","path":backup,"recovery_key":hex::encode([8;32])}))
        .is_err());
    restored
        .execute(json!({"action":"restore","path":backup,"recovery_key":hex::encode([7;32])}))
        .unwrap();
    assert_eq!(restored.device().unwrap(), identity);
    assert_eq!(
        restored.snapshot().unwrap()["rooms"][0]["albums"][0]["title"],
        "Summer"
    );
    let dest = other.path().join("photo.jpg");
    restored
        .execute(json!({"action":"materialize","room_id":r,"asset_id":a,"destination":dest}))
        .unwrap();
    assert_eq!(fs::read(dest).unwrap(), b"private-photo-original");
}
#[test]
fn albums_references_do_not_delete_media_and_cannot_cross_rooms() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let other = room(&mut e);
    let a = import(&mut e, &r, &media(&dir));
    let album = e
        .execute(json!({"action":"save_album","room_id":r,"title":"Trip","asset_ids":[a]}))
        .unwrap()["id"]
        .clone();
    e.execute(json!({"action":"save_album","room_id":r,"id":album,"title":"Trip","asset_ids":[]}))
        .unwrap();
    assert_eq!(e.assets(&r).unwrap().len(), 1);
    assert!(e
        .execute(json!({"action":"save_album","room_id":other,"title":"Wrong","asset_ids":[a]}))
        .is_err());
}
#[test]
fn local_storage_replica_is_verified_and_fetchable() {
    let (dir, mut e) = vault();
    let storage = tempfile::tempdir().unwrap();
    let r = room(&mut e);
    let a = import(&mut e, &r, &media(&dir));
    let source=e.execute(json!({"action":"add_source","room_id":r,"name":"Home server","kind":"local","endpoint":storage.path()})).unwrap()["id"].clone();
    assert_eq!(
        e.execute(json!({"action":"replicate","source_id":source,"asset_id":a}))
            .unwrap()["state"],
        "complete"
    );
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        2
    );
    fs::remove_file(e.blob(&r, &a).unwrap()).unwrap();
    e.execute(json!({"action":"fetch","room_id":r,"asset_id":a}))
        .unwrap();
    assert!(e.blob(&r, &a).unwrap().is_file());
}
#[test]
fn verified_copy_requires_original_authentication_and_supports_repair() {
    let (dir, mut e) = vault();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let r = room(&mut e);
    let path = media(&dir);
    let a = import(&mut e, &r, &path);
    let source = e
        .execute(json!({"action":"add_source","room_id":r,
        "name":"First backup","kind":"local","endpoint":first.path()}))
        .unwrap()["id"]
        .clone();
    let other = e
        .execute(json!({"action":"add_source","room_id":r,
        "name":"Second backup","kind":"local","endpoint":second.path()}))
        .unwrap()["id"]
        .clone();
    let blob = e.blob(&r, &a).unwrap();
    let original_ciphertext = fs::read(&blob).unwrap();
    let remote = first.path().join(format!("{r}-{a}.frblob"));

    // Matching ciphertext is insufficient: detect corrupt local data before sending it.
    let mut corrupt = original_ciphertext.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    fs::write(&blob, &corrupt).unwrap();
    let job = e
        .execute(json!({"action":"replicate","source_id":source,"asset_id":a}))
        .unwrap();
    assert_eq!(job["state"], "waiting");
    assert!(!remote.exists());
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        0
    );
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["availability"],
        "Local original needs repair"
    );
    drop(e);
    let mut e = Engine::open(dir.path(), [7; 32]).unwrap();
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["damaged"],
        true
    );
    assert_eq!(
        e.execute(json!({"action":"import","room_id":r,"path":path}))
            .unwrap()["duplicate"],
        true
    );
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["damaged"],
        false
    );
    assert_eq!(
        e.execute(json!({"action":"replicate","source_id":source,"asset_id":a}))
            .unwrap()["state"],
        "complete"
    );
    assert_eq!(
        e.execute(json!({"action":"replicate","source_id":other,"asset_id":a}))
            .unwrap()["state"],
        "complete"
    );

    // Local damage does not invalidate independently verified remote originals.
    fs::write(&blob, &corrupt).unwrap();
    assert_eq!(
        e.execute(json!({"action":"replicate","source_id":source,"asset_id":a}))
            .unwrap()["state"],
        "waiting"
    );
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        2
    );
    e.execute(json!({"action":"fetch","room_id":r,"asset_id":a}))
        .unwrap();
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        3
    );

    // Readback must authenticate the original, not merely trust an old verification.
    fs::write(&remote, &corrupt).unwrap();
    assert_eq!(
        e.execute(json!({"action":"replicate","source_id":source,"asset_id":a}))
            .unwrap()["state"],
        "waiting"
    );
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        2
    );
    fs::remove_file(&blob).unwrap();
    e.execute(json!({"action":"fetch","room_id":r,"asset_id":a}))
        .unwrap();
    let destination = dir.path().join("repaired.jpg");
    e.execute(json!({"action":"materialize","room_id":r,"asset_id":a,"destination":destination}))
        .unwrap();
    assert_eq!(fs::read(destination).unwrap(), b"private-photo-original");
    assert_eq!(
        e.execute(json!({"action":"replicate","source_id":source,"asset_id":a}))
            .unwrap()["state"],
        "complete"
    );
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        3
    );
}

#[test]
fn failed_reads_preserve_copy_health_without_confusing_destination_errors() {
    let (dir, mut e) = vault();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let r = room(&mut e);
    let path = media(&dir);
    let a = import(&mut e, &r, &path);
    for storage in [&first, &second] {
        let source = e
            .execute(json!({"action":"add_source","room_id":r,
            "name":"Backup","kind":"local","endpoint":storage.path()}))
            .unwrap()["id"]
            .clone();
        e.execute(json!({"action":"replicate","source_id":source,"asset_id":a}))
            .unwrap();
        fs::write(storage.path().join(format!("{r}-{a}.frblob")), b"truncated").unwrap();
    }
    fs::remove_file(e.blob(&r, &a).unwrap()).unwrap();
    assert!(e
        .execute(json!({"action":"fetch","room_id":r,"asset_id":a}))
        .is_err());
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        0
    );
    drop(e);
    let mut e = Engine::open(dir.path(), [7; 32]).unwrap();
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        0
    );

    // An invalid destination is not evidence of damage to a healthy original.
    e.execute(json!({"action":"import","room_id":r,"path":path}))
        .unwrap();
    let destination = dir.path().join("output-directory");
    fs::create_dir(&destination).unwrap();
    assert!(e
        .execute(json!({"action":"materialize","room_id":r,"asset_id":a,
        "destination":destination}))
        .is_err());
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["damaged"],
        false
    );
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        1
    );

    // Source truncation must be remembered even though materialization rolls back.
    fs::write(e.blob(&r, &a).unwrap(), b"truncated").unwrap();
    let output = dir.path().join("damaged.jpg");
    assert!(e
        .execute(json!({"action":"materialize","room_id":r,"asset_id":a,
        "destination":output}))
        .is_err());
    assert!(!output.exists());
    drop(e);
    let e = Engine::open(dir.path(), [7; 32]).unwrap();
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["damaged"],
        true
    );
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        0
    );
}

#[test]
fn quota_exhaustion_keeps_original_and_queues_job() {
    let (dir, mut e) = vault();
    let storage = tempfile::tempdir().unwrap();
    let r = room(&mut e);
    let a = import(&mut e, &r, &media(&dir));
    let source=e.execute(json!({"action":"add_source","room_id":r,"name":"Full","kind":"local","endpoint":storage.path(),"budget":1})).unwrap()["id"].clone();
    let job = e
        .execute(json!({"action":"replicate","source_id":source,"asset_id":a}))
        .unwrap();
    assert_eq!(job["state"], "waiting");
    assert!(e.blob(&r, &a).unwrap().is_file());
}
#[test]
fn local_source_sync_has_no_credentials_in_snapshot() {
    let (_dir, mut e) = vault();
    let r = room(&mut e);
    e.execute(json!({"action":"add_source","room_id":r,"name":"DAV","kind":"webdav","endpoint":"https://example.com/vault","token":"SECRET-TOKEN"})).unwrap();
    assert!(!e.snapshot().unwrap().to_string().contains("SECRET-TOKEN"));
}
#[test]
fn timeline_revisions_survive_and_invalid_timings_fail() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let a = import(&mut e, &r, &media(&dir));
    let mut project = json!({"id":engine::id(),"room_id":r,"title":"Our film","version_id":"","editor":"","width":1920,"height":1080,"clips":[{"id":engine::id(),"asset_id":a,"track":0,"start":0,"trim_in":0,"duration":3,"speed":1,"volume":1,"opacity":1,"exposure":0,"saturation":1,"title":"Birthday","fade_in":0.3,"fade_out":0.3,"opacity_keyframes":[],"volume_keyframes":[]}],"chapters":[],"published_asset":null});
    e.execute(json!({"action":"save_story","project":project}))
        .unwrap();
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["stories"][0]["clips"][0]["fade_in_offset"],
        0.0
    );
    project["clips"][0]["duration"] = json!(4);
    project["clips"][0]["fade_in_offset"] = json!(1.25);
    project["clips"][0]["fade_out_offset"] = json!(2.0);
    e.execute(json!({"action":"save_story","project":project}))
        .unwrap();
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["stories"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    drop(e);
    let mut e = Engine::open(dir.path(), [7; 32]).unwrap();
    let stories = e.snapshot().unwrap()["rooms"][0]["stories"]
        .as_array()
        .unwrap()
        .clone();
    assert!(stories
        .iter()
        .any(|story| story["clips"][0]["fade_in_offset"] == 1.25
            && story["clips"][0]["fade_out_offset"] == 2.0));
    let before = fs::read(dir.path().join("catalog.fr")).unwrap();
    project["clips"][0]["fade_in_offset"] = json!(-1);
    assert!(e
        .execute(json!({"action":"save_story","project":project}))
        .is_err());
    assert_eq!(fs::read(dir.path().join("catalog.fr")).unwrap(), before);
    project["clips"][0]["fade_in_offset"] = json!(0);
    project["clips"][0]["fade_out_offset"] = json!(86401);
    assert!(e
        .execute(json!({"action":"save_story","project":project}))
        .is_err());
    project["clips"][0]["fade_out_offset"] = json!(0);
    project["clips"][0]["duration"] = json!(-1);
    assert!(e
        .execute(json!({"action":"save_story","project":project}))
        .is_err());
}

#[test]
fn versioned_drafts_and_shared_split_preserve_automation_without_publishing() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let a = import(&mut e, &r, &media(&dir));
    let created = e
        .execute(json!({"action":"create_story","room_id":r,"title":"Portable draft"}))
        .unwrap();
    let mut project = created["project"].clone();
    assert_eq!(project["format_version"], 1);
    assert_eq!(project["private_draft"], true);
    project["clips"] = json!([{"id":engine::id(),"asset_id":a,"track":2,"start":7,"trim_in":5,"duration":8,"speed":2,"volume":0.8,"opacity":0.7,"exposure":0,"saturation":1,"title":"","fade_in":3,"fade_out":3,"opacity_keyframes":[{"time":1,"value":0.2},{"time":3,"value":0.9},{"time":7,"value":0.4}],"volume_keyframes":[{"time":0,"value":0.1},{"time":5,"value":0.8},{"time":8,"value":0.3}]}]);
    let split = e.execute(json!({"action":"split_story_clip","project":project,"clip_id":project["clips"][0]["id"],"source_time":4})).unwrap()["project"].clone();
    assert_eq!(split["private_draft"], true);
    assert_eq!(split["clips"][0]["id"], project["clips"][0]["id"]);
    assert_eq!(split["clips"][0]["fade_out_offset"], 2.0);
    assert_eq!(split["clips"][1]["fade_in_offset"], 2.0);
    assert_eq!(split["clips"][1]["start"], 9.0);
    assert_eq!(split["clips"][1]["trim_in"], 9.0);
    let left: model::Clip = serde_json::from_value(split["clips"][0].clone()).unwrap();
    let right: model::Clip = serde_json::from_value(split["clips"][1].clone()).unwrap();
    assert!((left.opacity_keyframes.last().unwrap().value - 0.775).abs() < 1e-9);
    assert!((right.opacity_keyframes.first().unwrap().value - 0.775).abs() < 1e-9);
    assert!((left.volume_keyframes.last().unwrap().value - 0.66).abs() < 1e-9);
    assert!((right.volume_keyframes.first().unwrap().value - 0.66).abs() < 1e-9);
    for clip in [&left, &right] {
        assert!(clip
            .opacity_keyframes
            .iter()
            .chain(&clip.volume_keyframes)
            .all(|frame| frame.time >= 0.0 && frame.time <= clip.duration));
    }
    // Previewing a split does not add a saved revision; saving it stays private.
    assert_eq!(
        e.snapshot().unwrap()["rooms"][0]["stories"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    e.execute(json!({"action":"save_story","project":split}))
        .unwrap();
    assert!(!e
        .room(&r)
        .unwrap()
        .operations
        .iter()
        .any(|op| op.kind == "story"));
    let before = fs::read(dir.path().join("catalog.fr")).unwrap();
    project["format_version"] = json!(2);
    assert!(e
        .execute(json!({"action":"save_story","project":project}))
        .is_err());
    assert!(e.execute(json!({"action":"split_story_clip","project":project,"clip_id":project["clips"][0]["id"],"source_time":4})).is_err());
    assert_eq!(fs::read(dir.path().join("catalog.fr")).unwrap(), before);
    drop(e);
    let reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    let latest = reopened.snapshot().unwrap()["rooms"][0]["stories"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .clone();
    assert_eq!(latest["format_version"], 1);
    assert_eq!(latest["clips"][1]["fade_in_offset"], 2.0);
}

#[test]
fn unsupported_local_timeline_format_is_never_rewritten_and_legacy_still_opens() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    e.execute(json!({"action":"create_story","room_id":r}))
        .unwrap();
    let mut state = serde_json::to_value(&e.state).unwrap();
    state["drafts"][0]
        .as_object_mut()
        .unwrap()
        .remove("format_version");
    let bytes = crypto::encrypt(
        &[7; 32],
        &serde_json::to_vec(&state).unwrap(),
        b"family-room-catalog-v1",
    )
    .unwrap();
    fs::write(dir.path().join("catalog.fr"), bytes).unwrap();
    drop(e);
    let e = Engine::open(dir.path(), [7; 32]).unwrap();
    assert_eq!(e.state.drafts[0].format_version, 1);
    state["drafts"][0]["format_version"] = json!(99);
    state["drafts"][0]["future_tracks"] = json!({"keep":"unknown future data"});
    let future = crypto::encrypt(
        &[7; 32],
        &serde_json::to_vec(&state).unwrap(),
        b"family-room-catalog-v1",
    )
    .unwrap();
    fs::write(dir.path().join("catalog.fr"), &future).unwrap();
    drop(e);
    assert!(Engine::open(dir.path(), [7; 32]).is_err());
    assert_eq!(fs::read(dir.path().join("catalog.fr")).unwrap(), future);
}

#[test]
fn viewers_and_removed_devices_cannot_create_or_overwrite_private_films() {
    let (_owner_dir, mut owner) = vault();
    let (_member_dir, mut member) = vault();
    let r = room(&mut owner);
    invite(&mut owner, &mut member, &r, Role::Viewer);
    assert!(member
        .execute(json!({"action":"create_story","room_id":r}))
        .is_err());
    owner.execute(json!({"action":"set_role","room_id":r,"device_id":member.device().unwrap(),"role":"contributor"})).unwrap();
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    let project = member
        .execute(json!({"action":"create_story","room_id":r}))
        .unwrap()["project"]
        .clone();
    owner
        .execute(json!({"action":"revoke_device","room_id":r,"device_id":member.device().unwrap()}))
        .unwrap();
    let changes = owner
        .execute(json!({"action":"access_policy","room_id":r}))
        .unwrap()["changes"]
        .clone();
    member
        .execute(json!({"action":"apply_access","room_id":r,"changes":changes}))
        .unwrap();
    assert!(member
        .execute(json!({"action":"create_story","room_id":r}))
        .is_err());
    assert!(member
        .execute(json!({"action":"save_story","project":project}))
        .is_err());
    assert_eq!(member.state.drafts.len(), 1);
}
#[test]
fn invalid_command_preserves_catalog_and_secret_is_not_in_snapshot() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let before = fs::read(dir.path().join("catalog.fr")).unwrap();
    assert!(e
        .execute(json!({"action":"save_album","room_id":r,"title":"Bad","asset_ids":["missing"]}))
        .is_err());
    assert_eq!(fs::read(dir.path().join("catalog.fr")).unwrap(), before);
    assert!(!e
        .snapshot()
        .unwrap()
        .to_string()
        .contains(&e.room(&r).unwrap().secret));
}

#[test]
fn captions_preserve_conflicting_offline_edits_and_private_drafts_do_not_sync() {
    let (dir, mut owner) = vault();
    let (_member_dir, mut member) = vault();
    let r = room(&mut owner);
    let a = import(&mut owner, &r, &media(&dir));
    invite(&mut owner, &mut member, &r, Role::Contributor);
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    owner.execute(json!({"action":"patch_asset","room_id":r,"asset_id":a,"fields":{"caption":"Owner offline"}})).unwrap();
    member.execute(json!({"action":"patch_asset","room_id":r,"asset_id":a,"fields":{"caption":"Member offline"}})).unwrap();
    owner.merge_packet(&r, &member.packet(&r).unwrap()).unwrap();
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    assert_eq!(
        owner.assets(&r).unwrap()[0].caption,
        member.assets(&r).unwrap()[0].caption
    );
    assert_eq!(
        owner
            .execute(json!({"action":"asset_history","room_id":r,"asset_id":a}))
            .unwrap()["history"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    owner.execute(json!({"action":"save_story","project":{"id":"draft","room_id":r,"title":"Private reel","version_id":"","editor":"","width":128,"height":128,"clips":[],"chapters":[],"published_asset":null,"auto_generated":true}})).unwrap();
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["stories"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(member.snapshot().unwrap()["rooms"][0]["stories"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn photos_import_commits_components_and_checkpoint_after_rechecking_source() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let other = room(&mut e);
    let path = media(&dir);
    let component = dir.path().join("paired.mov");
    fs::write(&component, b"paired motion original").unwrap();
    let watch = e
        .execute(json!({"action":"watch_photos","room_id":r,"seen":[]}))
        .unwrap()["id"]
        .clone();
    let command = json!({"action":"import_photo","id":watch,"room_id":r,
        "identifier":"photo/1","paths":[path,component],"captured_at":12345});
    e.execute(json!({"action":"pause_watch","id":watch,"paused":true}))
        .unwrap();
    assert_eq!(e.execute(command.clone()).unwrap()["imported"], false);
    assert!(e.assets(&r).unwrap().is_empty());
    assert!(e.state.watches[0].seen.is_empty());
    e.execute(json!({"action":"pause_watch","id":watch,"paused":false}))
        .unwrap();
    let mut wrong_destination = command.clone();
    wrong_destination["room_id"] = json!(other);
    assert_eq!(e.execute(wrong_destination).unwrap()["imported"], false);
    assert!(e.assets(&other).unwrap().is_empty());

    let mut failed = command.clone();
    failed["paths"] = json!([path, dir.path().join("missing.mov")]);
    assert!(e.execute(failed).is_err());
    assert!(e.assets(&r).unwrap().is_empty());
    assert!(e.state.watches[0].seen.is_empty());
    let result = e.execute(command.clone()).unwrap();
    assert_eq!(result["imported"], true);
    let primary = result["asset_ids"][0].as_str().unwrap();
    let assets = e.assets(&r).unwrap();
    let photo = assets.iter().find(|a| a.id == primary).unwrap();
    assert_eq!(photo.captured_at, 12345);
    assert_eq!(
        photo.components,
        vec![result["asset_ids"][1].as_str().unwrap()]
    );
    assert_eq!(e.state.watches[0].seen, vec!["photo/1"]);
    assert_eq!(e.execute(command.clone()).unwrap()["imported"], false);
    assert_eq!(e.assets(&r).unwrap().len(), 2);

    // An in-flight download from a replaced rule must not use either destination.
    e.execute(json!({"action":"watch_photos","room_id":other,"seen":[]}))
        .unwrap();
    let mut stale = command;
    stale["identifier"] = json!("photo/2");
    assert_eq!(e.execute(stale).unwrap()["imported"], false);
    assert!(e.assets(&other).unwrap().is_empty());
    assert!(e.state.watches[0].seen.is_empty());
}

#[test]
fn photo_exclusion_edits_preserve_complete_items_and_retry_after_reopening() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let path = media(&dir);
    let motion = dir.path().join("paired.MOV");
    fs::write(&motion, b"motion original").unwrap();
    let watch = e
        .execute(json!({"action":"watch_photos","room_id":r,
        "seen":[],"excluded_extensions":[" .JPG ", "mov", ".jpg"]}))
        .unwrap()["id"]
        .clone();
    assert_eq!(e.state.watches[0].excluded_extensions, vec!["jpg", "mov"]);
    let batch = json!({"action":"import_photo","id":watch,"room_id":r,
        "identifier":"photo/live","paths":[path,motion],"captured_at":52});
    assert_eq!(e.execute(batch.clone()).unwrap()["imported"], false);
    assert!(e.assets(&r).unwrap().is_empty());
    assert!(e.state.watches[0].seen.is_empty());
    assert!(e
        .execute(json!({"action":"set_watch_exclusions","id":watch,
        "excluded_extensions":["jpg", "../mov"]}))
        .is_err());
    assert_eq!(e.state.watches[0].excluded_extensions, vec!["jpg", "mov"]);
    e.execute(json!({"action":"set_watch_exclusions","id":watch,
        "excluded_extensions":[" .MOV "]}))
        .unwrap();
    // Excluding just the motion component must not partially publish its photo.
    assert_eq!(e.execute(batch.clone()).unwrap()["imported"], false);
    assert!(e.assets(&r).unwrap().is_empty());
    assert!(e.state.watches[0].seen.is_empty());
    e.execute(json!({"action":"set_watch_exclusions","id":watch,
        "excluded_extensions":[]}))
        .unwrap();
    drop(e);
    let mut e = Engine::open(dir.path(), [7; 32]).unwrap();
    let result = e.execute(batch).unwrap();
    assert_eq!(result["imported"], true);
    let photo = e
        .assets(&r)
        .unwrap()
        .into_iter()
        .find(|a| a.id == result["asset_ids"][0])
        .unwrap();
    assert_eq!(
        photo.components,
        vec![result["asset_ids"][1].as_str().unwrap()]
    );
    assert_eq!(e.state.watches[0].seen, vec!["photo/live"]);

    let (_viewer_dir, mut viewer) = vault();
    invite(&mut e, &mut viewer, &r, Role::Viewer);
    assert!(viewer
        .execute(json!({"action":"watch_folder","room_id":r,
        "path":dir.path(),"include_existing":true}))
        .is_err());
    assert!(viewer
        .execute(json!({"action":"watch_photos","room_id":r,"seen":[]}))
        .is_err());
    assert!(viewer.state.watches.is_empty());
}

#[test]
fn folder_upload_rules_respect_destination_exclusions_and_pause() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let other = room(&mut e);
    let source = dir.path().join("camera");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("old.jpg"), b"old original").unwrap();
    fs::write(source.join("skip.mp4"), b"excluded video").unwrap();
    let rule=e.execute(json!({"action":"watch_folder","room_id":r,"path":source,"include_existing":true,"excluded_extensions":["mp4"]})).unwrap()["id"].clone();
    e.execute(json!({"action":"pause_watch","id":rule,"paused":true}))
        .unwrap();
    e.execute(json!({"action":"scan_watches"})).unwrap();
    assert!(e.assets(&r).unwrap().is_empty());
    e.execute(json!({"action":"pause_watch","id":rule,"paused":false}))
        .unwrap();
    // Files must settle before the importer reads them.
    std::thread::sleep(std::time::Duration::from_millis(2100));
    e.execute(json!({"action":"scan_watches"})).unwrap();
    assert_eq!(e.assets(&r).unwrap().len(), 1);
    assert!(e.assets(&other).unwrap().is_empty());
    e.execute(json!({"action":"scan_watches"})).unwrap();
    assert_eq!(e.assets(&r).unwrap().len(), 1);
}

#[test]
fn portable_export_bounds_unicode_names_and_preserves_original_name_mapping() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let filename = format!("CON:{}?.JPG", "é".repeat(100));
    let path = dir.path().join(&filename);
    fs::write(&path, b"portable original").unwrap();
    let a = import(&mut e, &r, &path);
    let destination = dir.path().join("portable-export");
    e.execute(json!({"action":"export_library","room_id":r,"destination":destination}))
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(destination.join("library.json")).unwrap()).unwrap();
    let asset = manifest["room"]["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|asset| asset["id"] == a)
        .unwrap();
    assert_eq!(asset["filename"], filename);
    let relative = asset["export_file"].as_str().unwrap();
    let name = std::path::Path::new(relative)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    assert!(name.len() <= 225);
    assert!(name.ends_with(".JPG"));
    assert!(name.starts_with(&format!("{a}-")));
    assert!(!name
        .chars()
        .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c)));
    assert_eq!(
        fs::read(destination.join(relative)).unwrap(),
        b"portable original"
    );
    assert_eq!(e.assets(&r).unwrap()[0].filename, filename);
}

#[test]
fn moments_and_portable_exports_preserve_originals_and_organization() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let a = import(&mut e, &r, &media(&dir));
    e.execute(json!({"action":"patch_asset","room_id":r,"asset_id":a,"fields":{"caption":"A real memory"}})).unwrap();
    e.execute(json!({"action":"save_moment","room_id":r,"title":"First day","asset_ids":[a],"featured":true})).unwrap();
    let (_member_dir, mut member) = vault();
    invite(&mut e, &mut member, &r, Role::Contributor);
    member.merge_packet(&r, &e.packet(&r).unwrap()).unwrap();
    assert_eq!(
        member.snapshot().unwrap()["rooms"][0]["moments"][0]["title"],
        "First day"
    );
    let output = dir.path().join("portable");
    e.execute(json!({"action":"export_library","room_id":r,"destination":output}))
        .unwrap();
    let metadata: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("library.json")).unwrap()).unwrap();
    assert_eq!(metadata["room"]["moments"][0]["title"], "First day");
    assert_eq!(metadata["room"]["assets"][0]["caption"], "A real memory");
    assert_eq!(
        fs::read(output.join("originals").join(format!("{a}-memory.jpg"))).unwrap(),
        b"private-photo-original"
    );
    assert!(!metadata.to_string().contains(&e.room(&r).unwrap().secret));
    assert!(e
        .execute(json!({"action":"export_library","room_id":r,"destination":output}))
        .is_err());
}

#[test]
fn interrupted_large_transfer_resumes_after_reopening_and_verifies() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let video = dir.path().join("large.mp4");
    let original = vec![0x73; 5 * 1024 * 1024 + 37];
    fs::write(&video, &original).unwrap();
    let a = import(&mut e, &r, &video);
    let cloud = dir.path().join("cloud");
    fs::create_dir(&cloud).unwrap();
    let source=e.execute(json!({"action":"add_source","room_id":r,"kind":"local","name":"Test cloud","endpoint":cloud})).unwrap()["id"].clone();
    let first = e
        .execute(json!({"action":"replicate_step","source_id":source,"asset_id":a}))
        .unwrap();
    assert_eq!(first["state"], "queued");
    assert!(first["transferred"].as_u64().unwrap() > 0);
    assert!(!cloud.join(format!("{r}-{a}.frblob")).exists());
    drop(e);
    let mut e = Engine::open(dir.path(), [7; 32]).unwrap();
    let finished = e
        .execute(json!({"action":"replicate","source_id":source,"asset_id":a}))
        .unwrap();
    assert_eq!(finished["state"], "complete");
    fs::remove_file(e.blob(&r, &a).unwrap()).unwrap();
    e.execute(json!({"action":"fetch","room_id":r,"asset_id":a}))
        .unwrap();
    let restored = dir.path().join("restored.mp4");
    e.execute(json!({"action":"materialize","room_id":r,"asset_id":a,"destination":restored}))
        .unwrap();
    assert_eq!(fs::read(restored).unwrap(), original);
}

#[cfg(feature = "gateway")]
#[test]
fn gateway_shares_ciphertext_between_devices_without_provider_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let app = crate::gateway::router(directory.path().to_owned(), None).unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let (shutdown, stop) = tokio::sync::oneshot::channel::<()>();
    let thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            sender.send(listener.local_addr().unwrap()).unwrap();
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = stop.await;
                })
                .await
                .unwrap();
        });
    });
    let endpoint = format!("http://{}", receiver.recv().unwrap());
    let (owner_dir, mut owner) = vault();
    let (_member_dir, mut member) = vault();
    let (_viewer_dir, mut viewer) = vault();
    let r = room(&mut owner);
    let a = import(&mut owner, &r, &media(&owner_dir));
    invite(&mut owner, &mut member, &r, Role::Contributor);
    invite(&mut owner, &mut viewer, &r, Role::Viewer);
    let connect = |engine: &mut Engine| {
        engine.execute(json!({"action":"add_source","room_id":r,"kind":"gateway","name":"Shared encrypted gateway","endpoint":endpoint})).unwrap()["id"].clone()
    };
    let owner_source = connect(&mut owner);
    owner
        .execute(json!({"action":"replicate","source_id":owner_source,"asset_id":a}))
        .unwrap();
    owner
        .execute(json!({"action":"sync_source","id":owner_source}))
        .unwrap();
    let member_source = connect(&mut member);
    member
        .execute(json!({"action":"sync_source","id":member_source}))
        .unwrap();
    assert_eq!(member.assets(&r).unwrap().len(), 1);
    member
        .execute(json!({"action":"fetch","room_id":r,"asset_id":a}))
        .unwrap();
    let recovered = owner_dir.path().join("from-member.jpg");
    member
        .execute(json!({"action":"materialize","room_id":r,"asset_id":a,"destination":recovered}))
        .unwrap();
    assert_eq!(fs::read(recovered).unwrap(), b"private-photo-original");
    member.execute(json!({"action":"patch_asset","room_id":r,"asset_id":a,"fields":{"caption":"Across devices"}})).unwrap();
    member
        .execute(json!({"action":"sync_source","id":member_source}))
        .unwrap();
    owner
        .execute(json!({"action":"sync_source","id":owner_source}))
        .unwrap();
    assert_eq!(owner.assets(&r).unwrap()[0].caption, "Across devices");
    let viewer_source = connect(&mut viewer);
    viewer
        .execute(json!({"action":"sync_source","id":viewer_source}))
        .unwrap();
    assert!(viewer
        .execute(json!({"action":"replicate","source_id":viewer_source,"asset_id":a}))
        .is_err());
    let status = reqwest::blocking::get(format!("{endpoint}/v1/rooms/{r}/objects"))
        .unwrap()
        .status();
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    let stored = fs::read(
        directory
            .path()
            .join("objects")
            .join(&r)
            .join(format!("{r}-{a}.frblob")),
    )
    .unwrap();
    assert!(!stored.windows(22).any(|b| b == b"private-photo-original"));
    shutdown.send(()).unwrap();
    thread.join().unwrap();
}

#[test]
fn source_budget_counts_verified_copies_after_local_eviction() {
    let (dir, mut e) = vault();
    let r = room(&mut e);
    let first = import(&mut e, &r, &media(&dir));
    let storage = tempfile::tempdir().unwrap();
    let source = e
        .execute(json!({"action":"add_source","room_id":r,"name":"Archive",
        "kind":"local","endpoint":storage.path(),"budget":130}))
        .unwrap()["id"]
        .clone();
    let done = e
        .execute(json!({"action":"replicate","room_id":r,"asset_id":first,"source_id":source}))
        .unwrap();
    assert_eq!(done["state"], "complete");
    fs::remove_file(e.blob(&r, &first).unwrap()).unwrap();
    let path = dir.path().join("second.jpg");
    fs::write(&path, b"another-photo-original").unwrap();
    let second = import(&mut e, &r, &path);
    let job = e
        .execute(json!({"action":"replicate","room_id":r,"asset_id":second,"source_id":source}))
        .unwrap();
    assert_eq!(job["state"], "waiting");
    assert!(e.blob(&r, &second).unwrap().exists());
}

#[test]
fn preferred_source_changes_only_future_destination() {
    let (_dir, mut e) = vault();
    let r = room(&mut e);
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let a = e.execute(json!({"action":"add_source","room_id":r,"name":"First","kind":"local","endpoint":first.path()})).unwrap()["id"].clone();
    let b = e.execute(json!({"action":"add_source","room_id":r,"name":"Second","kind":"local","endpoint":second.path()})).unwrap()["id"].clone();
    assert_eq!(e.snapshot().unwrap()["sources"][0]["preferred"], true);
    e.execute(json!({"action":"prefer_source","id":b})).unwrap();
    let snapshot = e.snapshot().unwrap();
    let sources = snapshot["sources"].as_array().unwrap();
    assert_eq!(sources.iter().filter(|s| s["preferred"] == true).count(), 1);
    assert_eq!(
        sources.iter().find(|s| s["id"] == a).unwrap()["preferred"],
        false
    );
    assert_eq!(
        sources.iter().find(|s| s["id"] == b).unwrap()["preferred"],
        true
    );
}

#[test]
fn signed_remote_timeline_cannot_exhaust_native_layout() {
    let (dir, mut owner) = vault();
    let (_other, mut member) = vault();
    let r = room(&mut owner);
    let path = dir.path().join("clip.mp4");
    fs::write(&path, b"protocol-test-original").unwrap();
    let a = import(&mut owner, &r, &path);
    invite(&mut owner, &mut member, &r, Role::Contributor);
    let project = json!({"id":engine::id(),"room_id":r,"title":"Film","version_id":"","editor":"",
        "width":1920,"height":1080,"clips":[{"id":engine::id(),"asset_id":a,"track":0,
        "start":0,"trim_in":0,"duration":3,"speed":1,"volume":1,"opacity":1,"exposure":0,
        "saturation":1,"title":"","fade_in":0,"fade_out":0,"opacity_keyframes":[],
        "volume_keyframes":[]}],"chapters":[],"published_asset":a});
    owner
        .execute(json!({"action":"save_story","project":project,"publish":true}))
        .unwrap();
    let signing_seed = owner.state.signing_seed.clone();
    let room_state = owner
        .state
        .rooms
        .iter_mut()
        .find(|room| room.id == r)
        .unwrap();
    let older = room_state
        .operations
        .iter_mut()
        .find(|op| op.kind == "story")
        .unwrap();
    older.entity_id = "ffffffff-ffff-ffff-ffff-ffffffffffff".into();
    older.value["version_id"] = json!(older.entity_id);
    signing::sign_operation(older, &signing_seed).unwrap();
    let mut newer = older.clone();
    newer.id = engine::id();
    newer.clock += 1;
    newer.entity_id = "00000000-0000-0000-0000-000000000000".into();
    newer.value["version_id"] = json!(newer.entity_id);
    newer.value["title"] = json!("Latest published revision");
    signing::sign_operation(&mut newer, &signing_seed).unwrap();
    owner.merge(&r, vec![newer]).unwrap();
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    assert_eq!(
        member.snapshot().unwrap()["rooms"][0]["stories"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["title"],
        "Latest published revision"
    );
    let mut op = owner
        .room(&r)
        .unwrap()
        .operations
        .iter()
        .find(|op| op.kind == "story")
        .unwrap()
        .clone();
    op.id = engine::id();
    op.entity_id = engine::id();
    op.value["version_id"] = json!(op.entity_id);
    op.value["clips"][0]["start"] = json!(1.0e300);
    signing::sign_operation(&mut op, &owner.state.signing_seed).unwrap();
    assert!(member.merge(&r, vec![op.clone()]).is_err());
    op.value["clips"][0]["start"] = json!(0);
    op.value["clips"][0]["track"] = json!(u32::MAX);
    signing::sign_operation(&mut op, &owner.state.signing_seed).unwrap();
    assert!(member.merge(&r, vec![op]).is_err());
}

#[test]
fn offline_album_additions_and_removals_converge_without_lost_membership() {
    let (dir, mut owner) = vault();
    let (_other, mut member) = vault();
    let r = room(&mut owner);
    let first = import(&mut owner, &r, &media(&dir));
    let p = dir.path().join("second.jpg");
    fs::write(&p, b"second").unwrap();
    let second = import(&mut owner, &r, &p);
    let p = dir.path().join("third.jpg");
    fs::write(&p, b"third").unwrap();
    let third = import(&mut owner, &r, &p);
    let album = owner
        .execute(json!({"action":"save_album","room_id":r,"title":"Memories","asset_ids":[first]}))
        .unwrap()["id"]
        .clone();
    invite(&mut owner, &mut member, &r, Role::Contributor);
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    owner.execute(json!({"action":"save_album","room_id":r,"id":album,"title":"Memories","asset_ids":[first,second]})).unwrap();
    member.execute(json!({"action":"save_album","room_id":r,"id":album,"title":"Memories","asset_ids":[first,third]})).unwrap();
    let packet = owner.packet(&r).unwrap();
    owner.merge_packet(&r, &member.packet(&r).unwrap()).unwrap();
    member.merge_packet(&r, &packet).unwrap();
    let ids = owner.snapshot().unwrap()["rooms"][0]["albums"][0]["asset_ids"].clone();
    assert_eq!(ids.as_array().unwrap().len(), 3);
    assert_eq!(
        ids,
        member.snapshot().unwrap()["rooms"][0]["albums"][0]["asset_ids"]
    );
    owner.execute(json!({"action":"save_album","room_id":r,"id":album,"title":"Memories","asset_ids":[second,third]})).unwrap();
    member.execute(json!({"action":"save_album","room_id":r,"id":album,"title":"Renamed offline","asset_ids":[first,second,third]})).unwrap();
    let packet = owner.packet(&r).unwrap();
    owner.merge_packet(&r, &member.packet(&r).unwrap()).unwrap();
    member.merge_packet(&r, &packet).unwrap();
    let a = owner.snapshot().unwrap()["rooms"][0]["albums"][0].clone();
    let b = member.snapshot().unwrap()["rooms"][0]["albums"][0].clone();
    assert_eq!(a, b);
    assert_eq!(a["asset_ids"].as_array().unwrap().len(), 2);
    assert!(!a["asset_ids"].as_array().unwrap().contains(&json!(first)));
    assert_eq!(owner.assets(&r).unwrap().len(), 3);
}

#[test]
fn publication_keeps_private_history_and_editing_a_movie_remains_private() {
    let (dir, mut owner) = vault();
    let (_other, mut member) = vault();
    let r = room(&mut owner);
    let path = dir.path().join("movie.mp4");
    fs::write(&path, b"protocol-fixture").unwrap();
    let a = import(&mut owner, &r, &path);
    invite(&mut owner, &mut member, &r, Role::Contributor);
    let mut project = json!({"id":engine::id(),"room_id":r,"title":"Film","version_id":"","editor":"",
        "width":1920,"height":1080,"clips":[{"id":engine::id(),"asset_id":a,"track":0,
        "start":0,"trim_in":0,"duration":3,"speed":1,"volume":1,"opacity":1,"exposure":0,
        "saturation":1,"title":"","fade_in":0,"fade_out":0,"opacity_keyframes":[],
        "volume_keyframes":[]}],"chapters":[],"published_asset":a});
    owner
        .execute(json!({"action":"save_story","project":project}))
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["stories"][0]["private_draft"],
        true
    );
    owner
        .execute(json!({"action":"save_story","project":project,"publish":true}))
        .unwrap();
    let published = owner.snapshot().unwrap()["rooms"][0]["stories"].clone();
    assert_eq!(published.as_array().unwrap().len(), 1);
    assert_eq!(published[0]["private_draft"], false);
    assert_eq!(owner.state.drafts.len(), 1);
    assert!(owner.state.drafts[0].archived);
    project["title"] = json!("Private changes to a published film");
    owner
        .execute(json!({"action":"save_story","project":project}))
        .unwrap();
    let local = owner.snapshot().unwrap()["rooms"][0]["stories"].clone();
    assert_eq!(local.as_array().unwrap().len(), 2);
    assert_eq!(local[1]["private_draft"], true);
    assert_eq!(local[1]["published_asset"], a);
    member.merge_packet(&r, &owner.packet(&r).unwrap()).unwrap();
    let shared = member.snapshot().unwrap()["rooms"][0]["stories"].clone();
    assert_eq!(shared.as_array().unwrap().len(), 1);
    assert_eq!(shared[0]["title"], "Film");
}
