# Ask your videos: indexing and search

Ask your videos is a local, evidence-grounded finder for speech, visible words, sounds, and described scenes. A result includes the video, timestamp, evidence type, and the text or label that caused the match. Opening a result seeks the original to that moment. The assistant does not invent a transcript or claim who is speaking.

## How indexing works

Each device analyzes the selected video and writes timestamped text observations into that device’s encrypted catalog. Room sync never includes this index. A Room member builds their own index on their own device. Local videos are the default; indexing a remote-only original requires an explicit choice to fetch it to the device first. Clearing the local index leaves the original untouched.

| Evidence | Analysis path | Search guarantee |
|---|---|---|
| Speech | Apple on-device Speech; Android optional, checksum-verified English Vosk model; Windows/Linux optional local whisper.cpp model | Searches recognized words at transcript timestamps. Recognition may be wrong; no speaker identity is inferred. |
| Visible text | Apple Vision; Android ML Kit; Windows/Linux Tesseract on FFmpeg-sampled frames | Quoted phrases must appear in order in OCR text. OCR errors can affect a match. |
| Sound events | Apple Sound Analysis | Suggestions only, such as possible applause or laughter. Other clients do not yet index general sound events. |
| Scene clues | Apple Vision classification; Android ML Kit labels; optional loopback-only local vision model on Windows/Linux | Suggestions only. Descriptions are bounded to visible objects/actions and never identify a person or assert a relationship. |

On Windows/Linux, `family-video-indexer` uses FFmpeg/FFprobe to sample frames and extract audio, Tesseract for OCR, and optional local whisper.cpp and local vision services. It processes media on-device and returns bounded timestamped observations; the shared Rust core validates, encrypts, stores, and searches them. The helper is included in the native desktop build. Setup variables and tool requirements are in the [README](../README.md).

## End-to-end OCR verification

The opt-in regression `FAMILY_ROOM_FFMPEG=ffmpeg FAMILY_ROOM_FFPROBE=ffprobe FAMILY_ROOM_TESSERACT=tesseract cargo test -p family-core --all-features desktop_video_ocr_flows_through_encrypted_exact_search -- --ignored` runs the bundled `frame-birthday.mp4` through FFmpeg sampling and Tesseract, indexes those observations into the encrypted local catalog, performs a quoted frame search, then reopens the catalog and repeats the exact search. It is ignored by the default test suite because it requires the external desktop media tools. This verifies the desktop OCR-to-search path, not transcription accuracy or Windows/Linux native packaging.



The opt-in speech regression `FAMILY_ROOM_FFMPEG=ffmpeg FAMILY_ROOM_FFPROBE=ffprobe FAMILY_ROOM_WHISPER_CLI=whisper-cli FAMILY_ROOM_WHISPER_MODEL=/path/to/ggml-tiny.en.bin cargo test -p family-core --all-features desktop_video_speech_flows_through_encrypted_exact_search -- --ignored` runs the bundled spoken-video fixture through local audio extraction and whisper.cpp, stores timestamped transcript evidence in the encrypted catalog, searches a quoted spoken word, then reopens and repeats the search. The tiny English model is not bundled with the app; its upstream size and SHA-1 are listed in the [whisper.cpp model table](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md).
## Export spoken captions

The video detail screen can save or share a WebVTT sidecar generated from the current device's indexed speech. Cue times retain recognizer timestamps; visual labels, OCR, and sound-event guesses are excluded. Captions reflect speech-recognition output and should be checked against the original. Index speech first with Ask your videos. The sidecar is created only after an explicit export action and is not added to Room sync. Apple video detail playback, Android Theater playback, Linux GTK Theater playback, and Windows WinUI Theater playback display indexed speech cues as timed captions with hide controls. Windows C# compilation is unverified on this host; transcript editing remains future work.

## What “exact” means

An **Exact phrase** result means the ordered query words occur in one recognized speech transcript or OCR text stream. It means exact against the index, not necessarily exact against the original recording: speech recognition and OCR can make mistakes. The result always opens the original at the cited time so the member can check it.

Words outside quotation marks are retained as optional search context. In **All** mode, matching speech/OCR remains the exact requirement while nearby frame, speech, or sound observations can rank repeated phrases and appear separately as a context clue. Context is a ranking aid, not an AND requirement: an exact phrase still appears when no clue matches. A visual label by itself can never verify quoted wording, and a clue such as “Grandma” does not prove who is speaking. A sample query is `Find “Happy birthday” while blowing out candles`.

The searcher also uses a small, shared concept vocabulary to find common paraphrases. For example, “crowd clapping” can find an indexed scene described as “audience applauding,” and “clapping” can find a possible applause sound event. These are **Suggestions**, never exact phrases. One-word clues are supported; multiword suggestions need enough matching terms to avoid returning every loosely related clip. Quoted phrases disable synonym expansion and stay literal.

Speech transcripts and sound-event labels have separate coverage. General audio browsing can reuse sound-event indexing without repeatedly retrying unavailable transcription. A quoted request in the audio or all-evidence mode specifically retries missing speech transcription, even when sound clues are already indexed. Android explains and offers its optional offline English model before the question field; if it is not installed, the turn says speech was not searched and identifies any returned results as non-speech evidence.

This concept vocabulary is not a general semantic embedding model. It is intentionally transparent and limited: broad paraphrases such as “the kids having a great time” cannot be promised unless the local transcript or scene description contains matching concepts. Full local embedding retrieval, multilingual speech models, sound-event parity, confidence calibration, and tested accuracy on representative family libraries remain future work.

## Privacy and controls

- Video bytes and analysis stay on the current device. Desktop vision analysis is allowed only through an explicitly configured loopback endpoint.
- Search uses the encrypted local catalog and a bounded in-memory inverted index. The index is not sent to a cloud chatbot, provider, gateway, or other Room member.
- Remote-only originals are fetched only after opting in and remain subject to the connected source’s network policy.
- Search results are grounded in returned evidence and include modality and timestamps. Follow-up questions reuse only the immediately previous query on that device; quoted requests start a fresh search.

See [validation history](VALIDATION.md) for tests and the platform-specific limitations verified on this development host.
