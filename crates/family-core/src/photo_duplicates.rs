//! Reviewable near-duplicate photo candidates from bounded local decoding.
//!
//! Fingerprints are cached in the encrypted device catalog and never synced.
//! Suggestions never mutate the Room catalog or original files.

use crate::model::{MediaAsset, PhotoFingerprint};
use image::{imageops::FilterType, ImageReader, Limits};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{BufRead, Cursor, Seek},
    path::Path,
};

const HAMMING_DISTANCE_LIMIT: u32 = 4;
const MAX_CANDIDATE_PAIRS: usize = 250_000;
const MAX_SUGGESTIONS: usize = 200;
const MAX_IMAGE_AXIS: u32 = 32_768;
const MAX_IMAGE_PIXELS: u64 = 16_000_000;
const MAX_DECODE_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_ENCRYPTED_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
const HASH_BANDS: [(u32, u32); 6] = [(0, 11), (11, 11), (22, 11), (33, 11), (44, 10), (54, 10)];

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct PhotoDuplicateMember {
    pub asset_id: String,
    pub filename: String,
    pub captured_at: i64,
    pub size: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct PhotoDuplicateSuggestion {
    pub first: PhotoDuplicateMember,
    pub second: PhotoDuplicateMember,
    pub different_bits: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct PhotoDuplicateAnalysis {
    pub suggestions: Vec<PhotoDuplicateSuggestion>,
    pub comparison_limit_reached: bool,
    pub result_limit_reached: bool,
}

/// Computes a horizontal difference hash without retaining decoded pixels.
/// Unsupported or oversized images remain available but are skipped.
pub(crate) fn perceptual_hash(path: &Path) -> Option<u64> {
    if std::fs::metadata(path).ok()?.len() > MAX_ENCRYPTED_SOURCE_BYTES {
        return None;
    }
    let reader = ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let format = reader.format()?;
    let dimensions = reader.into_dimensions().ok()?;
    let reader = ImageReader::open(path).ok()?;
    hash_reader(reader, format, dimensions)
}

pub(crate) fn perceptual_hash_bytes(bytes: &[u8]) -> Option<u64> {
    if bytes.len() as u64 > MAX_ENCRYPTED_SOURCE_BYTES {
        return None;
    }
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let format = reader.format()?;
    let dimensions = reader.into_dimensions().ok()?;
    hash_reader(ImageReader::new(Cursor::new(bytes)), format, dimensions)
}

fn hash_reader<R: BufRead + Seek>(
    reader: ImageReader<R>,
    format: image::ImageFormat,
    (width, height): (u32, u32),
) -> Option<u64> {
    if width == 0
        || height == 0
        || width > MAX_IMAGE_AXIS
        || height > MAX_IMAGE_AXIS
        || u64::from(width).saturating_mul(u64::from(height)) > MAX_IMAGE_PIXELS
    {
        return None;
    }

    let mut reader = ImageReader::with_format(reader.into_inner(), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_AXIS);
    limits.max_image_height = Some(MAX_IMAGE_AXIS);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    let resized = reader
        .decode()
        .ok()?
        .resize_exact(9, 8, FilterType::Triangle)
        .to_luma8();

    let mut hash = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            if resized.get_pixel(x, y)[0] > resized.get_pixel(x + 1, y)[0] {
                hash |= 1 << (y * 8 + x);
            }
        }
    }
    Some(hash)
}

pub(crate) fn suggest_photo_duplicates(
    room_id: &str,
    assets: &[MediaAsset],
    fingerprints: &[PhotoFingerprint],
) -> PhotoDuplicateAnalysis {
    let fingerprints: BTreeMap<_, _> = fingerprints
        .iter()
        .filter(|fingerprint| fingerprint.room_id == room_id)
        .map(|fingerprint| {
            (
                (
                    fingerprint.room_id.as_str(),
                    fingerprint.asset_id.as_str(),
                    fingerprint.asset_sha256.as_str(),
                ),
                fingerprint.difference_hash,
            )
        })
        .collect();

    let mut candidates: Vec<_> = assets
        .iter()
        .filter_map(|asset| {
            let eligible = asset.room_id == room_id && asset.kind == "photo" && !asset.deleted;
            let hash = eligible.then(|| {
                fingerprints
                    .get(&(
                        asset.room_id.as_str(),
                        asset.id.as_str(),
                        asset.sha256.as_str(),
                    ))
                    .copied()
            })??;
            Some(Candidate { asset, hash })
        })
        .collect();
    candidates.sort_by(|left, right| {
        left.asset
            .captured_at
            .cmp(&right.asset.captured_at)
            .then_with(|| left.asset.id.cmp(&right.asset.id))
    });

    // With six disjoint bands, any pair within four changed bits must share
    // at least one entire band. This avoids all-pairs work for large Rooms.
    let mut buckets: BTreeMap<(u8, u16), Vec<usize>> = BTreeMap::new();
    let mut possible_pairs = BTreeSet::new();
    let mut comparison_limit_reached = false;
    'assets: for (index, asset) in candidates.iter().enumerate() {
        for (band_index, (offset, width)) in HASH_BANDS.iter().copied().enumerate() {
            let mask = (1u64 << width) - 1;
            let value = ((asset.hash >> offset) & mask) as u16;
            let bucket = buckets.entry((band_index as u8, value)).or_default();
            for previous in bucket.iter().copied() {
                possible_pairs.insert((previous, index));
                if possible_pairs.len() >= MAX_CANDIDATE_PAIRS {
                    comparison_limit_reached = true;
                    break 'assets;
                }
            }
            bucket.push(index);
        }
    }

    let mut suggestions: Vec<_> = possible_pairs
        .into_iter()
        .filter_map(|(left_index, right_index)| {
            let left = candidates[left_index];
            let right = candidates[right_index];
            let different_bits = (left.hash ^ right.hash).count_ones();
            (different_bits <= HAMMING_DISTANCE_LIMIT).then(|| PhotoDuplicateSuggestion {
                first: duplicate_member(left.asset),
                second: duplicate_member(right.asset),
                different_bits,
            })
        })
        .collect();
    suggestions.sort_by(|left, right| {
        left.different_bits
            .cmp(&right.different_bits)
            .then_with(|| left.first.captured_at.cmp(&right.first.captured_at))
            .then_with(|| left.first.asset_id.cmp(&right.first.asset_id))
            .then_with(|| left.second.asset_id.cmp(&right.second.asset_id))
    });
    let result_limit_reached = suggestions.len() > MAX_SUGGESTIONS;
    suggestions.truncate(MAX_SUGGESTIONS);

    PhotoDuplicateAnalysis {
        suggestions,
        comparison_limit_reached,
        result_limit_reached,
    }
}

#[derive(Clone, Copy)]
struct Candidate<'a> {
    asset: &'a MediaAsset,
    hash: u64,
}

fn duplicate_member(asset: &MediaAsset) -> PhotoDuplicateMember {
    PhotoDuplicateMember {
        asset_id: asset.id.clone(),
        filename: asset.filename.clone(),
        captured_at: asset.captured_at,
        size: asset.size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};

    fn asset(id: &str, room_id: &str, kind: &str, captured_at: i64) -> MediaAsset {
        MediaAsset {
            id: id.into(),
            room_id: room_id.into(),
            filename: format!("{id}.jpg"),
            kind: kind.into(),
            sha256: format!("sha-{id}"),
            size: 128,
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

    fn fingerprint(asset: &MediaAsset, hash: u64) -> PhotoFingerprint {
        PhotoFingerprint {
            room_id: asset.room_id.clone(),
            asset_id: asset.id.clone(),
            asset_sha256: asset.sha256.clone(),
            difference_hash: hash,
        }
    }

    #[test]
    fn finds_close_photo_pairs_but_not_distant_or_non_photo_items() {
        let mut deleted = asset("deleted", "room-a", "photo", 1);
        deleted.deleted = true;
        let other_room = asset("other-room", "room-b", "photo", 2);
        let video = asset("video", "room-a", "video", 3);
        let first = asset("first", "room-a", "photo", 10);
        let close = asset("close", "room-a", "photo", 20);
        let far = asset("far", "room-a", "photo", 30);
        let assets = vec![
            first.clone(),
            close.clone(),
            far.clone(),
            deleted.clone(),
            other_room.clone(),
            video.clone(),
        ];
        let fingerprints = vec![
            fingerprint(&first, 0),
            fingerprint(&close, 0b111),
            fingerprint(&far, 0b1_1111_0000),
            fingerprint(&deleted, 0),
            fingerprint(&other_room, 0),
            fingerprint(&video, 0),
        ];

        let analysis = suggest_photo_duplicates("room-a", &assets, &fingerprints);

        assert_eq!(analysis.suggestions.len(), 1);
        assert_eq!(analysis.suggestions[0].first.asset_id, "first");
        assert_eq!(analysis.suggestions[0].second.asset_id, "close");
        assert_eq!(analysis.suggestions[0].different_bits, 3);
        assert!(!analysis.comparison_limit_reached);
        assert!(!analysis.result_limit_reached);
    }

    #[test]
    fn ignores_fingerprints_for_a_replaced_original() {
        let first = asset("first", "room-a", "photo", 10);
        let second = asset("second", "room-a", "photo", 20);
        let mut stale = fingerprint(&second, 0);
        stale.asset_sha256 = "previous-content-hash".into();

        let analysis = suggest_photo_duplicates(
            "room-a",
            &[first.clone(), second.clone()],
            &[fingerprint(&first, 0), stale],
        );

        assert!(analysis.suggestions.is_empty());
    }

    #[test]
    fn caps_result_count_and_reports_when_more_matches_are_available() {
        let assets: Vec<_> = (0..24)
            .map(|index| asset(&format!("photo-{index}"), "room-a", "photo", index))
            .collect();
        let fingerprints: Vec<_> = assets.iter().map(|asset| fingerprint(asset, 0)).collect();

        let analysis = suggest_photo_duplicates("room-a", &assets, &fingerprints);

        assert_eq!(analysis.suggestions.len(), MAX_SUGGESTIONS);
        assert!(analysis.result_limit_reached);
        assert!(!analysis.comparison_limit_reached);
    }

    #[test]
    fn caps_candidate_work_for_a_bucket_with_many_identical_fingerprints() {
        let assets: Vec<_> = (0..710)
            .map(|index| asset(&format!("photo-{index}"), "room-a", "photo", index))
            .collect();
        let fingerprints: Vec<_> = assets.iter().map(|asset| fingerprint(asset, 0)).collect();

        let analysis = suggest_photo_duplicates("room-a", &assets, &fingerprints);

        assert!(analysis.comparison_limit_reached);
        assert!(analysis.result_limit_reached);
        assert_eq!(analysis.suggestions.len(), MAX_SUGGESTIONS);
    }

    #[test]
    fn generates_stable_difference_hashes_for_supported_images() {
        let directory = tempfile::tempdir().unwrap();
        let original_path = directory.path().join("original.png");
        let brighter_path = directory.path().join("brighter.png");
        let original = GrayImage::from_fn(90, 80, |x, _| Luma([(x * 2) as u8]));
        let brighter = GrayImage::from_fn(90, 80, |x, _| Luma([(x * 2 + 20) as u8]));
        original.save(&original_path).unwrap();
        brighter.save(&brighter_path).unwrap();

        assert_eq!(
            perceptual_hash(&original_path),
            perceptual_hash(&brighter_path)
        );
    }

    #[test]
    fn unsupported_image_files_are_skipped_without_blocking_import() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("unsupported.heic");
        std::fs::write(&path, b"image decoder not available").unwrap();

        assert_eq!(perceptual_hash(&path), None);
    }
}
