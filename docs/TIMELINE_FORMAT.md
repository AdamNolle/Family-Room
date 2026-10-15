# Portable film format 1

`StoryProject` carries `format_version: 1`. Unversioned legacy projects are read as format 1. Unknown versions require an app update: the core refuses editing, saving or rewriting a local catalog containing them. Native clients must not drop unknown fields and then save the result. This is a format guard, not a complete peer capability negotiation service.

The project identifies its Room, immutable version/editor, dimensions, title, chapters and optional published movie. Clips refer to media asset identities rather than file paths. Each clip carries its layer, start, source trim/duration, speed, gain, opacity, exposure, saturation, title, fades and automation. Originals stay unchanged.

| Field | Unit / meaning |
|---|---|
| `start` | Seconds on the output timeline |
| `trim_in`, `duration` | Source seconds |
| `speed` | Source seconds per output second; clip ends at `start + duration / speed` |
| Keyframe `time` | Source seconds relative to this clip's trim point |
| `fade_in`, `fade_out` | Output seconds |
| `fade_in_offset` | Output seconds already elapsed in the original fade-in |
| `fade_out_offset` | Output seconds remaining beyond this piece in the original fade-out |
| Chapter `time` | Seconds on the output timeline |

Automation sorts by time and linearly interpolates values; outside its endpoints it holds the closest value. Empty automation uses the clip's constant value. Fade gain multiplies opacity and audio gain. Existing offsets default to zero when absent. Tracks are numbered 0–15. The core bounds dimensions, project length, source times, gains and record sizes before accepting a project.

`create_story` saves a new private draft immediately. `split_story_clip` validates Room contribution permission and referenced assets, derives two pieces and returns the project; it does not publish or save a new revision. It retains the left identity, gives the right piece a new identity, moves its source trim/start, slices and rebases automation, and preserves fades at the cut. Native clients apply the response only if the draft has not changed while the command runs.

Shortening a clip retains an interpolated automation endpoint at the cut, so earlier keyframes keep their meaning. Its fade-out follows the shortened endpoint; splitting preserves the original fade instead. Extending holds the existing final automation value. Undo restores the original project.

`save_story` stores immutable private revisions unless publication is explicitly requested. Private edits to a published movie retain that movie reference without publishing edits. Shared project editor leases, a revision browser, subtitles and equivalent output certification and Windows/Linux native rendering remain release work. A supported project format does not imply that every client can render every feature.
