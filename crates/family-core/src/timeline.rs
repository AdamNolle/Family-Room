use crate::{
    error::invalid,
    model::{Keyframe, StoryProject},
    CoreError,
};

pub(crate) const FORMAT_VERSION: u32 = 1;

pub(crate) fn default_format_version() -> u32 {
    FORMAT_VERSION
}

fn value_at(time: f64, frames: &[Keyframe], fallback: f64) -> f64 {
    let mut frames: Vec<_> = frames.iter().collect();
    frames.sort_by(|a, b| a.time.total_cmp(&b.time));
    let Some(first) = frames.first() else {
        return fallback;
    };
    if time <= first.time {
        return first.value;
    }
    for pair in frames.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if time <= b.time {
            return if a.time == b.time {
                b.value
            } else {
                a.value + (b.value - a.value) * ((time - a.time) / (b.time - a.time))
            };
        }
    }
    frames.last().map_or(fallback, |frame| frame.value)
}

fn slice(frames: &[Keyframe], from: f64, to: f64, fallback: f64) -> Vec<Keyframe> {
    if frames.is_empty() {
        return vec![];
    }
    let mut middle: Vec<_> = frames
        .iter()
        .filter(|frame| frame.time > from && frame.time < to)
        .collect();
    middle.sort_by(|a, b| a.time.total_cmp(&b.time));
    let mut result = vec![Keyframe {
        time: 0.0,
        value: value_at(from, frames, fallback),
    }];
    result.extend(middle.into_iter().map(|frame| Keyframe {
        time: frame.time - from,
        value: frame.value,
    }));
    result.push(Keyframe {
        time: to - from,
        value: value_at(to, frames, fallback),
    });
    result
}

/// Source-time split shared through the Swift/Kotlin/C JSON boundary.
/// The caller validates the project before and after applying the edit.
pub(crate) fn split(
    story: &mut StoryProject,
    clip_id: &str,
    source_time: f64,
) -> Result<(), CoreError> {
    let index = story
        .clips
        .iter()
        .position(|clip| clip.id == clip_id)
        .ok_or_else(|| invalid("Select a timeline clip to split"))?;
    let original = story.clips[index].clone();
    if !source_time.is_finite() || source_time < 0.001 || original.duration - source_time < 0.001 {
        return Err(invalid(
            "The split must leave at least one millisecond on each side",
        ));
    }
    let mut left = original.clone();
    let mut right = original.clone();
    left.duration = source_time;
    right.id = crate::engine::id();
    right.start += source_time / right.speed;
    right.trim_in += source_time;
    right.duration -= source_time;
    right.fade_in_offset = (right.fade_in_offset + source_time / right.speed).min(86400.0);
    left.fade_out_offset = (left.fade_out_offset + right.duration / right.speed).min(86400.0);
    left.opacity_keyframes = slice(
        &original.opacity_keyframes,
        0.0,
        source_time,
        original.opacity,
    );
    right.opacity_keyframes = slice(
        &original.opacity_keyframes,
        source_time,
        original.duration,
        original.opacity,
    );
    left.volume_keyframes = slice(
        &original.volume_keyframes,
        0.0,
        source_time,
        original.volume,
    );
    right.volume_keyframes = slice(
        &original.volume_keyframes,
        source_time,
        original.duration,
        original.volume,
    );
    story.clips[index] = left;
    story.clips.push(right);
    Ok(())
}
