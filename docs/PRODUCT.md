# Family Room product contract

Family Room gives private groups one shared photo/video library through native iOS, iPadOS, macOS, Android, Windows and Linux apps. Members choose local, cloud, home-server and peer copies. Provider membership must never be a prerequisite for joining.

## Vocabulary

A **Room** is a private circle. A **Member** holds access. A **Person** is depicted in media. A **Moment** is an occasion, an **Album** a curated collection, a **Story** a film project and its publication, and a **Storage source** a location holding copies. Naming a Person never invites them.

An **Album** holds ordered photos and videos without owning their originals. A **Playlist** holds published Stories in a chosen watch order; removing a Story from a Playlist leaves the film and its source media in the Room. Draft Stories cannot enter a Playlist. Room contributors can arrange collections, while viewers can browse and play those shared with them.

## Experience

Home, Library, Albums, People and Theater are primary navigation. Room management, storage and preferences live behind the Room switcher/settings. Search and Add stay readily available. Phones use focused navigation; tablets use a sidebar and touch-friendly canvas; desktops use keyboard shortcuts, multi-selection and inspectors.

Cozy Glass lets photos dominate: warm framing, neutral image beds, serif editorial headings and precise native controls. Glass is selective. The optional scattered wall complements the grid. Moments are featured by people, never ranked by a numeric family-importance score. Home can resurface photos and videos captured on this local calendar date in earlier years; February 29 memories appear on February 28 in non-leap years. DVD nostalgia belongs to the Theater vault.

Onboarding should create/join a Room, select storage, connect an automatic source/destination, set up recovery and show first contributions. Joining members can use the existing Room storage. Current onboarding provides create/join/recovery then Room settings; the guided storage and recovery checklist is unfinished.

Media details put the photo/video first, followed by caption, date, People, conversation, audience and availability. Diagnostics expand separately. People filters support any/all and a visible clear action. Membership and face identification stay separate. Apple and Android offer explicit local face-crop review for already-saved Room photos; members choose the Person tag for the whole photo. Face crops are temporary and never name, group, or create a member automatically.

The **Ask your videos** assistant searches timestamped speech, on-screen words, visual suggestions, and Apple-only sound-event suggestions such as applause or laughter. Exact results are verbatim transcript or OCR evidence; classifier-derived scene and sound labels are always suggestions. A conservative local concept vocabulary helps match common paraphrases such as “audience applauding” with “crowd clapping,” without upgrading suggestions to exact results. Apple uses Vision, on-device Speech, and Sound Analysis; Android uses ML Kit and optional Vosk speech; Windows/Linux use FFmpeg/Tesseract with optional local whisper.cpp and loopback-only scene labeling. Results show their evidence and timestamp, then open the original moment. Words outside a quoted phrase act as optional context: the assistant can rank a repeated exact phrase using nearby audio or frame clues and show those clues separately. Only a matching transcript/OCR stream makes the wording exact; context never identifies a speaker. Indexes are private to each device. Local originals are indexed by default; remote-only originals require explicit opt-in and download. General embedding-based paraphrases, speaker identity, and long-running conversational memory remain unimplemented; explicit follow-up questions can reuse the immediately previous query locally. See the [video indexing design](VIDEO_SEARCH.md).

Automatic uploads require explicit source permission and destination selection. Newly accessible items publish after setup. Pause, exclusions and optional history are part of the contract. Weekly reels remain private drafts until published; unsaved generated drafts expire after fourteen days without deleting originals.

## Preservation and availability

Preserve originals, unknown formats and Live/Motion components; derive compatible previews. A cached preview, an accessible original and an independent verified backup are distinct. Status language should say “Saved on this device,” “Original available from cloud,” “Preview saved; waiting for Maya’s computer,” “Original has one verified copy,” or “Upload paused: storage full” only when supported by real state.

Quota is measured from accounts. Room storage preferences include budgets, preferred destinations and explicit overflow. Exhaustion preserves local originals. Transport supports verification, resume, cancellation and retries. Offline edits must converge while preserving caption conflicts and project versions.

## Privacy

Encrypt originals and metadata before transmission. Keep recognition embeddings/search indexes on devices; share only confirmed tags as encrypted metadata. Provide whole-Room and restricted-album access scopes, trusted-device pairing, recovery keys, revocation and new keys after removal. Previously downloaded media cannot be remotely erased. Gateways/providers receive ciphertext and never receive Room keys.

## Creative release

The requested release includes layered video/audio, detailed color controls, keyframes, mixing, voice-over, transitions, titles, music, beat matching, automatic sequencing, chapters/subtitles, undo/redo, autosave and recoverable versions. Editing is non-destructive. Start with one editor per shared project; preserve offline conflicts. All native clients must reopen the portable project and render equivalent output.

Watch parties, dedicated TV apps, physical DVD authoring, print/photo-book orders and legacy-contact workflows are later extensions. The digitized-home-video DVD viewing mode remains in the main Theater design.

The complete inventory and testable gates are in the [feature register](FEATURE_REGISTER.md). No milestone is declared complete until its acceptance criteria pass on the required platforms.
