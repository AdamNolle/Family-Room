use super::*;
use crate::{engine::Engine, model::Role};
use serde_json::json;
use std::fs;

#[test]
fn imported_asset_records_contribution_time_separately_from_capture_time() {
    let (directory, mut core) = vault();
    let room_id = room(&mut core);
    let source = media(&directory);
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let asset_id = import(&mut core, &room_id, &source);

    let asset = core
        .assets(&room_id)
        .unwrap()
        .into_iter()
        .find(|asset| asset.id == asset_id)
        .unwrap();
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    assert!((before..=after).contains(&asset.contributed_at));
    assert!(asset.contributed_at > 0);

    let mut legacy = serde_json::to_value(&asset).unwrap();
    legacy.as_object_mut().unwrap().remove("contributed_at");
    let decoded: crate::model::MediaAsset = serde_json::from_value(legacy).unwrap();
    assert_eq!(decoded.contributed_at, 0);
}

#[test]
fn eviction_respects_pins_and_requires_a_verified_restorable_copy() {
    let (dir, mut core) = vault();
    let remote = tempfile::tempdir().unwrap();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &media(&dir));
    let evict = || {
        json!({
            "action":"evict_original",
            "room_id":room_id,
            "asset_id":asset_id
        })
    };

    assert!(
        core.execute(evict()).is_err(),
        "a sole local copy is preserved"
    );
    let source_id = core
        .execute(json!({
            "action":"add_source",
            "room_id":room_id,
            "name":"Local archive",
            "kind":"local",
            "endpoint":remote.path()
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    core.execute(json!({"action":"replicate","source_id":source_id,"asset_id":asset_id}))
        .unwrap();
    core.execute(json!({
        "action":"pin_original",
        "room_id":room_id,
        "asset_id":asset_id
    }))
    .unwrap();
    assert!(
        core.execute(evict()).is_err(),
        "a pinned original is preserved"
    );

    core.execute(json!({
        "action":"unpin_original",
        "room_id":room_id,
        "asset_id":asset_id
    }))
    .unwrap();
    core.execute(evict()).unwrap();
    let evicted = core.snapshot().unwrap();
    assert_eq!(evicted["rooms"][0]["assets"][0]["local"], false);
    assert_eq!(evicted["rooms"][0]["assets"][0]["verified_copies"], 1);
    assert_eq!(
        evicted["rooms"][0]["assets"][0]["availability"],
        "Original available from connected storage"
    );

    core.execute(json!({"action":"fetch","room_id":room_id,"asset_id":asset_id}))
        .unwrap();
    let restored = core.snapshot().unwrap();
    assert_eq!(restored["rooms"][0]["assets"][0]["local"], true);
    assert_eq!(restored["rooms"][0]["assets"][0]["pinned_offline"], false);
}

#[test]
fn device_storage_cleanup_verifies_replicas_and_preserves_pinned_originals() {
    let (dir, mut core) = vault();
    let remote = tempfile::tempdir().unwrap();
    let room_id = room(&mut core);
    let first_path = media(&dir);
    let first_id = import(&mut core, &room_id, &first_path);
    let second_path = dir.path().join("pinned.jpg");
    fs::write(&second_path, b"a second independent original").unwrap();
    let pinned_id = import(&mut core, &room_id, &second_path);
    let source_id = core
        .execute(json!({
            "action":"add_source",
            "room_id":room_id,
            "name":"Verified archive",
            "kind":"local",
            "endpoint":remote.path()
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    for asset_id in [&first_id, &pinned_id] {
        core.execute(json!({
            "action":"replicate", "source_id":source_id, "asset_id":asset_id
        }))
        .unwrap();
    }
    core.execute(json!({
        "action":"pin_original", "room_id":room_id, "asset_id":pinned_id
    }))
    .unwrap();

    let result = core
        .execute(json!({
            "action":"free_device_storage",
            "room_id":room_id,
            "bytes_to_free":u64::MAX
        }))
        .unwrap();
    assert_eq!(result["complete"], false);
    assert_eq!(result["pinned_originals_preserved"], 1);
    assert_eq!(result["evicted_asset_ids"], json!([first_id]));
    assert!(result["bytes_freed"].as_u64().unwrap() > 0);
    assert_eq!(
        result["local_bytes_after"],
        result["local_bytes_before"].as_u64().unwrap() - result["bytes_freed"].as_u64().unwrap()
    );

    let snapshot = core.snapshot().unwrap();
    let assets = snapshot["rooms"][0]["assets"].as_array().unwrap();
    let evicted = assets.iter().find(|asset| asset["id"] == first_id).unwrap();
    let pinned = assets
        .iter()
        .find(|asset| asset["id"] == pinned_id)
        .unwrap();
    assert_eq!(evicted["local"], false);
    assert_eq!(evicted["verified_copies"], 1);
    assert_eq!(pinned["local"], true);
    assert_eq!(pinned["pinned_offline"], true);
}

#[test]
fn automatic_cache_limit_waits_for_a_verified_copy_and_preserves_pins() {
    let (dir, mut core) = vault();
    let archive = tempfile::tempdir().unwrap();
    let room_id = room(&mut core);
    let older_id = import(&mut core, &room_id, &media(&dir));
    let pinned_path = dir.path().join("pinned.jpg");
    fs::write(&pinned_path, b"keep this original on this device").unwrap();
    let pinned_id = import(&mut core, &room_id, &pinned_path);
    core.execute(json!({
        "action":"pin_original", "room_id":room_id, "asset_id":pinned_id
    }))
    .unwrap();
    let pinned_bytes = fs::metadata(core.blob(&room_id, &pinned_id).unwrap())
        .unwrap()
        .len();
    let room_operations_before_policy = core.room(&room_id).unwrap().operations.len();

    let configured = core
        .execute(json!({
            "action":"set_device_cache_limit",
            "room_id":room_id,
            "max_bytes":pinned_bytes
        }))
        .unwrap();
    assert_eq!(configured["enabled"], true);
    assert_eq!(configured["cleanup"]["complete"], false);
    assert!(core.blob(&room_id, &older_id).unwrap().is_file());
    assert_eq!(
        core.room(&room_id).unwrap().operations.len(),
        room_operations_before_policy,
        "cache preferences are local device state, not Room history"
    );

    core.execute(json!({
        "action":"add_source",
        "room_id":room_id,
        "name":"Home archive",
        "kind":"local",
        "endpoint":archive.path()
    }))
    .unwrap();
    let sync = core
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();
    assert_eq!(sync["cache_cleanup"]["enabled"], true);
    assert_eq!(sync["cache_cleanup"]["complete"], true);
    assert!(!core.blob(&room_id, &older_id).unwrap().exists());
    assert!(core.blob(&room_id, &pinned_id).unwrap().is_file());

    let snapshot = core.snapshot().unwrap();
    assert_eq!(
        snapshot["rooms"][0]["device_cache_limit_bytes"],
        pinned_bytes
    );
    let assets = snapshot["rooms"][0]["assets"].as_array().unwrap();
    assert_eq!(
        assets
            .iter()
            .find(|asset| asset["id"] == pinned_id)
            .unwrap()["pinned_offline"],
        true
    );
    assert_eq!(
        assets.iter().find(|asset| asset["id"] == older_id).unwrap()["verified_copies"],
        1
    );
    drop(core);
    let reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    assert_eq!(
        reopened.snapshot().unwrap()["rooms"][0]["device_cache_limit_bytes"],
        pinned_bytes
    );
}

#[test]
fn eviction_rejects_a_stale_replica_that_fails_integrity_verification() {
    let (dir, mut core) = vault();
    let remote = tempfile::tempdir().unwrap();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &media(&dir));
    core.execute(json!({
        "action":"add_source",
        "room_id":room_id,
        "name":"Local archive",
        "kind":"local",
        "endpoint":remote.path()
    }))
    .unwrap();
    let source_id = core.snapshot().unwrap()["sources"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    core.execute(json!({"action":"replicate","source_id":source_id,"asset_id":asset_id}))
        .unwrap();
    fs::write(
        remote.path().join(format!("{room_id}-{asset_id}.frblob")),
        b"tampered remote copy",
    )
    .unwrap();

    assert!(core
        .execute(json!({
            "action":"evict_original",
            "room_id":room_id,
            "asset_id":asset_id
        }))
        .is_err());
    let snapshot = core.snapshot().unwrap();
    assert_eq!(snapshot["rooms"][0]["assets"][0]["local"], true);
    assert_eq!(snapshot["rooms"][0]["assets"][0]["verified_copies"], 1);
    assert!(core
        .state
        .replicas
        .iter()
        .all(|replica| replica.asset_id != asset_id || !replica.verified));
}

#[test]
fn eviction_respects_storage_network_policy_without_touching_the_local_copy() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &media(&dir));
    let source_id = core
        .execute(json!({
            "action":"add_source",
            "room_id":room_id,
            "name":"Metered cloud",
            "kind":"webdav",
            "endpoint":"https://example.invalid/vault",
            "token":"test-token"
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    core.execute(json!({
        "action":"set_source_network_policy",
        "id":source_id,
        "policy":"unmetered_only"
    }))
    .unwrap();
    core.execute(json!({"action":"set_network_state","state":"metered"}))
        .unwrap();
    core.state.replicas.push(crate::model::Replica {
        asset_id: asset_id.clone(),
        source_id,
        verified: true,
    });

    let error = core
        .execute(json!({
            "action":"evict_original",
            "room_id":room_id,
            "asset_id":asset_id
        }))
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("waiting for an unmetered network"));
    let asset = &core.snapshot().unwrap()["rooms"][0]["assets"][0];
    assert_eq!(asset["local"], true);
    assert_eq!(asset["pinned_offline"], false);
    assert_eq!(asset["verified_copies"], 2);
}
fn vault() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path(), [7; 32]).unwrap();
    (dir, engine)
}

#[test]
fn metered_policy_queues_media_without_contacting_remote_storage() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &media(&dir));
    let source_id = core.execute(json!({"action":"add_source","room_id":room_id,
        "name":"Remote","kind":"webdav","endpoint":"http://127.0.0.1:1/vault","token":"test-token"}))
        .unwrap()["id"].as_str().unwrap().to_owned();
    assert!(core
        .execute(json!({"action":"set_source_network_policy","id":source_id,"policy":"wifi"}))
        .is_err());
    core.execute(
        json!({"action":"set_source_network_policy","id":source_id,"policy":"unmetered_only"}),
    )
    .unwrap();
    core.execute(json!({"action":"set_network_state","state":"metered"}))
        .unwrap();
    let job = core
        .execute(json!({"action":"replicate_step","source_id":source_id,"asset_id":asset_id}))
        .unwrap();
    assert_eq!(job["state"], "waiting");
    assert_eq!(job["error"], "Upload waiting for an unmetered network");
    assert_eq!(
        core.snapshot().unwrap()["rooms"][0]["assets"][0]["local"],
        true
    );
    assert_eq!(
        core.snapshot().unwrap()["sources"][0]["network_policy"],
        "unmetered_only"
    );
    assert_eq!(core.snapshot().unwrap()["transfers"][0]["state"], "waiting");
    drop(core);
    let mut reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    assert_eq!(
        reopened.snapshot().unwrap()["sources"][0]["network_policy"],
        "unmetered_only"
    );
    let job = reopened
        .execute(json!({"action":"retry_transfer","id":job["id"]}))
        .unwrap();
    assert_eq!(job["state"], "waiting");
    assert_eq!(job["error"], "Upload waiting for an unmetered network");
    reopened
        .execute(json!({"action":"set_network_state","state":"offline"}))
        .unwrap();
    let job = reopened
        .execute(json!({"action":"replicate","source_id":source_id,"asset_id":asset_id}))
        .unwrap();
    assert_eq!(job["error"], "Upload waiting for a network");
    fs::remove_file(reopened.blob(&room_id, &asset_id).unwrap()).unwrap();
    let error = reopened
        .execute(json!({"action":"fetch","room_id":room_id,"asset_id":asset_id}))
        .unwrap_err();
    assert!(error.to_string().contains("Download waiting for a network"));
    reopened
        .execute(json!({"action":"set_network_state","state":"metered"}))
        .unwrap();
    let error = reopened
        .execute(json!({"action":"fetch","room_id":room_id,"asset_id":asset_id}))
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("Download waiting for an unmetered network"));
}

#[test]
fn offline_original_pins_are_local_persistent_and_do_not_change_room_history() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &media(&dir));
    let operations_before = core.room(&room_id).unwrap().operations.len();

    core.execute(json!({
        "action":"pin_original",
        "room_id":room_id,
        "asset_id":asset_id
    }))
    .unwrap();
    core.execute(json!({
        "action":"pin_original",
        "room_id":room_id,
        "asset_id":asset_id
    }))
    .unwrap();

    let pinned = core.snapshot().unwrap();
    assert_eq!(pinned["rooms"][0]["assets"][0]["pinned_offline"], true);
    assert_eq!(
        core.room(&room_id).unwrap().operations.len(),
        operations_before
    );

    drop(core);
    let mut reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    let restored = reopened.snapshot().unwrap();
    assert_eq!(restored["rooms"][0]["assets"][0]["pinned_offline"], true);
    assert_eq!(restored["rooms"][0]["assets"][0]["local"], true);

    reopened
        .execute(json!({
            "action":"unpin_original",
            "room_id":room_id,
            "asset_id":asset_id
        }))
        .unwrap();
    let unpinned = reopened.snapshot().unwrap();
    assert_eq!(unpinned["rooms"][0]["assets"][0]["pinned_offline"], false);
    assert_eq!(unpinned["rooms"][0]["assets"][0]["local"], true);
}

#[test]
fn older_encrypted_catalog_shape_defaults_to_no_offline_pins() {
    let state: crate::model::State = serde_json::from_value(json!({
        "version": 1,
        "signing_seed": "legacy-seed",
        "rooms": [],
        "sources": [],
        "watches": [],
        "transfers": [],
        "replicas": []
    }))
    .unwrap();

    assert!(state.offline_pins.is_empty());
    assert!(state.device_cache_limits.is_empty());
}

#[test]
fn corrupt_local_original_is_never_marked_as_an_offline_pin() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &media(&dir));
    let original = core.blob(&room_id, &asset_id).unwrap();
    fs::write(original, b"corrupted original bytes").unwrap();

    let result = core.execute(json!({
        "action":"pin_original",
        "room_id":room_id,
        "asset_id":asset_id
    }));

    assert!(result.is_err());
    let asset = &core.snapshot().unwrap()["rooms"][0]["assets"][0];
    assert_eq!(asset["local"], false);
    assert_eq!(asset["damaged"], true);
    assert_eq!(asset["pinned_offline"], false);
}

#[test]
fn pinning_a_remote_only_original_fetches_and_verifies_before_pinning() {
    let (dir, mut core) = vault();
    let remote = tempfile::tempdir().unwrap();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &media(&dir));
    let source_id = core
        .execute(json!({
            "action":"add_source",
            "room_id":room_id,
            "name":"Local archive",
            "kind":"local",
            "endpoint":remote.path()
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    core.execute(json!({"action":"replicate","source_id":source_id,"asset_id":asset_id}))
        .unwrap();
    fs::remove_file(core.blob(&room_id, &asset_id).unwrap()).unwrap();

    core.execute(json!({
        "action":"pin_original",
        "room_id":room_id,
        "asset_id":asset_id
    }))
    .unwrap();

    let asset = &core.snapshot().unwrap()["rooms"][0]["assets"][0];
    assert_eq!(asset["local"], true);
    assert_eq!(asset["pinned_offline"], true);
    assert_eq!(asset["verified_copies"], 2);
}

#[test]
fn offline_sync_queues_pending_media_without_remote_catalog_request() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &media(&dir));
    let source_id = core.execute(json!({"action":"add_source","room_id":room_id,
        "name":"Remote","kind":"webdav","endpoint":"http://127.0.0.1:1/vault","token":"test-token"}))
        .unwrap()["id"].as_str().unwrap().to_owned();
    core.execute(json!({"action":"set_network_state","state":"offline"}))
        .unwrap();
    let synced = core
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();
    assert_eq!(synced["errors"][0], "Waiting for a network");
    let snapshot = core.snapshot().unwrap();
    assert_eq!(snapshot["transfers"][0]["source_id"], source_id);
    assert_eq!(snapshot["transfers"][0]["asset_id"], asset_id);
    assert_eq!(snapshot["transfers"][0]["state"], "waiting");
    assert_eq!(snapshot["rooms"][0]["assets"][0]["local"], true);
}

#[test]
fn video_search_finds_sparse_evidence_in_a_full_size_index() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let movie = dir.path().join("long-recording.mp4");
    fs::write(&movie, b"video-original-for-index-test").unwrap();
    let asset_id = import(&mut core, &room_id, &movie);
    let mut observations: Vec<_> = (0..14_999_u64)
        .map(|position| {
            json!({"modality":"screen_text","start_ms":position * 5_000,
            "end_ms":position * 5_000,"text":"ordinary kitchen","confidence_percent":75})
        })
        .collect();
    observations.push(json!({"modality":"visual_label","start_ms":74_995_000,
        "end_ms":74_995_000,"text":"blue bicycle","confidence_percent":95}));
    core.execute(
        json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
        "audio_indexed":false,"frames_indexed":true,"frame_interval_ms":5_000,
        "observations":observations}),
    )
    .unwrap();
    let found = core
        .execute(json!({"action":"search_video","room_id":room_id,
        "query":"find the blue bicycle","modality":"frame"}))
        .unwrap();
    assert_eq!(found["matches"].as_array().unwrap().len(), 1);
    assert_eq!(found["matches"][0]["asset_id"], asset_id);
    assert_eq!(found["matches"][0]["start_ms"], 74_995_000);
    assert_eq!(found["matches"][0]["evidence"][0]["text"], "blue bicycle");
    let indexes: Vec<_> = core
        .state
        .video_indexes
        .iter()
        .filter(|index| index.room_id == room_id)
        .collect();
    let reference = crate::video_index::search(
        &indexes,
        &core.assets(&room_id).unwrap(),
        "find the blue bicycle",
        "frame",
    );
    assert_eq!(found, reference);
}

#[test]
fn video_postings_refresh_after_reindex_clear_room_switch_and_reopen() {
    let (dir, mut core) = vault();
    let room_a = room(&mut core);
    let movie_a = dir.path().join("first.mp4");
    fs::write(&movie_a, b"first-video-original").unwrap();
    let asset_a = import(&mut core, &room_a, &movie_a);
    let index = |core: &mut Engine, room_id: &str, asset_id: &str, words: &str| {
        core.execute(
            json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
            "frames_indexed":true,"frame_interval_ms":2_000,
            "observations":[{"modality":"screen_text","start_ms":2_000,
                "end_ms":2_000,"text":words,"confidence_percent":90}]}),
        )
        .unwrap();
    };
    let search = |core: &mut Engine, room_id: &str, words: &str| {
        core.execute(json!({"action":"search_video","room_id":room_id,
            "query":words,"modality":"frame"}))
            .unwrap()
    };
    index(&mut core, &room_a, &asset_a, "red kite");
    assert_eq!(
        search(&mut core, &room_a, "red kite")["matches"][0]["asset_id"],
        asset_a
    );
    let catalog_before_reads = fs::read(dir.path().join("catalog.fr")).unwrap();
    let _ = search(&mut core, &room_a, "red kite");
    core.execute(json!({"action":"snapshot"})).unwrap();
    core.execute(json!({"action":"video_index_status","room_id":room_a}))
        .unwrap();
    core.execute(json!({"action":"set_network_state","state":"offline"}))
        .unwrap();
    assert_eq!(
        fs::read(dir.path().join("catalog.fr")).unwrap(),
        catalog_before_reads
    );
    index(&mut core, &room_a, &asset_a, "blue boat");
    assert!(search(&mut core, &room_a, "red kite")["matches"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        search(&mut core, &room_a, "blue boat")["matches"][0]["asset_id"],
        asset_a
    );
    core.execute(json!({"action":"clear_video_index","room_id":room_a,"asset_id":asset_a}))
        .unwrap();
    assert!(search(&mut core, &room_a, "blue boat")["matches"]
        .as_array()
        .unwrap()
        .is_empty());
    index(&mut core, &room_a, &asset_a, "blue boat");
    let room_b = room(&mut core);
    let movie_b = dir.path().join("second.mp4");
    fs::write(&movie_b, b"second-video-original").unwrap();
    let asset_b = import(&mut core, &room_b, &movie_b);
    index(&mut core, &room_b, &asset_b, "blue boat");
    let found_b = search(&mut core, &room_b, "blue boat");
    assert_eq!(found_b["matches"][0]["room_id"], room_b);
    assert_eq!(found_b["matches"][0]["asset_id"], asset_b);
    assert_eq!(
        search(&mut core, &room_a, "blue boat")["matches"][0]["asset_id"],
        asset_a
    );
    drop(core);
    let mut reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    assert_eq!(
        search(&mut reopened, &room_a, "blue boat")["matches"][0]["asset_id"],
        asset_a
    );
}

#[test]
fn room_likes_converge_per_member_and_survive_reopen() {
    let (owner_dir, mut owner) = vault();
    let (viewer_dir, mut viewer) = vault();
    let room_id = room(&mut owner);
    let asset_id = import(&mut owner, &room_id, &media(&owner_dir));
    invite(&mut owner, &mut viewer, &room_id, Role::Viewer);
    viewer
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();

    let owner_like = owner
        .execute(json!({"action":"set_like","room_id":room_id,
            "asset_id":asset_id,"liked":true}))
        .unwrap();
    let operation_count = owner.room(&room_id).unwrap().operations.len();
    assert_eq!(
        owner
            .execute(json!({"action":"set_like","room_id":room_id,
        "asset_id":asset_id,"liked":true}))
            .unwrap()["id"],
        owner_like["id"]
    );
    assert_eq!(
        owner.room(&room_id).unwrap().operations.len(),
        operation_count
    );
    let viewer_like = viewer
        .execute(json!({"action":"set_like","room_id":room_id,
            "asset_id":asset_id,"liked":true}))
        .unwrap();
    assert_ne!(owner_like["id"], viewer_like["id"]);
    owner
        .merge_packet(&room_id, &viewer.packet(&room_id).unwrap())
        .unwrap();
    viewer
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["reactions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|reaction| reaction["liked"] == true)
            .count(),
        2
    );

    viewer
        .execute(json!({"action":"set_like","room_id":room_id,
            "asset_id":asset_id,"liked":false}))
        .unwrap();
    owner
        .merge_packet(&room_id, &viewer.packet(&room_id).unwrap())
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["reactions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|reaction| reaction["liked"] == true)
            .count(),
        1
    );
    assert!(viewer
        .execute(json!({"action":"set_like","room_id":room_id,
            "asset_id":"missing","liked":true}))
        .is_err());
    assert!(viewer
        .execute(json!({"action":"set_like","room_id":room_id,
            "asset_id":asset_id,"liked":"yes"}))
        .is_err());
    let mut forged = viewer
        .room(&room_id)
        .unwrap()
        .operations
        .iter()
        .rev()
        .find(|operation| operation.kind == "reaction")
        .unwrap()
        .clone();
    forged.id = engine::id();
    forged.clock += 1;
    forged.value["author"] = json!(owner.device().unwrap());
    signing::sign_operation(&mut forged, &viewer.state.signing_seed).unwrap();
    assert!(owner.merge(&room_id, vec![forged]).is_err());
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["reactions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|reaction| reaction["liked"] == true)
            .count(),
        1
    );
    drop(viewer);
    let reopened = Engine::open(viewer_dir.path(), [7; 32]).unwrap();
    assert_eq!(
        reopened.snapshot().unwrap()["rooms"][0]["reactions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|reaction| reaction["liked"] == true)
            .count(),
        1
    );
}
#[test]
fn video_reindex_preserves_other_modality_only_when_requested() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("speech-and-frames.mp4");
    fs::write(&movie, b"original video for partial reindex").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);

    owner.execute(json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
        "audio_indexed":true,"frames_indexed":false,"observations":[
            {"modality":"speech","start_ms":1200,"end_ms":1800,"text":"yellow balloon","confidence_percent":90}
        ]})).unwrap();
    owner.execute(json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
        "audio_indexed":false,"frames_indexed":true,"frame_interval_ms":2000,
        "preserve_existing":true,"observations":[
            {"modality":"screen_text","start_ms":3000,"end_ms":3000,"text":"HAPPY BIRTHDAY","confidence_percent":92}
        ]})).unwrap();
    for (query, modality) in [("yellow balloon", "audio"), ("happy birthday", "frame")] {
        assert_eq!(
            owner
                .execute(json!({"action":"search_video","room_id":room_id,
            "query":query,"modality":modality}))
                .unwrap()["matches"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    owner
        .execute(
            json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
        "audio_indexed":true,"frames_indexed":false,"preserve_existing":true,
        "observations":[{"modality":"speech","start_ms":4000,"end_ms":4500,
            "text":"blue bicycle","confidence_percent":85}]}),
        )
        .unwrap();
    assert!(owner
        .execute(json!({"action":"search_video","room_id":room_id,
        "query":"yellow balloon","modality":"audio"}))
        .unwrap()["matches"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        owner
            .execute(json!({"action":"search_video","room_id":room_id,
        "query":"happy birthday","modality":"frame"}))
            .unwrap()["matches"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    owner
        .execute(
            json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
        "audio_indexed":false,"frames_indexed":true,"frame_interval_ms":2000,
        "observations":[{"modality":"visual_label","start_ms":5000,"end_ms":5000,
            "text":"garden","confidence_percent":80}]}),
        )
        .unwrap();
    assert!(owner
        .execute(json!({"action":"search_video","room_id":room_id,
        "query":"blue bicycle","modality":"audio"}))
        .unwrap()["matches"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn video_search_finds_time_stamped_audio_and_frame_evidence_locally() {
    let (dir, mut owner) = vault();
    let (viewer_dir, mut viewer) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("party.mp4");
    fs::write(&movie, b"movie original for index contract").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    let observations = json!([
        {"modality":"speech","start_ms":12000,"end_ms":14000,
            "text":"We found the yellow balloon","confidence_percent":91},
        {"modality":"screen_text","start_ms":12500,"end_ms":12500,
            "text":"HAPPY BIRTHDAY","confidence_percent":96},
        {"modality":"visual_label","start_ms":12500,"end_ms":12500,
            "text":"balloon","confidence_percent":88},
        {"modality":"speech","start_ms":60000,"end_ms":62000,
            "text":"Meet us in the garden","confidence_percent":90}
    ]);
    owner
        .execute(
            json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
        "audio_indexed":true,"frames_indexed":true,"frame_interval_ms":2000,
        "observations":observations}),
        )
        .unwrap();
    let transcript = owner
        .execute(json!({
            "action":"export_video_transcript",
            "room_id":room_id,
            "asset_id":asset_id,
        }))
        .unwrap();
    assert_eq!(transcript["cue_count"], 2);
    assert!(transcript["webvtt"]
        .as_str()
        .unwrap()
        .contains("00:00:12.000 --> 00:00:14.000\nWe found the yellow balloon"));
    assert!(transcript["webvtt"]
        .as_str()
        .unwrap()
        .contains("00:01:00.000 --> 00:01:02.000\nMeet us in the garden"));
    let status = owner
        .execute(json!({"action":"video_index_status","room_id":room_id}))
        .unwrap();
    assert_eq!(status["videos"][0]["indexed"], true);
    assert_eq!(status["videos"][0]["audio_indexed"], true);
    let found = owner
        .execute(json!({"action":"search_video","room_id":room_id,
        "query":"find video with happy birthday balloon"}))
        .unwrap();
    assert_eq!(found["matches"].as_array().unwrap().len(), 1);
    assert_eq!(found["matches"][0]["asset_id"], asset_id);
    assert_eq!(found["matches"][0]["start_ms"], 12000);
    assert_eq!(found["matches"][0]["evidence"].as_array().unwrap().len(), 3);
    assert_eq!(found["coverage"]["indexed"], 1);
    let conversational = owner.execute(json!({"action":"search_video","room_id":room_id,
  "query":"show me the video where someone says yellow balloon while happy birthday appears on screen"})).unwrap();
    assert_eq!(conversational["matches"][0]["start_ms"], 12000);
    let quoted = owner
        .execute(json!({"action":"search_video","room_id":room_id,
 "query":"Where does grandma say ‘yellow balloon’?","modality":"audio"}))
        .unwrap();
    assert_eq!(quoted["matches"].as_array().unwrap().len(), 1,
 "A quoted phrase is searchable when surrounding speaker context is absent from the transcript.");
    assert_eq!(quoted["matches"][0]["start_ms"], 12000);
    let sign_question = owner
        .execute(json!({
            "action":"search_video","room_id":room_id,
            "query":"What does the sign say?","modality":"all"
        }))
        .unwrap();
    assert_eq!(sign_question["matches"].as_array().unwrap().len(), 1);
    assert_eq!(sign_question["matches"][0]["start_ms"], 12500);
    assert_eq!(sign_question["matches"][0]["match_kind"], "evidence");
    assert!(sign_question["answer"]
        .as_str()
        .unwrap()
        .contains("HAPPY BIRTHDAY"));
    assert!(sign_question["answer"]
        .as_str()
        .unwrap()
        .contains("00:12.5"));
    assert!(sign_question["matches"][0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evidence| evidence["text"] == "HAPPY BIRTHDAY"));

    let sign_audio_only = owner
        .execute(json!({
            "action":"search_video","room_id":room_id,
            "query":"What does the sign say?","modality":"audio"
        }))
        .unwrap();
    assert!(sign_audio_only["matches"].as_array().unwrap().is_empty());

    let speech_question = owner
        .execute(json!({
            "action":"search_video","room_id":room_id,
            "query":"What did Grandma say?","modality":"all"
        }))
        .unwrap();
    assert_eq!(speech_question["matches"].as_array().unwrap().len(), 2);
    assert!(speech_question["matches"]
        .as_array()
        .unwrap()
        .iter()
        .all(|match_| match_["match_kind"] == "evidence"));
    assert!(speech_question["answer"]
        .as_str()
        .unwrap()
        .contains("We found the yellow balloon"));
    assert!(speech_question["matches"]
        .as_array()
        .unwrap()
        .iter()
        .all(|match_| {
            match_["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .all(|evidence| evidence["modality"] == "speech")
        }));

    let audio_only = owner
        .execute(json!({"action":"search_video","room_id":room_id,
        "query":"happy birthday balloon","modality":"audio"}))
        .unwrap();
    assert!(audio_only["matches"].as_array().unwrap().is_empty());
    let frame_only = owner
        .execute(json!({"action":"search_video","room_id":room_id,
        "query":"happy birthday balloon","modality":"frame"}))
        .unwrap();
    assert_eq!(frame_only["matches"].as_array().unwrap().len(), 1);
    let sign_text = owner
        .execute(json!({"action":"search_video","room_id":room_id,
        "query":"What does the birthday sign say?","modality":"frame"}))
        .unwrap();
    assert_eq!(sign_text["matches"].as_array().unwrap().len(), 1);
    assert_eq!(sign_text["matches"][0]["start_ms"], 12500);
    for (query, modality) in [
        ("happy birthday balloon", "all"),
        ("happy birthday balloon", "audio"),
        ("happy birthday balloon", "frame"),
        ("What does the birthday sign say?", "frame"),
        ("What does the sign say?", "all"),
        ("What does the sign say?", "audio"),
        ("What's written on screen?", "all"),
        ("What did Grandma say?", "all"),
        ("yellow balloon", "all"),
        ("missing bicycle", "all"),
        ("Where does grandma say \"yellow balloon\"?", "audio"),
        ("Where did grandma say 'yellow balloon'?", "audio"),
    ] {
        let reference = {
            let indexes: Vec<_> = owner
                .state
                .video_indexes
                .iter()
                .filter(|index| index.room_id == room_id)
                .collect();
            crate::video_index::search(&indexes, &owner.assets(&room_id).unwrap(), query, modality)
        };
        let cached = owner
            .execute(json!({"action":"search_video","room_id":room_id,
            "query":query,"modality":modality}))
            .unwrap();
        assert_eq!(cached, reference, "{query} ({modality})");
    }
    assert!(!fs::read(dir.path().join("catalog.fr"))
        .unwrap()
        .windows(b"HAPPY BIRTHDAY".len())
        .any(|window| window == b"HAPPY BIRTHDAY"));
    invite(&mut owner, &mut viewer, &room_id, Role::Viewer);
    viewer
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();
    let remote = viewer
        .execute(json!({"action":"search_video","room_id":room_id,
        "query":"happy birthday"}))
        .unwrap();
    assert_eq!(remote["coverage"]["indexed"], 0);
    drop(viewer_dir);
    drop(owner);
    let mut reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    assert_eq!(
        reopened
            .execute(json!({"action":"search_video","room_id":room_id,
        "query":"garden"}))
            .unwrap()["matches"][0]["start_ms"],
        60000
    );
    reopened
        .execute(json!({"action":"clear_video_index","room_id":room_id,
        "asset_id":asset_id}))
        .unwrap();
    assert_eq!(
        reopened
            .execute(json!({"action":"search_video","room_id":room_id,
        "query":"garden"}))
            .unwrap()["coverage"]["indexed"],
        0
    );
}

#[test]
fn video_search_follow_up_uses_previous_question_as_local_context() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("birthday.mp4");
    fs::write(&movie, b"indexed local video for follow-up search").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":true, "frames_indexed":true, "frame_interval_ms":2000,
            "observations":[{
                "modality":"speech", "start_ms":12000, "end_ms":14000,
                "text":"I see candles beside the yellow balloon", "confidence_percent":92
            }]
        }))
        .unwrap();

    let without_context = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"What about the candles?", "modality":"all"
        }))
        .unwrap();
    assert_eq!(without_context["matches"].as_array().unwrap().len(), 1);
    assert_eq!(without_context["matches"][0]["query_terms"], 1);

    let follow_up = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"What about the candles?", "context_query":"Find the yellow balloon",
            "modality":"all"
        }))
        .unwrap();
    assert_eq!(follow_up["contextual_search"], true);
    assert!(follow_up["answer"]
        .as_str()
        .unwrap()
        .contains("previous video question"));
    assert_eq!(follow_up["matches"].as_array().unwrap().len(), 1);
    assert_eq!(follow_up["matches"][0]["asset_id"], asset_id);
    assert_eq!(follow_up["matches"][0]["match_kind"], "suggestion");
    assert_eq!(follow_up["matches"][0]["query_terms"], 3);
    assert_eq!(follow_up["matches"][0]["matched_terms"], 3);

    let exact = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"What does she say ‘yellow balloon’?",
            "context_query":"Find the birthday candles", "modality":"audio"
        }))
        .unwrap();
    assert!(!exact["contextual_search"].as_bool().unwrap_or(false));
    assert_eq!(exact["matches"][0]["match_kind"], "exact");
}

#[test]
fn video_search_returns_transparent_partial_suggestions_but_keeps_exact_queries_strict() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("park.mp4");
    fs::write(&movie, b"video for partial evidence search").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":false, "frames_indexed":true, "frame_interval_ms":2_000,
            "observations":[
                {"modality":"visual_label", "start_ms":12_000, "end_ms":12_000,
                 "text":"dog", "confidence_percent":91},
                {"modality":"screen_text", "start_ms":12_500, "end_ms":12_500,
                 "text":"ball", "confidence_percent":96}
            ]
        }))
        .unwrap();

    let suggested = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find a dog playing with a ball", "modality":"frame"
        }))
        .unwrap();
    assert_eq!(
        suggested["matches"].as_array().unwrap().len(),
        1,
        "expected a closest indexed match, got {suggested}"
    );
    assert_eq!(suggested["matches"][0]["match_kind"], "suggestion");
    assert_eq!(suggested["matches"][0]["matched_terms"], 2);
    assert_eq!(suggested["matches"][0]["query_terms"], 3);
    assert!(suggested["answer"]
        .as_str()
        .unwrap()
        .contains("No exact match"));
    assert!(suggested["matches"][0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .all(|evidence| evidence["modality"] == "screen_text"
            || evidence["modality"] == "visual_label"));

    let indexes: Vec<_> = owner
        .state
        .video_indexes
        .iter()
        .filter(|index| index.room_id == room_id)
        .collect();
    assert_eq!(
        suggested,
        crate::video_index::search(
            &indexes,
            &owner.assets(&room_id).unwrap(),
            "Find a dog playing with a ball",
            "frame"
        )
    );

    let wrong_modality = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find a dog playing with a ball", "modality":"audio"
        }))
        .unwrap();
    assert!(wrong_modality["matches"].as_array().unwrap().is_empty());

    let quoted = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"\"dog playing with a ball\"", "modality":"frame"
        }))
        .unwrap();
    assert!(quoted["matches"].as_array().unwrap().is_empty());

    let too_few_terms = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find the dog playing catch with a ball at the park", "modality":"frame"
        }))
        .unwrap();
    assert!(too_few_terms["matches"].as_array().unwrap().is_empty());

    let exact_movie = dir.path().join("exact-park.mp4");
    fs::write(&exact_movie, b"video with every requested indexed term").unwrap();
    let exact_asset_id = import(&mut owner, &room_id, &exact_movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":exact_asset_id,
            "audio_indexed":false, "frames_indexed":true, "frame_interval_ms":2_000,
            "observations":[
                {"modality":"visual_label", "start_ms":30_000, "end_ms":30_000,
                 "text":"dog playing ball", "confidence_percent":97}
            ]
        }))
        .unwrap();
    let exact = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find a dog playing with a ball", "modality":"frame"
        }))
        .unwrap();
    let matches = exact["matches"].as_array().unwrap();
    assert!(matches
        .iter()
        .any(|result| result["asset_id"] == exact_asset_id));
    assert!(matches
        .iter()
        .all(|result| result["match_kind"] == "suggestion"));
}

#[test]
fn video_search_finds_visual_actions_as_timestamped_suggestions() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("birthday.mp4");
    fs::write(&movie, b"video with a locally described visual moment").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":false, "frames_indexed":true, "visual_labels_indexed":true,
            "frame_interval_ms":2_000,
            "observations":[{
                "modality":"visual_label", "start_ms":24_000, "end_ms":24_000,
                "text":"child blowing candles on a birthday cake", "confidence_percent":55
            }]
        }))
        .unwrap();

    let result = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find the moment with a child blowing candles", "modality":"frame"
        }))
        .unwrap();

    assert_eq!(result["matches"].as_array().unwrap().len(), 1);
    assert_eq!(result["matches"][0]["match_kind"], "suggestion");
    assert_eq!(result["matches"][0]["start_ms"], 24_000);
    assert_eq!(
        result["matches"][0]["evidence"][0]["modality"],
        "visual_label"
    );
    assert!(result["answer"]
        .as_str()
        .unwrap()
        .contains("No exact match"));
}

#[test]
fn video_search_matches_common_word_forms_but_keeps_quoted_phrases_literal() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("birthday-action.mp4");
    fs::write(&movie, b"video with a locally described visual moment").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":true, "frames_indexed":true, "visual_labels_indexed":true,
            "frame_interval_ms":2_000,
            "observations":[
                {
                    "modality":"visual_label", "start_ms":24_000, "end_ms":24_000,
                    "text":"child blowing out a candle", "confidence_percent":55
                },
                {
                    "modality":"speech", "start_ms":31_000, "end_ms":33_000,
                    "text":"We blew out the candle together", "confidence_percent":90
                }
            ]
        }))
        .unwrap();

    let paraphrase = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find the kid who blows out candles", "modality":"frame"
        }))
        .unwrap();
    assert_eq!(paraphrase["matches"].as_array().unwrap().len(), 1);
    assert_eq!(paraphrase["matches"][0]["asset_id"], asset_id);
    assert_eq!(paraphrase["matches"][0]["start_ms"], 24_000);
    assert_eq!(paraphrase["matches"][0]["match_kind"], "suggestion");
    let indexes: Vec<_> = owner
        .state
        .video_indexes
        .iter()
        .filter(|index| index.room_id == room_id)
        .collect();
    assert_eq!(
        paraphrase,
        crate::video_index::search(
            &indexes,
            &owner.assets(&room_id).unwrap(),
            "Find the kid who blows out candles",
            "frame"
        )
    );

    let spoken = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Where did someone blow out a candle?", "modality":"audio"
        }))
        .unwrap();
    assert_eq!(spoken["matches"].as_array().unwrap().len(), 1);
    assert_eq!(spoken["matches"][0]["asset_id"], asset_id);
    assert_eq!(spoken["matches"][0]["start_ms"], 31_000);
    assert_eq!(spoken["matches"][0]["match_kind"], "suggestion");
    assert_eq!(spoken["matches"][0]["evidence"][0]["modality"], "speech");

    let exact_speech = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find ‘blew out the candle’", "modality":"audio"
        }))
        .unwrap();
    assert_eq!(exact_speech["matches"].as_array().unwrap().len(), 1);
    assert_eq!(exact_speech["matches"][0]["start_ms"], 31_000);
    assert_eq!(exact_speech["matches"][0]["match_kind"], "exact");

    let literal = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find ‘kid blows out candles’", "modality":"frame"
        }))
        .unwrap();
    assert!(literal["matches"].as_array().unwrap().is_empty());
}

#[test]
fn moment_suggestions_are_read_only_and_use_room_capture_times() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let mut asset_ids = Vec::new();
    for (index, captured_at) in [1_000_i64, 1_120, 1_240].into_iter().enumerate() {
        let path = dir.path().join(format!("memory-{index}.jpg"));
        fs::write(&path, format!("photo-{index}")).unwrap();
        let asset_id = import(&mut owner, &room_id, &path);
        owner
            .execute(json!({
                "action":"patch_asset", "room_id":room_id, "asset_id":asset_id,
                "fields":{"captured_at":captured_at}
            }))
            .unwrap();
        asset_ids.push(asset_id);
    }
    let catalog_before = fs::read(dir.path().join("catalog.fr")).unwrap();

    let response = owner
        .execute(json!({"action":"suggest_moments", "room_id":room_id}))
        .unwrap();

    assert_eq!(response["suggestions"].as_array().unwrap().len(), 1);
    assert_eq!(response["suggestions"][0]["start"], 1_000);
    assert_eq!(response["suggestions"][0]["end"], 1_240);
    assert_eq!(response["suggestions"][0]["asset_ids"], json!(asset_ids));
    assert_eq!(
        catalog_before,
        fs::read(dir.path().join("catalog.fr")).unwrap()
    );
    assert!(owner.snapshot().unwrap()["rooms"][0]["moments"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn video_search_exact_phrase_seeks_to_the_phrase_not_unrelated_nearby_evidence() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("birthday.mp4");
    fs::write(&movie, b"video for precise phrase seeking").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":false, "frames_indexed":true, "frame_interval_ms":2000,
            "observations":[
                {"modality":"screen_text", "start_ms":7000, "end_ms":7000,
                 "text":"Earlier scene", "confidence_percent":94},
                {"modality":"screen_text", "start_ms":10000, "end_ms":10000,
                 "text":"HAPPY", "confidence_percent":96},
                {"modality":"screen_text", "start_ms":10500, "end_ms":10500,
                 "text":"BIRTHDAY", "confidence_percent":97}
            ]
        }))
        .unwrap();

    let result = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find the exact on-screen text “HAPPY BIRTHDAY”", "modality":"frame"
        }))
        .unwrap();

    assert_eq!(result["matches"].as_array().unwrap().len(), 1);
    assert_eq!(result["matches"][0]["start_ms"], 10000);
    assert_eq!(result["matches"][0]["end_ms"], 10500);
    assert!(result["matches"][0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .all(|evidence| evidence["text"] != "Earlier scene"));
    let indexes: Vec<_> = owner
        .state
        .video_indexes
        .iter()
        .filter(|index| index.room_id == room_id)
        .collect();
    let fallback = crate::video_index::search(
        &indexes,
        &owner.assets(&room_id).unwrap(),
        "Find the exact on-screen text “HAPPY BIRTHDAY”",
        "frame",
    );
    assert_eq!(
        result, fallback,
        "cached and fallback search must select the same phrase bounds"
    );
}

#[test]
fn video_search_labels_machine_generated_scene_matches_as_suggestions() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("bike.mp4");
    fs::write(&movie, b"video with a locally generated scene label").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":false, "frames_indexed":true, "frame_interval_ms":2000,
            "observations":[{"modality":"visual_label", "start_ms":18000,
                "end_ms":18000, "text":"red bicycle", "confidence_percent":95}]
        }))
        .unwrap();

    for query in ["Find a red bicycle", "Find the exact scene \"red bicycle\""] {
        let result = owner
            .execute(json!({
                "action":"search_video", "room_id":room_id,
                "query":query, "modality":"frame"
            }))
            .unwrap();
        assert_eq!(result["matches"].as_array().unwrap().len(), 1);
        assert_eq!(result["matches"][0]["match_kind"], "suggestion");
        assert!(result["answer"]
            .as_str()
            .unwrap()
            .contains("No exact match"));
        let indexes: Vec<_> = owner
            .state
            .video_indexes
            .iter()
            .filter(|index| index.room_id == room_id)
            .collect();
        assert_eq!(
            result,
            crate::video_index::search(&indexes, &owner.assets(&room_id).unwrap(), query, "frame")
        );
    }

    let reversed = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find the exact scene \"bicycle red\"", "modality":"frame"
        }))
        .unwrap();
    assert!(reversed["matches"].as_array().unwrap().is_empty());
}

#[test]
fn video_search_sound_events_are_audio_suggestions_not_exact_matches() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("applause.mp4");
    fs::write(&movie, b"video with locally classified sound event").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":false, "sound_events_indexed":true, "frames_indexed":false,
            "observations":[{"modality":"sound_event", "start_ms":18000,
                "end_ms":21000, "text":"applause", "confidence_percent":91}]
        }))
        .unwrap();

    let result = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find exact sound \"applause\"", "modality":"audio"
        }))
        .unwrap();
    assert_eq!(result["matches"].as_array().unwrap().len(), 1);
    assert_eq!(result["matches"][0]["match_kind"], "suggestion");
    assert_eq!(result["matches"][0]["start_ms"], 18_000);
    assert_eq!(
        result["matches"][0]["evidence"][0]["modality"],
        "sound_event"
    );
    assert_eq!(result["coverage"]["sound_events_indexed"], 1);
    assert!(result["answer"]
        .as_str()
        .unwrap()
        .contains("No exact match"));

    let frame_only = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"applause", "modality":"frame"
        }))
        .unwrap();
    assert!(frame_only["matches"].as_array().unwrap().is_empty());
    let indexes: Vec<_> = owner
        .state
        .video_indexes
        .iter()
        .filter(|index| index.room_id == room_id)
        .collect();
    assert_eq!(
        result,
        crate::video_index::search(
            &indexes,
            &owner.assets(&room_id).unwrap(),
            "Find exact sound \"applause\"",
            "audio"
        )
    );
}

#[test]
fn video_reindex_preserves_sound_events_from_an_earlier_audio_pass() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("laughter-reindex.mp4");
    fs::write(&movie, b"video with locally classified sound event").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":false, "sound_events_indexed":true, "frames_indexed":false,
            "observations":[{"modality":"sound_event", "start_ms":7000,
                "end_ms":9000, "text":"laughter", "confidence_percent":87}]
        }))
        .unwrap();
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":false, "sound_events_indexed":false, "frames_indexed":true,
            "frame_interval_ms":2000, "preserve_existing":true,
            "observations":[{"modality":"screen_text", "start_ms":1000,
                "end_ms":1000, "text":"birthday", "confidence_percent":90}]
        }))
        .unwrap();

    let result = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"\"laughter\"", "modality":"audio"
        }))
        .unwrap();
    assert_eq!(result["matches"].as_array().unwrap().len(), 1);
    assert_eq!(result["matches"][0]["match_kind"], "suggestion");
    assert_eq!(result["matches"][0]["start_ms"], 7000);
    assert_eq!(result["coverage"]["sound_events_indexed"], 1);
}

#[test]
fn video_reindex_preserves_completed_scene_label_coverage_when_a_later_pass_has_none() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("bike-reindex.mp4");
    fs::write(&movie, b"video with prior local scene labels").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":false, "frames_indexed":true,
            "visual_labels_indexed":true, "frame_interval_ms":2000,
            "observations":[{"modality":"visual_label", "start_ms":18000,
                "end_ms":18000, "text":"red bicycle", "confidence_percent":55}]
        }))
        .unwrap();

    owner
        .execute(json!({
            "action":"index_video", "room_id":room_id, "asset_id":asset_id,
            "audio_indexed":false, "frames_indexed":true,
            "visual_labels_indexed":false, "frame_interval_ms":2000,
            "preserve_existing":true,
            "observations":[{"modality":"screen_text", "start_ms":19000,
                "end_ms":19000, "text":"Family ride", "confidence_percent":95}]
        }))
        .unwrap();

    let status = owner
        .execute(json!({
            "action":"video_index_status", "room_id":room_id
        }))
        .unwrap();
    assert_eq!(status["videos"][0]["visual_labels_indexed"], true);
    let result = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find \"red bicycle\"", "modality":"frame"
        }))
        .unwrap();
    assert_eq!(result["matches"][0]["match_kind"], "suggestion");
    assert!(result["matches"][0]["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["text"] == "red bicycle"));
}

#[test]
fn video_search_ignores_speaker_framing_and_requires_exact_quoted_order() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("family-speech.mp4");
    fs::write(&movie, b"video for attributed exact-search test").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);
    owner
        .execute(
            json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
            "audio_indexed":true,"frames_indexed":true,"frame_interval_ms":2000,
            "observations":[
                {"modality":"speech","start_ms":1000,"end_ms":1800,
                    "text":"We found the yellow","confidence_percent":92},
                {"modality":"speech","start_ms":1900,"end_ms":2400,
                    "text":"balloon","confidence_percent":91},
                {"modality":"speech","start_ms":9000,"end_ms":10500,
                    "text":"Yellow balloon found we","confidence_percent":99},
                {"modality":"screen_text","start_ms":15000,"end_ms":15000,
                    "text":"HAPPY","confidence_percent":96},
                {"modality":"visual_label","start_ms":15000,"end_ms":15000,
                    "text":"cake","confidence_percent":90},
                {"modality":"screen_text","start_ms":15500,"end_ms":15500,
                    "text":"BIRTHDAY TODAY","confidence_percent":95}
            ]}),
        )
        .unwrap();

    let reference = |core: &Engine, query: &str| {
        let indexes: Vec<_> = core
            .state
            .video_indexes
            .iter()
            .filter(|index| index.room_id == room_id)
            .collect();
        crate::video_index::search(&indexes, &core.assets(&room_id).unwrap(), query, "audio")
    };
    let attributed_query = "Where does Grandma say we found the yellow balloon?";
    let attributed_expected = reference(&owner, attributed_query);
    let attributed = owner
        .execute(json!({"action":"search_video","room_id":room_id,
            "query":attributed_query,"modality":"audio"}))
        .unwrap();
    assert_eq!(attributed, attributed_expected);
    assert!(!attributed["matches"].as_array().unwrap().is_empty());
    assert_eq!(attributed["matches"][0]["start_ms"], 1000);
    assert_eq!(
        attributed["matches"][0]["evidence"][0]["text"],
        "We found the yellow"
    );

    let exact_query = "Find Grandma saying ‘we found the yellow balloon’";
    let exact_expected = reference(&owner, exact_query);
    let exact = owner
        .execute(json!({"action":"search_video","room_id":room_id,
            "query":exact_query,"modality":"audio"}))
        .unwrap();
    assert_eq!(exact, exact_expected);
    assert_eq!(exact["matches"].as_array().unwrap().len(), 1);
    assert_eq!(exact["matches"][0]["start_ms"], 1000);

    let wrong_order_query = "\"balloon we yellow found\"";
    let wrong_order_expected = reference(&owner, wrong_order_query);
    let wrong_order = owner
        .execute(json!({"action":"search_video","room_id":room_id,
            "query":wrong_order_query,"modality":"audio"}))
        .unwrap();
    assert_eq!(wrong_order, wrong_order_expected);
    assert_eq!(wrong_order["matches"].as_array().unwrap().len(), 0);

    let frame_exact = owner
        .execute(json!({"action":"search_video","room_id":room_id,
            "query":"\"happy birthday today\"","modality":"frame"}))
        .unwrap();
    assert_eq!(frame_exact["matches"].as_array().unwrap().len(), 1);
    assert_eq!(frame_exact["matches"][0]["start_ms"], 15000);
}

#[test]
fn video_index_rejects_invalid_or_unavailable_evidence() {
    let (dir, mut engine) = vault();
    let room_id = room(&mut engine);
    let photo = import(&mut engine, &room_id, &media(&dir));
    assert!(engine
        .execute(json!({"action":"index_video","room_id":room_id,
        "asset_id":photo,"audio_indexed":true,"observations":[]}))
        .is_err());
    let movie = dir.path().join("movie.mp4");
    fs::write(&movie, b"video bytes").unwrap();
    let asset = import(&mut engine, &room_id, &movie);
    for invalid in [
        json!({"modality":"speaker_identity","start_ms":0,"end_ms":0,"text":"Maya","confidence_percent":90}),
        json!({"modality":"speech","start_ms":5,"end_ms":4,"text":"hello","confidence_percent":90}),
        json!({"modality":"speech","start_ms":0,"end_ms":2,"text":"hello","confidence_percent":101}),
    ] {
        assert!(engine
            .execute(json!({"action":"index_video","room_id":room_id,
            "asset_id":asset,"audio_indexed":true,"observations":[invalid]}))
            .is_err());
    }
    assert!(engine
        .execute(json!({"action":"search_video","room_id":room_id,
        "query":""}))
        .is_err());
}

#[test]
fn video_search_requires_all_terms_and_excludes_deleted_originals() {
    let (dir, mut engine) = vault();
    let room_id = room(&mut engine);
    let movie = dir.path().join("bike.mp4");
    fs::write(&movie, b"indexed movie original").unwrap();
    let asset_id = import(&mut engine, &room_id, &movie);
    engine
        .execute(
            json!({"action":"index_video","room_id":room_id,"asset_id":asset_id,
            "frames_indexed":true,"frame_interval_ms":2000,"observations":[
                {"modality":"visual_label","start_ms":1000,"end_ms":3000,
                 "text":"blue bicycle","confidence_percent":92},
                {"modality":"screen_text","start_ms":45000,"end_ms":47000,
                 "text":"birthday cake","confidence_percent":97}
            ]}),
        )
        .unwrap();
    let query = |engine: &mut Engine, phrase: &str| {
        engine
            .execute(json!({
                "action":"search_video","room_id":room_id,"query":phrase,"modality":"frame"
            }))
            .unwrap()
    };
    assert_eq!(
        query(&mut engine, "find blue bicycle")["matches"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(query(&mut engine, "find blue cake")["matches"]
        .as_array()
        .unwrap()
        .is_empty());
    engine
        .execute(json!({"action":"patch_asset","room_id":room_id,
        "asset_id":asset_id,"fields":{"deleted":true}}))
        .unwrap();
    assert!(query(&mut engine, "blue bicycle")["matches"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(engine
        .execute(json!({"action":"video_index_status","room_id":room_id}))
        .unwrap()["videos"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn album_without_new_fields_decodes_with_safe_defaults() {
    let album: crate::model::Album = serde_json::from_value(json!({
        "id": "album-1", "room_id": "room-1", "title": "Older album",
        "asset_ids": ["photo-1"], "pinned": false
    }))
    .unwrap();
    assert_eq!(album.caption, "");
    assert_eq!(album.cover_asset_id, None);
}

#[test]
fn album_caption_and_cover_survive_reorder_and_publish_with_scope_mapping() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let first = import(&mut owner, &room_id, &media(&dir));
    let second_path = dir.path().join("second.jpg");
    fs::write(&second_path, b"second memory").unwrap();
    let second = import(&mut owner, &room_id, &second_path);
    let album_id = owner
        .execute(json!({"action":"save_album","room_id":room_id,
        "title":"Spring days","caption":"  We were together.  ",
        "asset_ids":[first,second],"cover_asset_id":second,"pinned":true}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let album = &owner.snapshot().unwrap()["rooms"][0]["albums"][0];
    assert_eq!(album["caption"], "We were together.");
    assert_eq!(album["cover_asset_id"], second);

    owner
        .execute(
            json!({"action":"save_album","room_id":room_id,"id":album_id,
        "title":"Spring days","asset_ids":[second,first]}),
        )
        .unwrap();
    let album = &owner.snapshot().unwrap()["rooms"][0]["albums"][0];
    assert_eq!(album["caption"], "We were together.");
    assert_eq!(album["cover_asset_id"], second);
    assert_eq!(album["pinned"], true);
    let oversized_note = "🍁".repeat(501);
    assert!(owner
        .execute(
            json!({"action":"save_album","room_id":room_id,"id":album_id,
            "title":"Spring days","asset_ids":[second,first],"caption":oversized_note})
        )
        .is_err());
    assert!(owner
        .execute(
            json!({"action":"save_album","room_id":room_id,"id":album_id,
        "title":"Spring days","asset_ids":[first],"cover_asset_id":second})
        )
        .is_err());

    let scope = owner
        .execute(json!({"action":"publish_album","room_id":room_id,
        "album_id":album_id}))
        .unwrap()["scope_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let snapshot = owner.snapshot().unwrap();
    let published = snapshot["rooms"]
        .as_array()
        .unwrap()
        .iter()
        .find(|room| room["id"] == scope)
        .unwrap();
    let shared_album = &published["albums"][0];
    assert_eq!(shared_album["caption"], "We were together.");
    assert_ne!(shared_album["cover_asset_id"], second);
    assert!(shared_album["asset_ids"]
        .as_array()
        .unwrap()
        .contains(&shared_album["cover_asset_id"]));

    owner
        .execute(
            json!({"action":"save_album","room_id":room_id,"id":album_id,
        "title":"Spring days","asset_ids":[first]}),
        )
        .unwrap();
    assert!(owner.snapshot().unwrap()["rooms"][0]["albums"][0]["cover_asset_id"].is_null());
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["albums"][0]["caption"],
        "We were together."
    );
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
    let uploaded = owner
        .execute(json!({"action":"replicate","source_id":owner_source,"asset_id":new}))
        .unwrap();
    assert_eq!(uploaded["state"], "complete", "{uploaded}");
    owner
        .execute(json!({"action":"sync_source","id":owner_source}))
        .unwrap();
    // The server rejects the old credential even before the removed client syncs.
    assert!(crate::providers::list(&stale_configuration).is_err());
    let synced = removed
        .execute(json!({"action":"sync_all","room_id":r}))
        .unwrap();
    assert!(synced["errors"].as_array().unwrap().is_empty(), "{synced}");
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

#[cfg(feature = "gateway")]
#[test]
fn gateway_accepts_owner_handoff_and_refreshes_existing_and_new_members() {
    let directory = tempfile::tempdir().unwrap();
    let app = crate::gateway::router(directory.path().to_owned(), None).unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let (shutdown, stop) = tokio::sync::oneshot::channel::<()>();
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
    let endpoint = format!("http://{}", receiver.recv().unwrap());
    let (owner_dir, mut owner) = vault();
    let (_successor_dir, mut successor) = vault();
    let (_member_dir, mut member) = vault();
    let (_late_dir, mut late_member) = vault();
    let room_id = room(&mut owner);
    invite(&mut owner, &mut successor, &room_id, Role::Admin);
    invite(&mut owner, &mut member, &room_id, Role::Contributor);
    let asset_id = import(&mut owner, &room_id, &media(&owner_dir));

    let add_gateway = |core: &mut Engine| {
        core.execute(json!({"action":"add_source","room_id":room_id,
            "name":"Shared gateway","kind":"gateway","endpoint":endpoint}))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let _owner_source = add_gateway(&mut owner);
    owner
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();
    let successor_source = add_gateway(&mut successor);
    successor
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();
    let _member_source = add_gateway(&mut member);
    member
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();

    let offer = owner
        .execute(json!({"action":"prepare_owner_transfer","room_id":room_id,
            "device_id":successor.device().unwrap()}))
        .unwrap()["offer"]
        .clone();
    successor
        .execute(json!({"action":"accept_owner_transfer","room_id":room_id,
            "offer":offer}))
        .unwrap();

    let source = successor
        .state
        .sources
        .iter()
        .find(|source| source.id == successor_source)
        .unwrap()
        .clone();
    let incoming = crate::providers::read_access(&source).expect("read the prior gateway policy");
    let mut candidate = successor.room(&room_id).unwrap().clone();
    crate::access::apply(&mut candidate, incoming, &successor.state.signing_seed)
        .expect("apply the prior gateway policy");

    let refreshed = successor
        .execute(json!({"action":"refresh_access","id":successor_source}))
        .unwrap();
    assert_eq!(refreshed["active"], true);

    let publish = successor
        .execute(json!({"action":"sync_source","id":successor_source}))
        .unwrap();
    assert!(publish.get("merged").is_some(), "{publish}");
    for core in [&mut owner, &mut member] {
        let refresh = core
            .execute(json!({"action":"sync_all","room_id":room_id}))
            .unwrap();
        assert!(
            refresh["errors"].as_array().unwrap().is_empty(),
            "{refresh}"
        );
    }
    assert_eq!(
        owner.room(&room_id).unwrap().owner_id,
        successor.device().unwrap()
    );
    assert_eq!(owner.room(&room_id).unwrap().grant.role, Role::Admin);
    assert_eq!(member.room(&room_id).unwrap().grant.role, Role::Contributor);
    assert_eq!(owner.assets(&room_id).unwrap()[0].id, asset_id);

    // A successor can bootstrap a fresh gateway with the complete signed
    // ownership chain, so a new storage endpoint retains the original trust
    // anchor instead of treating the successor as an unrelated Room owner.
    let fresh_gateway_directory = tempfile::tempdir().unwrap();
    let app = crate::gateway::router(fresh_gateway_directory.path().to_owned(), None).unwrap();
    let (fresh_sender, fresh_receiver) = std::sync::mpsc::channel();
    let (fresh_shutdown, fresh_stop) = tokio::sync::oneshot::channel::<()>();
    let fresh_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            fresh_sender.send(listener.local_addr().unwrap()).unwrap();
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = fresh_stop.await;
                })
                .await
                .unwrap();
        });
    });
    let fresh_endpoint = format!("http://{}", fresh_receiver.recv().unwrap());
    let fresh_source = successor
        .execute(json!({"action":"add_source","room_id":room_id,
            "name":"Successor gateway","kind":"gateway","endpoint":fresh_endpoint}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    successor
        .execute(json!({"action":"sync_source","id":fresh_source}))
        .unwrap();
    let _ = fresh_shutdown.send(());
    fresh_thread.join().unwrap();

    let invitation = successor
        .execute(json!({"action":"invite","room_id":room_id,
            "device_id":late_member.device().unwrap(),"role":"viewer"}))
        .unwrap()["invitation"]
        .clone();
    successor
        .execute(json!({"action":"sync_source","id":successor_source}))
        .unwrap();
    late_member
        .execute(json!({"action":"join","invitation":invitation}))
        .unwrap();
    let _late_source = add_gateway(&mut late_member);
    let late_sync = late_member
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();
    assert!(
        late_sync["errors"].as_array().unwrap().is_empty(),
        "{late_sync}"
    );
    assert_eq!(
        late_member.room(&room_id).unwrap().owner_id,
        successor.device().unwrap()
    );
    assert_eq!(late_member.room(&room_id).unwrap().grant.role, Role::Viewer);

    owner
        .execute(json!({"action":"revoke_device","room_id":room_id,
            "device_id":member.device().unwrap()}))
        .unwrap_err();
    let shutdown_result = shutdown.send(());
    assert!(shutdown_result.is_ok());
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

#[test]
fn bulk_capture_date_corrections_are_atomic_and_preserve_originals() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let first_path = media(&dir);
    let first = import(&mut core, &room_id, &first_path);
    let second_path = dir.path().join("second.jpg");
    fs::write(&second_path, b"second-private-original").unwrap();
    let second = import(&mut core, &room_id, &second_path);
    for (asset_id, captured_at) in [(&first, 1_000_i64), (&second, 1_060_i64)] {
        core.execute(json!({"action":"patch_asset","room_id":room_id,
            "asset_id":asset_id,"fields":{"captured_at":captured_at}}))
            .unwrap();
    }

    let before = core.snapshot().unwrap();
    let result = core
        .execute(json!({
            "action":"patch_asset_dates", "room_id":room_id,
            "dates":[
                {"asset_id":first,"captured_at":86_401},
                {"asset_id":second,"captured_at":86_461}
            ]
        }))
        .unwrap();
    assert_eq!(result["updated"], 2);

    let assets = core.snapshot().unwrap()["rooms"][0]["assets"]
        .as_array()
        .unwrap()
        .clone();
    let first_asset = assets.iter().find(|asset| asset["id"] == first).unwrap();
    let second_asset = assets.iter().find(|asset| asset["id"] == second).unwrap();
    let before_first = before["rooms"][0]["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|asset| asset["id"] == first)
        .unwrap();
    assert_eq!(first_asset["captured_at"], 86_401);
    assert_eq!(second_asset["captured_at"], 86_461);
    assert_eq!(first_asset["sha256"], before_first["sha256"]);
    assert_eq!(fs::read(&first_path).unwrap(), b"private-photo-original");
    assert_eq!(fs::read(&second_path).unwrap(), b"second-private-original");

    let before_failed_batch = core.snapshot().unwrap();
    assert!(core
        .execute(json!({
            "action":"patch_asset_dates", "room_id":room_id,
            "dates":[
                {"asset_id":first,"captured_at":99_999},
                {"asset_id":"missing-asset","captured_at":100_000}
            ]
        }))
        .is_err());
    assert_eq!(core.snapshot().unwrap(), before_failed_batch);
}

#[test]
fn batch_media_edits_update_selected_items_and_validate_before_writing() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let first = import(&mut core, &room_id, &media(&dir));
    let second_path = dir.path().join("second.jpg");
    fs::write(&second_path, b"another-private-photo").unwrap();
    let second = import(&mut core, &room_id, &second_path);

    let result = core
        .execute(json!({
            "action": "patch_assets",
            "room_id": room_id,
            "asset_ids": [first, second],
            "fields": {"caption": "Autumn walk", "favorite": true}
        }))
        .unwrap();

    assert_eq!(result["updated"], 2);
    for asset in core.snapshot().unwrap()["rooms"][0]["assets"]
        .as_array()
        .unwrap()
    {
        assert_eq!(asset["caption"], "Autumn walk");
        assert_eq!(asset["favorite"], true);
    }

    let before_failed_batch = core.snapshot().unwrap();
    assert!(core
        .execute(json!({
            "action": "patch_assets",
            "room_id": room_id,
            "asset_ids": [first, "missing-asset"],
            "fields": {"caption": "Must not partially apply"}
        }))
        .is_err());
    assert_eq!(core.snapshot().unwrap(), before_failed_batch);
}

#[test]
fn bulk_person_tags_apply_and_remove_only_for_a_person_in_the_same_room() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let first = import(&mut core, &room_id, &media(&dir));
    let second_path = dir.path().join("second-person-tag.jpg");
    fs::write(&second_path, b"another-private-photo").unwrap();
    let second = import(&mut core, &room_id, &second_path);
    let person_id = core
        .execute(json!({"action":"save_person","room_id":room_id,"name":"Grandma"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    assert_eq!(
        core.execute(json!({
            "action": "tag_assets",
            "room_id": room_id,
            "asset_ids": [first, second],
            "person_id": person_id
        }))
        .unwrap()["updated"],
        2
    );
    let other_room = room(&mut core);
    let foreign_person = core
        .execute(json!({
            "action": "save_person",
            "room_id": other_room,
            "name": "Person from another Room"
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let tagged = core.snapshot().unwrap();
    for asset in tagged["rooms"][0]["assets"].as_array().unwrap() {
        assert_eq!(asset["people"], json!([person_id]));
    }
    assert!(core
        .execute(json!({
            "action": "tag_assets",
            "room_id": room_id,
            "asset_ids": [first],
            "person_id": foreign_person
        }))
        .is_err());
    assert_eq!(core.snapshot().unwrap(), tagged);

    assert!(core
        .execute(json!({
            "action": "tag_assets",
            "room_id": room_id,
            "asset_ids": [first, "missing-asset"],
            "person_id": person_id,
            "remove": true
        }))
        .is_err());
    assert_eq!(core.snapshot().unwrap(), tagged);

    core.execute(json!({
        "action": "tag_assets",
        "room_id": room_id,
        "asset_ids": [first, second],
        "person_id": person_id,
        "remove": true
    }))
    .unwrap();
    for asset in core.snapshot().unwrap()["rooms"][0]["assets"]
        .as_array()
        .unwrap()
    {
        assert_eq!(asset["people"], json!([]));
    }
}

#[test]
fn merging_duplicate_people_keeps_tags_private_and_converges_with_offline_edits() {
    let (owner_dir, mut owner) = vault();
    let (_member_dir, mut member) = vault();
    let room_id = room(&mut owner);
    let asset_id = import(&mut owner, &room_id, &media(&owner_dir));
    let canonical_id = owner
        .execute(json!({
            "action": "save_person",
            "room_id": room_id,
            "name": "Avery"
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let duplicate_id = owner
        .execute(json!({
            "action": "save_person",
            "room_id": room_id,
            "name": "Avery Smith"
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    owner
        .execute(json!({
            "action": "tag_assets",
            "room_id": room_id,
            "person_id": duplicate_id,
            "asset_ids": [asset_id]
        }))
        .unwrap();
    invite(&mut owner, &mut member, &room_id, Role::Contributor);
    member
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();

    owner
        .execute(json!({
            "action": "merge_person",
            "room_id": room_id,
            "source_person_id": duplicate_id,
            "target_person_id": canonical_id
        }))
        .unwrap();

    // This member has not seen the merge and still writes the duplicate ID.
    member
        .execute(json!({
            "action": "tag_assets",
            "room_id": room_id,
            "person_id": duplicate_id,
            "asset_ids": [asset_id]
        }))
        .unwrap();
    owner
        .merge_packet(&room_id, &member.packet(&room_id).unwrap())
        .unwrap();

    let snapshot = owner.snapshot().unwrap();
    let people = snapshot["rooms"][0]["people"].as_array().unwrap();
    assert_eq!(people.len(), 1);
    assert_eq!(people[0]["id"], canonical_id);
    assert_eq!(people[0]["name"], "Avery");
    assert_eq!(
        snapshot["rooms"][0]["assets"][0]["people"],
        json!([canonical_id])
    );

    drop(owner);
    let reopened = Engine::open(owner_dir.path(), [7; 32]).unwrap();
    let restored = reopened.snapshot().unwrap();
    assert_eq!(restored["rooms"][0]["people"].as_array().unwrap().len(), 1);
    assert_eq!(
        restored["rooms"][0]["assets"][0]["people"],
        json!([canonical_id])
    );
}

#[test]
fn concurrent_person_merges_choose_the_same_visible_profile_on_both_devices() {
    let (_owner_dir, mut owner) = vault();
    let (_member_dir, mut member) = vault();
    let room_id = room(&mut owner);
    let first_id = owner
        .execute(json!({
            "action": "save_person",
            "room_id": room_id,
            "name": "First profile"
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let second_id = owner
        .execute(json!({
            "action": "save_person",
            "room_id": room_id,
            "name": "Second profile"
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    invite(&mut owner, &mut member, &room_id, Role::Contributor);
    member
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();

    owner
        .execute(json!({
            "action": "merge_person",
            "room_id": room_id,
            "source_person_id": second_id,
            "target_person_id": first_id
        }))
        .unwrap();
    member
        .execute(json!({
            "action": "merge_person",
            "room_id": room_id,
            "source_person_id": first_id,
            "target_person_id": second_id
        }))
        .unwrap();

    owner
        .merge_packet(&room_id, &member.packet(&room_id).unwrap())
        .unwrap();
    member
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();

    let expected_id = first_id.min(second_id);
    let owner_people = owner.snapshot().unwrap()["rooms"][0]["people"].clone();
    let member_people = member.snapshot().unwrap()["rooms"][0]["people"].clone();
    assert_eq!(owner_people.as_array().unwrap().len(), 1);
    assert_eq!(member_people.as_array().unwrap().len(), 1);
    assert_eq!(owner_people[0]["id"], expected_id);
    assert_eq!(member_people, owner_people);
}

#[test]
fn batch_delete_respects_each_contributor_and_never_partially_deletes() {
    let (owner_dir, mut owner) = vault();
    let (member_dir, mut member) = vault();
    let room_id = room(&mut owner);
    let owner_asset = import(&mut owner, &room_id, &media(&owner_dir));
    invite(&mut owner, &mut member, &room_id, Role::Contributor);
    member
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();

    let member_photo = member_dir.path().join("member.jpg");
    fs::write(&member_photo, b"member-original").unwrap();
    let member_asset = import(&mut member, &room_id, &member_photo);

    assert!(member
        .execute(json!({
            "action": "patch_assets",
            "room_id": room_id,
            "asset_ids": [member_asset, owner_asset],
            "fields": {"deleted": true}
        }))
        .is_err());

    let snapshot = member.snapshot().unwrap();
    let assets = snapshot["rooms"][0]["assets"].as_array().unwrap();
    assert!(assets.iter().all(|asset| asset["deleted"] == false));
}

#[test]
fn bulk_trash_and_restore_preserve_original_bytes() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let original = dir.path().join("recoverable.jpg");
    fs::write(&original, b"the-original-photo-bytes").unwrap();
    let first = import(&mut core, &room_id, &original);
    let second_path = dir.path().join("recoverable-video.mp4");
    fs::write(&second_path, b"the-original-video-bytes").unwrap();
    let second = import(&mut core, &room_id, &second_path);

    for deleted in [true, false] {
        let result = core
            .execute(json!({
                "action":"patch_assets",
                "room_id":room_id,
                "asset_ids":[first, second],
                "fields":{"deleted":deleted}
            }))
            .unwrap();
        assert_eq!(result["updated"], 2);

        let trash_path = dir.path().join("verified-original-while-trashed.jpg");
        core.execute(json!({
            "action":"materialize",
            "room_id":room_id,
            "asset_id":first,
            "destination":trash_path
        }))
        .unwrap();
        assert_eq!(fs::read(trash_path).unwrap(), b"the-original-photo-bytes");

        let snapshot = core.snapshot().unwrap();
        assert!(snapshot["rooms"][0]["assets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|asset| asset["deleted"] == deleted));
    }
}

#[test]
fn viewers_cannot_edit_media_through_single_or_batch_commands() {
    let (owner_dir, mut owner) = vault();
    let (_viewer_dir, mut viewer) = vault();
    let room_id = room(&mut owner);
    let asset_id = import(&mut owner, &room_id, &media(&owner_dir));
    let person_id = owner
        .execute(json!({"action":"save_person","room_id":room_id,"name":"Grandma"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    invite(&mut owner, &mut viewer, &room_id, Role::Viewer);
    viewer
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();

    assert!(viewer
        .execute(json!({
            "action": "patch_asset",
            "room_id": room_id,
            "asset_id": asset_id,
            "fields": {"caption": "Viewer edit"}
        }))
        .is_err());
    assert!(viewer
        .execute(json!({
            "action": "patch_assets",
            "room_id": room_id,
            "asset_ids": [asset_id],
            "fields": {"favorite": true}
        }))
        .is_err());
    assert!(viewer
        .execute(json!({
            "action": "patch_asset_dates",
            "room_id": room_id,
            "dates": [{"asset_id": asset_id, "captured_at": 1234}]
        }))
        .is_err());
    assert!(viewer
        .execute(json!({
            "action": "tag_assets",
            "room_id": room_id,
            "asset_ids": [asset_id],
            "person_id": person_id
        }))
        .is_err());

    let snapshot = viewer.snapshot().unwrap();
    let asset = &snapshot["rooms"][0]["assets"][0];
    assert_eq!(asset["caption"], "");
    assert_eq!(asset["favorite"], false);
}

#[test]
fn a_member_can_leave_locally_and_an_owner_must_transfer_first() {
    let (_owner_dir, mut owner) = vault();
    let (member_dir, mut member) = vault();
    let room_id = room(&mut owner);
    invite(&mut owner, &mut member, &room_id, Role::Contributor);

    let source_dir = tempfile::tempdir().unwrap();
    let source_id = member
        .execute(json!({"action":"add_source","room_id":room_id,
            "name":"Private archive","kind":"local","endpoint":source_dir.path()}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let watch_id = member
        .execute(json!({"action":"watch_folder","room_id":room_id,
            "path":member_dir.path(),"include_existing":true}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let owner_error = owner
        .execute(json!({"action":"leave_room","room_id":room_id}))
        .unwrap_err();
    assert!(owner_error.to_string().contains("Transfer ownership"));
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["owner_id"],
        owner.device().unwrap()
    );

    let result = member
        .execute(json!({"action":"leave_room","room_id":room_id}))
        .unwrap();
    assert_eq!(result["local_access_removed"], true);
    assert_eq!(result["shared_roster_changed"], false);
    let snapshot = member.snapshot().unwrap();
    assert!(snapshot["rooms"].as_array().unwrap().is_empty());
    assert!(snapshot["sources"].as_array().unwrap().is_empty());
    assert!(snapshot["watches"].as_array().unwrap().is_empty());
    assert!(snapshot["sources"]
        .as_array()
        .unwrap()
        .iter()
        .all(|source| source["id"] != source_id));
    assert!(snapshot["watches"]
        .as_array()
        .unwrap()
        .iter()
        .all(|watch| watch["id"] != watch_id));
    assert!(member.room(&room_id).is_err());
    let owner_snapshot = owner.snapshot().unwrap();
    assert_eq!(owner_snapshot["rooms"].as_array().unwrap().len(), 1);
    assert!(owner_snapshot["rooms"][0]["members"]
        .as_array()
        .unwrap()
        .iter()
        .any(|grant| grant["device_id"] == member.device().unwrap()));
    drop(member);
    let reopened_member = Engine::open(member_dir.path(), [7; 32]).unwrap();
    assert!(reopened_member.snapshot().unwrap()["rooms"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn a_former_owner_can_leave_after_the_successor_accepts_ownership() {
    let (_owner_dir, mut owner) = vault();
    let (successor_dir, mut successor) = vault();
    let room_id = room(&mut owner);
    invite(&mut owner, &mut successor, &room_id, Role::Admin);
    let offer = owner
        .execute(json!({"action":"prepare_owner_transfer","room_id":room_id,
            "device_id":successor.device().unwrap()}))
        .unwrap()["offer"]
        .clone();
    successor
        .execute(json!({"action":"accept_owner_transfer","room_id":room_id,
            "offer":offer}))
        .unwrap();
    owner
        .merge_packet(&room_id, &successor.packet(&room_id).unwrap())
        .unwrap();

    let former_owner = owner.device().unwrap();
    let leave = owner
        .execute(json!({"action":"leave_room","room_id":room_id}))
        .unwrap();
    assert_eq!(leave["local_access_removed"], true);
    assert!(owner.snapshot().unwrap()["rooms"]
        .as_array()
        .unwrap()
        .is_empty());
    let current_owner = successor.snapshot().unwrap();
    assert_eq!(
        current_owner["rooms"][0]["owner_id"],
        successor.device().unwrap()
    );
    assert!(current_owner["rooms"][0]["members"]
        .as_array()
        .unwrap()
        .iter()
        .any(|grant| grant["device_id"] == former_owner && grant["role"] == "admin"));
    let successor_id = successor.device().unwrap();
    drop(owner);
    drop(successor);
    let reopened = Engine::open(successor_dir.path(), [7; 32]).unwrap();
    assert_eq!(
        reopened.snapshot().unwrap()["rooms"][0]["owner_id"],
        successor_id
    );
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
fn accepted_owner_transfer_reissues_grants_rotates_keys_and_demotes_previous_owner() {
    let (owner_dir, mut owner) = vault();
    let (_successor_dir, mut successor) = vault();
    let (_member_dir, mut member) = vault();
    let room_id = room(&mut owner);
    invite(&mut owner, &mut successor, &room_id, Role::Admin);
    invite(&mut owner, &mut member, &room_id, Role::Contributor);
    let media_path = media(&owner_dir);
    let asset_id = import(&mut owner, &room_id, &media_path);
    successor
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();
    member
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();

    let offer = owner
        .execute(json!({"action":"prepare_owner_transfer","room_id":room_id,
            "device_id":successor.device().unwrap()}))
        .unwrap()["offer"]
        .clone();
    let accepted = successor
        .execute(json!({"action":"accept_owner_transfer","room_id":room_id,
            "offer":offer}))
        .unwrap();
    let changes = accepted["changes"].clone();

    owner
        .execute(json!({"action":"apply_access","room_id":room_id,"changes":changes}))
        .unwrap();
    member
        .execute(json!({"action":"apply_access","room_id":room_id,"changes":changes}))
        .unwrap();

    assert_eq!(
        owner.room(&room_id).unwrap().owner_id,
        successor.device().unwrap()
    );
    assert_eq!(owner.room(&room_id).unwrap().grant.role, Role::Admin);
    assert_eq!(successor.room(&room_id).unwrap().grant.role, Role::Owner);
    assert_eq!(member.room(&room_id).unwrap().grant.role, Role::Contributor);
    assert_eq!(crate::access::epoch(owner.room(&room_id).unwrap()), 1);
    assert_eq!(crate::access::epoch(successor.room(&room_id).unwrap()), 1);
    assert_eq!(crate::access::epoch(member.room(&room_id).unwrap()), 1);
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["assets"][0]["id"],
        asset_id
    );
    assert!(owner
        .execute(json!({"action":"set_role","room_id":room_id,
            "device_id":member.device().unwrap(),"role":"viewer"}))
        .is_err());
    successor
        .execute(json!({"action":"set_role","room_id":room_id,
            "device_id":member.device().unwrap(),"role":"viewer"}))
        .unwrap();

    drop(owner);
    let reopened = Engine::open(owner_dir.path(), [7; 32]).unwrap();
    assert_eq!(
        reopened.room(&room_id).unwrap().owner_id,
        successor.device().unwrap()
    );
    assert_eq!(reopened.room(&room_id).unwrap().grant.role, Role::Admin);
}

#[test]
fn owner_handoff_offers_are_device_bound_and_conflicting_successors_cannot_merge() {
    let (_owner_dir, mut owner) = vault();
    let (_first_dir, mut first_successor) = vault();
    let (_second_dir, mut second_successor) = vault();
    let room_id = room(&mut owner);
    invite(&mut owner, &mut first_successor, &room_id, Role::Admin);
    invite(
        &mut owner,
        &mut second_successor,
        &room_id,
        Role::Contributor,
    );
    for member in [&mut first_successor, &mut second_successor] {
        member
            .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
            .unwrap();
    }

    let first_offer = owner
        .execute(json!({"action":"prepare_owner_transfer","room_id":room_id,
            "device_id":first_successor.device().unwrap()}))
        .unwrap()["offer"]
        .clone();
    let competing_offer = owner
        .execute(json!({"action":"prepare_owner_transfer","room_id":room_id,
            "device_id":second_successor.device().unwrap()}))
        .unwrap()["offer"]
        .clone();

    assert!(second_successor
        .execute(json!({"action":"accept_owner_transfer","room_id":room_id,
            "offer":first_offer}))
        .is_err());
    first_successor
        .execute(json!({"action":"accept_owner_transfer","room_id":room_id,
            "offer":first_offer}))
        .unwrap();
    second_successor
        .execute(json!({"action":"accept_owner_transfer","room_id":room_id,
            "offer":competing_offer}))
        .unwrap();

    let competing_packet = second_successor.packet(&room_id).unwrap();
    assert!(first_successor
        .merge_packet(&room_id, &competing_packet)
        .is_err());
    assert_eq!(
        first_successor.room(&room_id).unwrap().owner_id,
        first_successor.device().unwrap()
    );
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
    let asset_id = owner
        .execute(json!({"action":"import","room_id":r,"path":media(&dir)}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let person_id = owner
        .execute(json!({"action":"save_person","room_id":r,"name":"Grandma"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    owner
        .execute(
            json!({"action":"patch_asset","room_id":r,"asset_id":asset_id,
            "fields":{"people":[person_id]}}),
        )
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["assets"][0]["people"],
        json!([person_id])
    );
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
fn invitations_can_be_reviewed_declined_or_accepted_after_reopening() {
    let (_owner_dir, mut owner) = vault();
    let (member_dir, mut member) = vault();
    let room_id = room(&mut owner);
    let invitation = owner
        .execute(json!({
            "action":"invite", "room_id":room_id,
            "device_id":member.device().unwrap(), "role":"viewer"
        }))
        .unwrap()["invitation"]
        .as_str()
        .unwrap()
        .to_owned();

    let preview = member
        .execute(json!({"action":"preview_invitation", "invitation":invitation}))
        .unwrap();
    assert_eq!(preview["room_id"], room_id);
    assert_eq!(preview["name"], "Family");
    assert_eq!(preview["role"], "viewer");
    assert!(member.snapshot().unwrap()["rooms"]
        .as_array()
        .unwrap()
        .is_empty());

    let staged = member
        .execute(json!({"action":"stage_invitation", "invitation":invitation}))
        .unwrap();
    let pending_id = staged["id"].as_str().unwrap().to_owned();
    let staged_again = member
        .execute(json!({"action":"stage_invitation", "invitation":invitation}))
        .unwrap();
    assert_eq!(
        staged_again["id"], pending_id,
        "staging the same invite is idempotent"
    );
    let snapshot = member.snapshot().unwrap();
    let pending = &snapshot["pending_invitations"][0];
    assert_eq!(pending["id"], pending_id);
    assert_eq!(pending["room_id"], room_id);
    assert_eq!(pending["name"], "Family");
    assert_eq!(pending["role"], "viewer");
    assert!(
        pending.get("invitation").is_none(),
        "never expose the device-bound token in snapshots"
    );
    let encrypted_catalog = fs::read(member_dir.path().join("catalog.fr")).unwrap();
    assert!(!encrypted_catalog
        .windows(invitation.len())
        .any(|window| window == invitation.as_bytes()));

    drop(member);
    let mut member = Engine::open(member_dir.path(), [7; 32]).unwrap();
    let declined = member
        .execute(json!({"action":"decline_invitation", "id":pending_id}))
        .unwrap();
    assert_eq!(declined["notified_owner"], false);
    assert!(member.snapshot().unwrap()["pending_invitations"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(member.snapshot().unwrap()["rooms"]
        .as_array()
        .unwrap()
        .is_empty());

    let staged = member
        .execute(json!({"action":"stage_invitation", "invitation":invitation}))
        .unwrap();
    let pending_id = staged["id"].as_str().unwrap().to_owned();
    drop(member);
    let mut member = Engine::open(member_dir.path(), [7; 32]).unwrap();
    let accepted = member
        .execute(json!({"action":"accept_invitation", "id":pending_id}))
        .unwrap();
    assert_eq!(accepted["id"], room_id);
    let snapshot = member.snapshot().unwrap();
    assert_eq!(snapshot["rooms"][0]["id"], room_id);
    assert!(snapshot["pending_invitations"]
        .as_array()
        .unwrap()
        .is_empty());
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
fn restore_refuses_to_discard_pending_invitations() {
    let (backup_dir, mut owner) = vault();
    let (_restore_dir, mut restored) = vault();
    let room_id = room(&mut owner);
    let backup = backup_dir.path().join("empty-room.frbackup");
    owner
        .execute(json!({"action":"backup", "path":backup}))
        .unwrap();
    let invitation = owner
        .execute(json!({
            "action":"invite",
            "room_id":room_id,
            "device_id":restored.device().unwrap(),
            "role":"viewer"
        }))
        .unwrap()["invitation"]
        .as_str()
        .unwrap()
        .to_owned();
    restored
        .execute(json!({"action":"stage_invitation", "invitation":invitation}))
        .unwrap();
    let pending_id = restored.snapshot().unwrap()["pending_invitations"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    assert!(restored
        .execute(json!({
            "action":"restore",
            "path":backup,
            "recovery_key":hex::encode([7;32])
        }))
        .is_err());
    let snapshot = restored.snapshot().unwrap();
    assert!(snapshot["rooms"].as_array().unwrap().is_empty());
    assert_eq!(snapshot["pending_invitations"][0]["id"], pending_id);
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
fn explicit_overflow_uses_only_selected_available_room_source() {
    let (dir, mut engine) = vault();
    let primary_dir = tempfile::tempdir().unwrap();
    let overflow_dir = tempfile::tempdir().unwrap();
    let other_dir = tempfile::tempdir().unwrap();
    let room_id = room(&mut engine);
    let other_room = room(&mut engine);
    let asset = import(&mut engine, &room_id, &media(&dir));
    let primary = engine
        .execute(json!({"action":"add_source","room_id":room_id,
        "name":"Full","kind":"local","endpoint":primary_dir.path(),"budget":1}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let overflow = engine
        .execute(json!({"action":"add_source","room_id":room_id,
        "name":"Spare","kind":"local","endpoint":overflow_dir.path()}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let unrelated = engine
        .execute(json!({"action":"add_source","room_id":other_room,
        "name":"Other Room","kind":"local","endpoint":other_dir.path()}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    assert!(engine
        .execute(json!({"action":"set_overflow","room_id":room_id,"id":primary}))
        .is_err());
    assert!(engine
        .execute(json!({"action":"set_overflow","room_id":room_id,"id":unrelated}))
        .is_err());
    engine
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();
    assert_eq!(
        engine.snapshot().unwrap()["rooms"][0]["assets"][0]["verified_copies"],
        1
    );
    assert!(engine.blob(&room_id, &asset).unwrap().is_file());

    engine
        .execute(json!({"action":"set_overflow","room_id":room_id,"id":overflow}))
        .unwrap();
    engine
        .execute(json!({"action":"pause_source","id":overflow,"paused":true}))
        .unwrap();
    engine
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();
    assert!(engine
        .state
        .replicas
        .iter()
        .all(|replica| replica.asset_id != asset));
    engine
        .execute(json!({"action":"pause_source","id":overflow,"paused":false}))
        .unwrap();
    engine
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();
    assert!(engine
        .state
        .replicas
        .iter()
        .any(|replica| replica.asset_id == asset
            && replica.source_id == overflow
            && replica.verified));

    drop(engine);
    let mut reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    let sources = &reopened.snapshot().unwrap()["sources"];
    assert_eq!(
        sources
            .as_array()
            .unwrap()
            .iter()
            .filter(|source| source["overflow"] == true)
            .count(),
        1
    );
    reopened
        .execute(json!({"action":"set_overflow","room_id":room_id,"id":null}))
        .unwrap();
    assert!(reopened.snapshot().unwrap()["sources"]
        .as_array()
        .unwrap()
        .iter()
        .all(|source| source["overflow"] == false));
    reopened
        .execute(json!({"action":"set_overflow","room_id":room_id,"id":overflow}))
        .unwrap();
    reopened
        .execute(json!({"action":"prefer_source","id":overflow}))
        .unwrap();
    assert!(reopened.snapshot().unwrap()["sources"]
        .as_array()
        .unwrap()
        .iter()
        .all(|source| source["overflow"] == false));
}

fn failing_webdav_catalog(status: u16) -> (String, std::thread::JoinHandle<bool>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/room", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let started = std::time::Instant::now();
        let mut saw_catalog = false;
        while started.elapsed() < std::time::Duration::from_secs(8) && !saw_catalog {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            };
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 4096];
            let count = stream.read(&mut request).unwrap_or(0);
            let first_line = String::from_utf8_lossy(&request[..count]);
            let put = first_line.starts_with("PUT ");
            let body = if put {
                ""
            } else {
                "<d:multistatus xmlns:d=\"DAV:\"/>"
            };
            let code = if put { status } else { 207 };
            let reason = if put { "Rejected" } else { "Multi-Status" };
            write!(stream, "HTTP/1.1 {code} {reason}\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            stream.flush().unwrap();
            saw_catalog = put;
        }
        saw_catalog
    });
    (endpoint, server)
}

#[test]
fn catalog_capacity_failure_uses_overflow_but_other_provider_errors_do_not() {
    for (status, should_overflow) in [(507, true), (503, false)] {
        let (dir, mut engine) = vault();
        let spare = tempfile::tempdir().unwrap();
        let room_id = room(&mut engine);
        let asset = import(&mut engine, &room_id, &media(&dir));
        let (endpoint, server) = failing_webdav_catalog(status);
        engine
            .execute(json!({"action":"add_source","room_id":room_id,
            "name":"DAV","kind":"webdav","endpoint":endpoint,"token":"test"}))
            .unwrap();
        let overflow = engine
            .execute(json!({"action":"add_source","room_id":room_id,
            "name":"Spare","kind":"local","endpoint":spare.path()}))
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        engine
            .execute(json!({"action":"set_overflow","room_id":room_id,"id":overflow}))
            .unwrap();
        let result = engine
            .execute(json!({"action":"sync_all","room_id":room_id}))
            .unwrap();
        assert!(!result["errors"].as_array().unwrap().is_empty());
        assert_eq!(
            engine
                .state
                .replicas
                .iter()
                .any(|replica| replica.asset_id == asset
                    && replica.source_id == overflow
                    && replica.verified),
            should_overflow
        );
        assert!(engine.blob(&room_id, &asset).unwrap().is_file());
        assert!(server.join().unwrap());
    }
}

#[test]
fn source_without_overflow_field_decodes_as_unselected() {
    let source: crate::model::StorageSource = serde_json::from_value(json!({
        "id":"legacy","room_id":"room","name":"Local","kind":"local",
        "endpoint":"/tmp/memories","token":"","paused":false,"preferred":true,"budget":null
    }))
    .unwrap();
    assert!(!source.overflow);
}

#[test]
fn exhausted_preferred_and_overflow_sources_keep_local_original() {
    let (dir, mut engine) = vault();
    let primary_dir = tempfile::tempdir().unwrap();
    let overflow_dir = tempfile::tempdir().unwrap();
    let room_id = room(&mut engine);
    let asset = import(&mut engine, &room_id, &media(&dir));
    engine
        .execute(json!({"action":"add_source","room_id":room_id,
        "name":"Full","kind":"local","endpoint":primary_dir.path(),"budget":1}))
        .unwrap();
    let overflow = engine
        .execute(json!({"action":"add_source","room_id":room_id,
        "name":"Also full","kind":"local","endpoint":overflow_dir.path(),"budget":1}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    engine
        .execute(json!({"action":"set_overflow","room_id":room_id,"id":overflow}))
        .unwrap();
    engine
        .execute(json!({"action":"sync_all","room_id":room_id}))
        .unwrap();
    assert!(engine.blob(&room_id, &asset).unwrap().is_file());
    assert!(engine
        .state
        .replicas
        .iter()
        .all(|replica| replica.asset_id != asset));
    assert_eq!(
        engine
            .state
            .transfers
            .iter()
            .filter(|job| job.asset_id == asset && job.state == "waiting")
            .count(),
        2
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
fn skipping_private_weekly_reel_preserves_its_source_and_only_discards_auto_drafts() {
    let (dir, mut owner) = vault();
    let r = room(&mut owner);
    let source = import(&mut owner, &r, &media(&dir));
    let blob = owner.blob(&r, &source).unwrap();
    let original = fs::read(&blob).unwrap();
    let operations_before = owner.room(&r).unwrap().operations.len();

    owner
        .execute(json!({
            "action":"save_story",
            "project":{
                "id":"weekly-reel", "room_id":r, "title":"This week",
                "version_id":"", "editor":"", "width":128, "height":128,
                "clips":[{"id":"clip","asset_id":source,"track":0,"start":0,
                    "trim_in":0,"duration":3,"speed":1,"volume":1,"opacity":1,
                    "exposure":0,"saturation":1,"title":"","fade_in":0,"fade_out":0,
                    "opacity_keyframes":[],"volume_keyframes":[]}],
                "chapters":[], "published_asset":null, "auto_generated":true
            }
        }))
        .unwrap();
    owner
        .execute(json!({
            "action":"save_story",
            "project":{
                "id":"weekly-reel", "room_id":r, "title":"This week",
                "version_id":"", "editor":"", "width":128, "height":128,
                "clips":[{"id":"clip","asset_id":source,"track":0,"start":0,
                    "trim_in":0,"duration":3,"speed":1,"volume":1,"opacity":1,
                    "exposure":0,"saturation":1,"title":"","fade_in":0,"fade_out":0,
                    "opacity_keyframes":[],"volume_keyframes":[]}],
                "chapters":[], "published_asset":null, "auto_generated":true
            }
        }))
        .unwrap();

    let skipped = owner
        .execute(json!({"action":"skip_auto_reel","room_id":r,"id":"weekly-reel"}))
        .unwrap();
    assert_eq!(skipped["skipped"], true);
    assert_eq!(skipped["versions_removed"], 2);
    assert!(owner.snapshot().unwrap()["rooms"][0]["stories"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(owner.assets(&r).unwrap().len(), 1);
    assert_eq!(owner.room(&r).unwrap().operations.len(), operations_before);
    assert_eq!(fs::read(blob).unwrap(), original);

    let manual = owner
        .execute(json!({"action":"create_story","room_id":r}))
        .unwrap();
    let manual_id = manual["project"]["id"].as_str().unwrap();
    assert!(owner
        .execute(json!({"action":"skip_auto_reel","room_id":r,"id":manual_id}))
        .is_err());
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["stories"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn saving_private_weekly_reel_keeps_it_beyond_expiry() {
    let (dir, mut owner) = vault();
    let r = room(&mut owner);
    let source = import(&mut owner, &r, &media(&dir));
    let mut project = json!({
        "id":"kept-weekly-reel", "room_id":r, "title":"This week",
        "version_id":"", "editor":"", "width":128, "height":128,
        "clips":[{"id":"clip","asset_id":source,"track":0,"start":0,
            "trim_in":0,"duration":3,"speed":1,"volume":1,"opacity":1,
            "exposure":0,"saturation":1,"title":"","fade_in":0,"fade_out":0,
            "opacity_keyframes":[],"volume_keyframes":[]}],
        "chapters":[], "published_asset":null, "auto_generated":true
    });
    owner
        .execute(json!({"action":"save_story","project":project}))
        .unwrap();
    project["auto_generated"] = json!(false);
    owner
        .execute(json!({"action":"save_story","project":project}))
        .unwrap();

    let stories = owner.snapshot().unwrap()["rooms"][0]["stories"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(
        stories.len(),
        1,
        "keeping a reel should replace its expiring card"
    );
    assert_eq!(stories[0]["auto_generated"], false);
    owner
        .state
        .drafts
        .iter_mut()
        .find(|story| !story.auto_generated && !story.archived)
        .unwrap()
        .created_at = 1;
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["stories"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(owner.assets(&r).unwrap().len(), 1);
}

#[test]
fn expired_weekly_reel_is_hidden_without_removing_source_media() {
    let (dir, mut owner) = vault();
    let r = room(&mut owner);
    let source = import(&mut owner, &r, &media(&dir));
    let blob = owner.blob(&r, &source).unwrap();
    let original = fs::read(&blob).unwrap();
    owner
        .execute(json!({
            "action":"save_story",
            "project":{
                "id":"old-weekly-reel", "room_id":r, "title":"Old week",
                "version_id":"", "editor":"", "width":128, "height":128,
                "clips":[], "chapters":[], "published_asset":null, "auto_generated":true
            }
        }))
        .unwrap();
    owner.state.drafts[0].created_at = 1;

    assert!(owner.snapshot().unwrap()["rooms"][0]["stories"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(owner.assets(&r).unwrap().len(), 1);
    assert_eq!(fs::read(blob).unwrap(), original);
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
    let history = owner
        .execute(json!({"action":"asset_history","room_id":r,"asset_id":a}))
        .unwrap()["history"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(history.len(), 2);
    let owner_device = owner.device().unwrap();
    let member_device = member.device().unwrap();
    assert!(history.iter().any(|revision| {
        revision["caption"] == "Owner offline"
            && revision["author"].as_str() == Some(owner_device.as_str())
    }));
    assert!(history.iter().any(|revision| {
        revision["caption"] == "Member offline"
            && revision["author"].as_str() == Some(member_device.as_str())
    }));
    assert!(history
        .windows(2)
        .all(|pair| pair[0]["clock"].as_u64() <= pair[1]["clock"].as_u64()));
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
fn android_content_tree_watch_keeps_local_items_scoped_and_resumable() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let other_room = room(&mut owner);
    let tree_uri = "content://com.android.externalstorage.documents/tree/primary%3ADCIM%2FCamera";
    assert!(owner
        .execute(json!({
            "action":"watch_content_tree",
            "room_id":room_id,
            "tree_uri":"content://",
            "include_existing":false,
            "initial_seen":[]
        }))
        .is_err());
    let existing = "content:old-item:1024:1000";
    let created = "content:new-item:2048:2000";
    let watch_id = owner
        .execute(json!({
            "action":"watch_content_tree",
            "room_id":room_id,
            "tree_uri":tree_uri,
            "include_existing":false,
            "initial_seen":[existing, existing],
            "excluded_extensions":["gif"]
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let rule = owner.snapshot().unwrap()["watches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|watch| watch["id"] == watch_id)
        .unwrap()
        .clone();
    assert_eq!(rule["room_id"], room_id);
    assert_eq!(rule["path"], tree_uri);
    assert_eq!(rule["seen"].as_array().unwrap().len(), 1);
    assert_eq!(rule["excluded_extensions"][0], "gif");

    owner
        .execute(json!({"action":"acknowledge_watch_item","id":watch_id,"identifier":created}))
        .unwrap();
    owner
        .execute(json!({"action":"acknowledge_watch_item","id":watch_id,"identifier":created}))
        .unwrap();
    let saved = owner.snapshot().unwrap()["watches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|watch| watch["id"] == watch_id)
        .unwrap()
        .clone();
    assert_eq!(saved["seen"].as_array().unwrap().len(), 2);

    let updated = owner
        .execute(json!({
            "action":"watch_content_tree",
            "room_id":room_id,
            "tree_uri":tree_uri,
            "include_existing":false,
            "initial_seen":["content:item-added-after-setup:1024:3000"],
            "excluded_extensions":["mov"]
        }))
        .unwrap();
    assert_eq!(updated["id"], watch_id);
    let after_reselect = owner.snapshot().unwrap()["watches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|watch| watch["id"] == watch_id)
        .unwrap()
        .clone();
    assert_eq!(after_reselect["seen"].as_array().unwrap().len(), 2);
    assert_eq!(after_reselect["excluded_extensions"][0], "mov");

    let history_tree = "content://provider/tree/history";
    let historical_rule = owner
        .execute(json!({
            "action":"watch_content_tree",
            "room_id":room_id,
            "tree_uri":history_tree,
            "include_existing":true,
            "initial_seen":[]
        }))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(owner.snapshot().unwrap()["watches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|watch| watch["id"] == historical_rule)
        .unwrap()["seen"]
        .as_array()
        .unwrap()
        .is_empty());

    owner
        .execute(json!({"action":"pause_watch","id":watch_id,"paused":true}))
        .unwrap();
    assert_eq!(owner.snapshot().unwrap()["watches"][0]["paused"], true);
    owner.execute(json!({"action":"scan_watches"})).unwrap();
    assert!(owner.assets(&room_id).unwrap().is_empty());
    assert!(owner.assets(&other_room).unwrap().is_empty());

    let (_viewer_dir, mut viewer) = vault();
    invite(&mut owner, &mut viewer, &room_id, Role::Viewer);
    assert!(viewer
        .execute(json!({"action":"watch_content_tree","room_id":room_id,"tree_uri":tree_uri,"initial_seen":[]}))
        .is_err());
    assert!(viewer
        .execute(json!({"action":"pause_watch","id":watch_id,"paused":false}))
        .is_err());

    drop(owner);
    let reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    let recovered = reopened.snapshot().unwrap()["watches"].as_array().unwrap()[0].clone();
    assert_eq!(recovered["id"], watch_id);
    assert_eq!(recovered["seen"].as_array().unwrap().len(), 2);
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
    let moment_id = e.execute(json!({"action":"save_moment","room_id":r,"title":"First day","asset_ids":[a],"featured":true})).unwrap()["id"].as_str().unwrap().to_owned();
    e.execute(json!({"action":"save_moment","room_id":r,"id":moment_id,"title":"First day","asset_ids":[a],"featured":false})).unwrap();
    let saved_moments = e.snapshot().unwrap();
    assert_eq!(
        saved_moments["rooms"][0]["moments"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(saved_moments["rooms"][0]["moments"][0]["featured"], false);
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

fn published_film(
    engine: &mut Engine,
    room_id: &str,
    dir: &tempfile::TempDir,
    name: &str,
) -> String {
    let path = dir.path().join(format!("{}.mp4", engine::id()));
    fs::write(&path, format!("published test movie: {name}")).unwrap();
    let asset_id = import(engine, room_id, &path);
    let story_id = engine::id();
    let project = json!({
        "id":story_id,"room_id":room_id,"title":name,"version_id":"","editor":"",
        "width":1920,"height":1080,
        "clips":[{"id":engine::id(),"asset_id":asset_id,"track":0,"start":0,
            "trim_in":0,"duration":3,"speed":1,"volume":1,"opacity":1,
            "exposure":0,"saturation":1,"title":"","fade_in":0,"fade_out":0,
            "opacity_keyframes":[],"volume_keyframes":[]}],
        "chapters":[],"published_asset":asset_id
    });
    engine
        .execute(json!({"action":"save_story","project":project,"publish":true}))
        .unwrap();
    story_id
}

#[test]
fn playlists_reorder_and_survive_reopen_without_touching_movie_originals() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let first = published_film(&mut owner, &room_id, &dir, "First film");
    let second = published_film(&mut owner, &room_id, &dir, "Second film");
    let originals = owner.assets(&room_id).unwrap();
    let saved = owner
        .execute(json!({"action":"save_playlist","room_id":room_id,
        "title":"Family favorites","story_ids":[first,second]}))
        .unwrap();
    let playlist_id = saved["id"].as_str().unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["playlists"][0]["story_ids"],
        json!([first, second])
    );
    owner
        .execute(
            json!({"action":"save_playlist","room_id":room_id,"id":playlist_id,
        "title":"The Saturday queue","story_ids":[second,first]}),
        )
        .unwrap();
    assert_eq!(owner.assets(&room_id).unwrap().len(), originals.len());
    drop(owner);
    let reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    let playlist = &reopened.snapshot().unwrap()["rooms"][0]["playlists"][0];
    assert_eq!(playlist["title"], "The Saturday queue");
    assert_eq!(playlist["story_ids"], json!([second, first]));
    assert_eq!(reopened.assets(&room_id).unwrap().len(), originals.len());
}

#[test]
fn playlists_reject_drafts_foreign_films_duplicates_and_viewer_edits() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let film = published_film(&mut owner, &room_id, &dir, "Published");
    let draft = owner
        .execute(json!({"action":"create_story","room_id":room_id}))
        .unwrap()["project"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    for ids in [json!([draft]), json!(["unknown"]), json!([film, film])] {
        assert!(owner
            .execute(json!({"action":"save_playlist","room_id":room_id,
            "title":"Invalid queue","story_ids":ids}))
            .is_err());
    }
    let other_room = room(&mut owner);
    assert!(owner
        .execute(json!({"action":"save_playlist","room_id":other_room,
        "title":"Wrong Room","story_ids":[film]}))
        .is_err());
    let (_viewer_dir, mut viewer) = vault();
    invite(&mut owner, &mut viewer, &room_id, Role::Viewer);
    viewer
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();
    assert!(viewer
        .execute(json!({"action":"save_playlist","room_id":room_id,
        "title":"Viewer queue","story_ids":[]}))
        .is_err());
}

#[test]
fn playlists_sync_as_encrypted_room_metadata_and_converge_after_offline_reorders() {
    let (dir, mut owner) = vault();
    let (_member_dir, mut member) = vault();
    let room_id = room(&mut owner);
    let first = published_film(&mut owner, &room_id, &dir, "First");
    let second = published_film(&mut owner, &room_id, &dir, "Second");
    invite(&mut owner, &mut member, &room_id, Role::Contributor);
    let saved = owner
        .execute(json!({"action":"save_playlist","room_id":room_id,
        "title":"Films","story_ids":[first,second]}))
        .unwrap();
    let playlist_id = saved["id"].as_str().unwrap();
    member
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();
    assert_eq!(
        member.snapshot().unwrap()["rooms"][0]["playlists"][0]["story_ids"],
        json!([first, second])
    );
    owner
        .execute(
            json!({"action":"save_playlist","room_id":room_id,"id":playlist_id,
        "title":"Owner order","story_ids":[second,first]}),
        )
        .unwrap();
    member
        .execute(
            json!({"action":"save_playlist","room_id":room_id,"id":playlist_id,
        "title":"Member order","story_ids":[first]}),
        )
        .unwrap();
    let owner_packet = owner.packet(&room_id).unwrap();
    let member_packet = member.packet(&room_id).unwrap();
    owner.merge_packet(&room_id, &member_packet).unwrap();
    member.merge_packet(&room_id, &owner_packet).unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["playlists"],
        member.snapshot().unwrap()["rooms"][0]["playlists"]
    );
}

#[test]
fn albums_refuse_duplicate_entries_and_removal_preserves_the_original() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let asset = import(&mut owner, &room_id, &media(&dir));
    let second_path = dir.path().join("another-memory.jpg");
    fs::write(&second_path, b"another private original").unwrap();
    let second = import(&mut owner, &room_id, &second_path);
    assert!(owner
        .execute(
            json!({"action":"save_album","room_id":room_id,"title":"Duplicate",
        "asset_ids":[asset,asset]})
        )
        .is_err());
    let id = owner
        .execute(
            json!({"action":"save_album","room_id":room_id,"title":"One memory",
        "asset_ids":[asset,second]}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    owner
        .execute(json!({"action":"save_album","room_id":room_id,"id":id,
        "title":"Ordered memories","asset_ids":[second,asset]}))
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["albums"][0]["asset_ids"],
        json!([second, asset])
    );
    owner
        .execute(json!({"action":"save_album","room_id":room_id,"id":id,
        "title":"Now empty","asset_ids":[]}))
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap()["rooms"][0]["albums"][0]["asset_ids"],
        json!([])
    );
    assert_eq!(owner.assets(&room_id).unwrap().len(), 2);
    assert!(owner.blob(&room_id, &asset).unwrap().is_file());
    assert!(owner.blob(&room_id, &second).unwrap().is_file());
}

#[test]
fn deleting_a_playlist_hides_only_the_queue_on_every_device() {
    let (dir, mut owner) = vault();
    let (_member_dir, mut member) = vault();
    let room_id = room(&mut owner);
    let film = published_film(&mut owner, &room_id, &dir, "Keep this movie");
    invite(&mut owner, &mut member, &room_id, Role::Contributor);
    let playlist_id = owner
        .execute(json!({"action":"save_playlist","room_id":room_id,
        "title":"Temporary queue","story_ids":[film]}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    member
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();
    assert_eq!(
        member.snapshot().unwrap()["rooms"][0]["playlists"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    owner
        .execute(json!({"action":"delete_playlist","room_id":room_id,"id":playlist_id}))
        .unwrap();
    assert!(owner
        .execute(json!({"action":"delete_playlist","room_id":room_id,"id":playlist_id}))
        .is_err());
    member
        .merge_packet(&room_id, &owner.packet(&room_id).unwrap())
        .unwrap();
    for engine in [&owner, &member] {
        let snapshot = engine.snapshot().unwrap();
        assert!(snapshot["rooms"][0]["playlists"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(snapshot["rooms"][0]["stories"].as_array().unwrap().len(), 1);
        assert_eq!(snapshot["rooms"][0]["assets"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn video_search_labels_only_ordered_single_modality_phrases_as_exact() {
    let (dir, mut owner) = vault();
    let room_id = room(&mut owner);
    let movie = dir.path().join("phrase-search.mp4");
    fs::write(&movie, b"video for exact multimodal search").unwrap();
    let asset_id = import(&mut owner, &room_id, &movie);

    let index = |owner: &mut Engine,
                 audio_indexed: bool,
                 frames_indexed: bool,
                 observations: serde_json::Value| {
        owner
            .execute(json!({
                "action":"index_video", "room_id":room_id, "asset_id":asset_id,
                "audio_indexed":audio_indexed, "frames_indexed":frames_indexed,
                "frame_interval_ms":if frames_indexed { Some(2_000_u64) } else { None },
                "observations":observations,
            }))
            .unwrap();
    };
    let search = |owner: &mut Engine| {
        owner
            .execute(json!({
                "action":"search_video", "room_id":room_id,
                "query":"happy birthday", "modality":"all"
            }))
            .unwrap()
    };

    index(
        &mut owner,
        true,
        true,
        json!([
            {"modality":"speech", "start_ms":10_000, "end_ms":10_000,
             "text":"birthday", "confidence_percent":95},
            {"modality":"screen_text", "start_ms":10_500, "end_ms":10_500,
             "text":"happy", "confidence_percent":95}
        ]),
    );
    let cross_modality = search(&mut owner);
    assert_eq!(cross_modality["matches"][0]["match_kind"], "suggestion");

    index(
        &mut owner,
        false,
        true,
        json!([{"modality":"screen_text", "start_ms":20_000, "end_ms":20_000,
            "text":"birthday happy", "confidence_percent":95}]),
    );
    let reversed = search(&mut owner);
    assert_eq!(reversed["matches"][0]["match_kind"], "suggestion");

    index(
        &mut owner,
        true,
        false,
        json!([
            {"modality":"speech", "start_ms":30_000, "end_ms":30_300,
             "text":"happy", "confidence_percent":95},
            {"modality":"speech", "start_ms":30_500, "end_ms":30_900,
             "text":"birthday", "confidence_percent":95}
        ]),
    );
    let ordered_transcript = search(&mut owner);
    assert_eq!(ordered_transcript["matches"][0]["match_kind"], "exact");
    assert_eq!(ordered_transcript["matches"][0]["start_ms"], 30_000);
    assert_eq!(ordered_transcript["matches"][0]["end_ms"], 30_900);

    let natural_language = owner
        .execute(json!({
            "action":"search_video", "room_id":room_id,
            "query":"Find the exact spoken words happy birthday", "modality":"audio"
        }))
        .unwrap();
    assert_eq!(natural_language["matches"][0]["match_kind"], "exact");
    assert_eq!(natural_language["matches"][0]["start_ms"], 30_000);
}

#[test]
fn similar_photo_suggestions_are_scoped_and_never_mutate_originals() {
    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let first_path = dir.path().join("first.png");
    let second_path = dir.path().join("second.png");
    image::GrayImage::from_fn(90, 80, |x, _| image::Luma([(x * 2) as u8]))
        .save(&first_path)
        .unwrap();
    image::GrayImage::from_fn(90, 80, |x, _| image::Luma([(x * 2 + 20) as u8]))
        .save(&second_path)
        .unwrap();

    let first_id = import(&mut core, &room_id, &first_path);
    let second_id = import(&mut core, &room_id, &second_path);
    assert_ne!(first_id, second_id, "different originals remain distinct");

    let first_blob = core.blob(&room_id, &first_id).unwrap();
    let second_blob = core.blob(&room_id, &second_id).unwrap();
    let original_encrypted_bytes = [
        fs::read(&first_blob).unwrap(),
        fs::read(&second_blob).unwrap(),
    ];
    let operations_before = core.room(&room_id).unwrap().operations.len();
    // Simulate a catalog created before local photo fingerprints existed.
    core.state.photo_fingerprints.clear();
    let before = core.snapshot().unwrap();
    let response = core
        .execute(json!({
            "action":"suggest_photo_duplicates",
            "room_id":room_id
        }))
        .unwrap();
    let after = core.snapshot().unwrap();

    assert_eq!(response["basis"], "perceptual_difference_hash");
    assert_eq!(response["newly_indexed"], 2);
    assert_eq!(core.state.photo_fingerprints.len(), 2);
    assert_eq!(response["suggestions"].as_array().unwrap().len(), 1);
    assert_ne!(
        response["suggestions"][0]["first"]["asset_id"],
        response["suggestions"][0]["second"]["asset_id"]
    );
    assert_eq!(before["rooms"][0]["assets"].as_array().unwrap().len(), 2);
    assert_eq!(after["rooms"][0]["assets"].as_array().unwrap().len(), 2);
    assert!(after["rooms"][0]["assets"]
        .as_array()
        .unwrap()
        .iter()
        .all(|asset| asset["deleted"] == false));
    assert_eq!(
        core.room(&room_id).unwrap().operations.len(),
        operations_before
    );
    assert_eq!(fs::read(&first_blob).unwrap(), original_encrypted_bytes[0]);
    assert_eq!(fs::read(&second_blob).unwrap(), original_encrypted_bytes[1]);

    drop(core);
    let mut reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    assert_eq!(reopened.state.photo_fingerprints.len(), 2);
    let reopened_response = reopened
        .execute(json!({
            "action":"suggest_photo_duplicates",
            "room_id":room_id
        }))
        .unwrap();
    assert_eq!(reopened_response["newly_indexed"], 0);
    assert_eq!(reopened_response["indexed_photos"], 2);
    assert_eq!(
        reopened_response["suggestions"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        reopened.room(&room_id).unwrap().operations.len(),
        operations_before
    );
}

#[test]
fn bounded_decryption_authenticates_and_rejects_plaintext_over_the_memory_limit() {
    let dir = tempfile::tempdir().unwrap();
    let original = dir.path().join("original.bin");
    let encrypted = dir.path().join("original.fr");
    let plaintext = vec![0x5a; 96 * 1024];
    fs::write(&original, &plaintext).unwrap();
    let key = [23; 32];
    crate::crypto::encrypt_file(&key, &original, &encrypted, "bounded-photo-test").unwrap();
    let (expected_hash, size) = crate::crypto::digest(&original).unwrap();

    let decrypted = crate::crypto::decrypt_file_bounded(
        &key,
        &encrypted,
        "bounded-photo-test",
        &expected_hash,
        size as usize,
    )
    .unwrap();
    assert_eq!(decrypted, plaintext);

    let too_small = crate::crypto::decrypt_file_bounded(
        &key,
        &encrypted,
        "bounded-photo-test",
        &expected_hash,
        size as usize - 1,
    );
    assert!(
        too_small.is_err(),
        "oversized plaintext must not be returned"
    );

    assert!(crate::crypto::decrypt_file_bounded(
        &key,
        &encrypted,
        "wrong-context",
        &expected_hash,
        size as usize,
    )
    .is_err());
    assert!(crate::crypto::decrypt_file_bounded(
        &key,
        &encrypted,
        "bounded-photo-test",
        &"00".repeat(32),
        size as usize,
    )
    .is_err());
}

#[test]
#[ignore = "requires FFmpeg, FFprobe, and Tesseract on PATH"]
fn desktop_video_ocr_flows_through_encrypted_exact_search() {
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../family-video-indexer/tests/fixtures/frame-birthday.mp4");
    let analysis = family_video_indexer::analyze_video_scoped(&fixture, false, true).unwrap();
    assert!(analysis.frames_indexed);
    assert!(analysis
        .observations
        .iter()
        .any(|item| item.modality == "screen_text" && item.text == "HAPPY BIRTHDAY"));

    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &fixture);
    core.execute(json!({
        "action":"index_video",
        "room_id":room_id,
        "asset_id":asset_id,
        "audio_indexed":analysis.audio_indexed,
        "frames_indexed":analysis.frames_indexed,
        "visual_labels_indexed":analysis.visual_labels_indexed,
        "frame_interval_ms":analysis.frame_interval_ms,
        "observations":analysis.observations
    }))
    .unwrap();

    let exact_query = || {
        json!({
            "action":"search_video",
            "room_id":room_id,
            "query":"\"HAPPY BIRTHDAY\"",
            "modality":"frame"
        })
    };
    let result = core.execute(exact_query()).unwrap();
    assert_eq!(result["matches"][0]["asset_id"], asset_id);
    assert_eq!(result["matches"][0]["match_kind"], "exact");
    assert_eq!(
        result["matches"][0]["evidence"][0]["text"],
        "HAPPY BIRTHDAY"
    );

    drop(core);
    let mut reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    let persisted = reopened.execute(exact_query()).unwrap();
    assert_eq!(persisted["matches"][0]["asset_id"], asset_id);
    assert_eq!(persisted["matches"][0]["match_kind"], "exact");
}

#[test]
#[ignore = "requires FFmpeg, FFprobe, whisper.cpp, and a local English model"]
fn desktop_video_speech_flows_through_encrypted_exact_search() {
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../platforms/android/app/src/androidTest/assets/spoken-birthday.mp4");
    let analysis = family_video_indexer::analyze_video_scoped(&fixture, true, false).unwrap();
    assert!(analysis.audio_indexed);
    assert!(analysis.observations.iter().any(|item| {
        item.modality == "speech" && item.text.to_ascii_lowercase().contains("birthday")
    }));

    let (dir, mut core) = vault();
    let room_id = room(&mut core);
    let asset_id = import(&mut core, &room_id, &fixture);
    core.execute(json!({
        "action":"index_video",
        "room_id":room_id,
        "asset_id":asset_id,
        "audio_indexed":analysis.audio_indexed,
        "frames_indexed":analysis.frames_indexed,
        "observations":analysis.observations
    }))
    .unwrap();

    let exact_query = || {
        json!({
            "action":"search_video",
            "room_id":room_id,
            "query":"\"birthday\"",
            "modality":"audio"
        })
    };
    let result = core.execute(exact_query()).unwrap();
    assert_eq!(result["matches"][0]["asset_id"], asset_id);
    assert_eq!(result["matches"][0]["match_kind"], "exact");
    assert!(result["matches"][0]["start_ms"].as_u64().is_some());

    drop(core);
    let mut reopened = Engine::open(dir.path(), [7; 32]).unwrap();
    let persisted = reopened.execute(exact_query()).unwrap();
    assert_eq!(persisted["matches"][0]["asset_id"], asset_id);
    assert_eq!(persisted["matches"][0]["match_kind"], "exact");
}
