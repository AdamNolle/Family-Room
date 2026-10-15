//! Reviewable Moment candidates based on local capture-time clusters.
//!
//! Suggestions are derived on demand and never modify the Room catalog. The
//! user must explicitly accept a suggestion through `save_moment`.

use crate::model::{MediaAsset, Moment};
use serde::Serialize;
use std::collections::HashSet;

const MINIMUM_ASSETS: usize = 3;
const MAX_CAPTURE_GAP_SECONDS: i64 = 3 * 60 * 60;
const MAX_MOMENT_SPAN_SECONDS: i64 = 12 * 60 * 60;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct MomentSuggestion {
    pub start: i64,
    pub end: i64,
    pub asset_ids: Vec<String>,
}

pub(crate) fn suggest_moments(
    room_id: &str,
    assets: &[MediaAsset],
    moments: &[Moment],
) -> Vec<MomentSuggestion> {
    let assigned: HashSet<&str> = moments
        .iter()
        .flat_map(|moment| moment.asset_ids.iter().map(String::as_str))
        .collect();
    let mut candidates: Vec<_> = assets
        .iter()
        .filter(|asset| {
            asset.room_id == room_id
                && matches!(asset.kind.as_str(), "photo" | "video")
                && !asset.deleted
                && !assigned.contains(asset.id.as_str())
        })
        .collect();
    candidates.sort_by(|left, right| {
        left.captured_at
            .cmp(&right.captured_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    let mut result = Vec::new();
    let mut cluster: Vec<&MediaAsset> = Vec::new();
    for asset in candidates {
        let starts_new_cluster = cluster.last().is_some_and(|last| {
            asset.captured_at.saturating_sub(last.captured_at) > MAX_CAPTURE_GAP_SECONDS
                || asset.captured_at.saturating_sub(cluster[0].captured_at)
                    > MAX_MOMENT_SPAN_SECONDS
        });
        if starts_new_cluster {
            push_cluster(&mut result, &cluster);
            cluster.clear();
        }
        cluster.push(asset);
    }
    push_cluster(&mut result, &cluster);
    result.sort_by_key(|suggestion| std::cmp::Reverse(suggestion.start));
    result
}

fn push_cluster(result: &mut Vec<MomentSuggestion>, cluster: &[&MediaAsset]) {
    if cluster.len() < MINIMUM_ASSETS {
        return;
    }
    result.push(MomentSuggestion {
        start: cluster[0].captured_at,
        end: cluster[cluster.len() - 1].captured_at,
        asset_ids: cluster.iter().map(|asset| asset.id.clone()).collect(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(id: &str, kind: &str, captured_at: i64) -> MediaAsset {
        MediaAsset {
            id: id.into(),
            room_id: "room-a".into(),
            filename: format!("{id}.{kind}"),
            kind: kind.into(),
            sha256: format!("hash-{id}"),
            size: 1,
            captured_at,
            contributed_at: 0,
            contributor: "member".into(),
            favorite: false,
            caption: String::new(),
            deleted: false,
            people: Vec::new(),
            components: Vec::new(),
            key_epoch: 0,
        }
    }

    #[test]
    fn groups_three_nearby_photos_and_videos_in_stable_order() {
        let assets = vec![
            asset("later-video", "video", 86_400),
            asset("first-photo", "photo", 0),
            asset("later-photo", "photo", 86_401),
            asset("middle-video", "video", 60),
            asset("first-video", "video", 120),
            asset("last-photo", "photo", 86_402),
        ];

        let suggestions = suggest_moments("room-a", &assets, &[]);

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].start, 86_400);
        assert_eq!(suggestions[0].end, 86_402);
        assert_eq!(
            suggestions[0].asset_ids,
            ["later-video", "later-photo", "last-photo"]
        );
        assert_eq!(
            suggestions[1].asset_ids,
            ["first-photo", "middle-video", "first-video"]
        );
    }

    #[test]
    fn capture_gap_boundary_is_inclusive_but_larger_gaps_split() {
        let at_boundary = vec![
            asset("one", "photo", 0),
            asset("two", "photo", MAX_CAPTURE_GAP_SECONDS),
            asset("three", "photo", MAX_CAPTURE_GAP_SECONDS * 2),
        ];
        assert_eq!(suggest_moments("room-a", &at_boundary, &[]).len(), 1);

        let over_boundary = vec![
            asset("one", "photo", 0),
            asset("two", "photo", MAX_CAPTURE_GAP_SECONDS + 1),
            asset("three", "photo", MAX_CAPTURE_GAP_SECONDS * 2 + 2),
        ];
        assert!(suggest_moments("room-a", &over_boundary, &[]).is_empty());
    }

    #[test]
    fn maximum_span_prevents_long_chains_from_becoming_one_moment() {
        let assets: Vec<_> = (0..6)
            .map(|index| {
                asset(
                    &format!("asset-{index}"),
                    "photo",
                    index * MAX_CAPTURE_GAP_SECONDS,
                )
            })
            .collect();

        let suggestions = suggest_moments("room-a", &assets, &[]);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].start, 0);
        assert_eq!(suggestions[0].end, MAX_MOMENT_SPAN_SECONDS);
        assert_eq!(suggestions[0].asset_ids.len(), 5);
    }

    #[test]
    fn excludes_deleted_other_room_non_media_and_already_assigned_items() {
        let mut deleted = asset("deleted", "photo", 10);
        deleted.deleted = true;
        let mut remote_room = asset("other-room", "photo", 10);
        remote_room.room_id = "room-b".into();
        let assigned = asset("assigned", "photo", 10);
        let assets = vec![
            assigned.clone(),
            asset("eligible-one", "photo", 10),
            asset("eligible-two", "video", 10),
            deleted,
            remote_room,
            asset("audio", "audio", 10),
            asset("file", "file", 10),
        ];
        let moments = vec![Moment {
            id: "saved".into(),
            room_id: "room-a".into(),
            title: "Already organized".into(),
            start: 10,
            end: 10,
            asset_ids: vec![assigned.id],
            featured: false,
        }];

        assert!(suggest_moments("room-a", &assets, &moments).is_empty());
    }
}
