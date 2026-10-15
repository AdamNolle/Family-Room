//! Device-local desktop video indexing using installed media tools.
//!
//! FFmpeg decodes short, evenly spaced frames; Tesseract reads visible text;
//! whisper.cpp transcribes speech only when a local model is configured.
//! No media or observations leave this process.

use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    env,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
use tempfile::TempDir;

const MAX_FRAMES: u64 = 300;
const MAX_VISUAL_LABEL_FRAMES: usize = 48;
const MAX_VIDEO_SECONDS: f64 = 7.0 * 24.0 * 60.0 * 60.0;
const AUDIO_CHUNK_SECONDS: u64 = 15 * 60;
const MAX_OBSERVATION_CHARS: usize = 1_024;
const MAX_VISION_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VideoObservation {
    pub modality: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub confidence_percent: u8,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VideoAnalysis {
    pub observations: Vec<VideoObservation>,
    pub audio_indexed: bool,
    pub frames_indexed: bool,
    pub visual_labels_indexed: bool,
    pub frame_interval_ms: u64,
    pub audio_message: String,
    pub visual_message: String,
}

#[derive(Debug)]
struct Toolchain {
    ffmpeg: OsStringPath,
    ffprobe: OsStringPath,
    tesseract: Option<OsStringPath>,
    whisper: Option<OsStringPath>,
    whisper_model: Option<PathBuf>,
    tesseract_language: String,
    vision_endpoint: Option<LocalVisionEndpoint>,
    vision_model: String,
    vision_message: String,
}

#[derive(Clone, Debug)]
struct LocalVisionEndpoint {
    authority: String,
    address: SocketAddr,
    path: String,
}

#[derive(Debug)]
struct OsStringPath(PathBuf);

impl LocalVisionEndpoint {
    fn parse(value: &str) -> Result<Self, String> {
        let rest = value
            .trim()
            .strip_prefix("http://")
            .ok_or_else(|| "The visual-labeling endpoint must use loopback HTTP.".to_owned())?;
        let (authority, path) = rest
            .split_once('/')
            .map(|(authority, path)| (authority, format!("/{path}")))
            .unwrap_or((rest, "/v1/chat/completions".to_owned()));
        if authority.is_empty()
            || authority.contains(['@', '?', '#'])
            || path.contains(['?', '#', '\\'])
            || path.chars().any(|character| character.is_ascii_control())
            || !path.starts_with('/')
        {
            return Err("The visual-labeling endpoint has an invalid URL.".into());
        }
        let (host, port) = if let Some(value) = authority.strip_prefix("[::1]:") {
            ("[::1]", value)
        } else {
            authority
                .rsplit_once(':')
                .ok_or_else(|| "The visual-labeling endpoint needs an explicit port.".to_owned())?
        };
        let port = port
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or_else(|| "The visual-labeling endpoint port is invalid.".to_owned())?;
        let address = match host {
            "127.0.0.1" | "localhost" => SocketAddr::from(([127, 0, 0, 1], port)),
            "[::1]" => SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port)),
            _ => {
                return Err(
                    "The visual-labeling endpoint must be on this device (localhost).".into(),
                )
            }
        };
        Ok(Self {
            authority: authority.to_owned(),
            address,
            path,
        })
    }

    fn label_image(&self, model: &str, image_path: &Path) -> Result<Vec<String>, String> {
        let image_size = std::fs::metadata(image_path)
            .map_err(|error| format!("Could not inspect a sampled frame: {error}"))?
            .len();
        if image_size > 8 * 1024 * 1024 {
            return Err("A sampled frame exceeded the local vision input limit.".into());
        }
        let image = std::fs::read(image_path).map_err(|error| {
            format!("Could not read a sampled frame for visual labeling: {error}")
        })?;
        let image_data = format!("data:image/jpeg;base64,{}", base64(&image));
        let request_body = serde_json::to_vec(&json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "Inspect only what is visibly present in this sampled video frame. Return JSON {\"labels\":[...]} with up to five concise, lower-case descriptions of clearly visible objects and directly observable actions or interactions that help someone find this moment (for example, 'child blowing candles on a cake' only when that is plainly visible). Do not transcribe visible text; OCR handles that. Do not infer anyone's identity, relationships, emotions, intent, or exact location. Leave uncertain details out, and never guess."},
                    {"type": "image_url", "image_url": {"url": image_data}}
                ]
            }],
            "temperature": 0,
            "max_tokens": 120,
            "stream": false
        }))
        .map_err(|error| format!("Could not encode the local visual-label request: {error}"))?;

        let mut stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(5))
            .map_err(|error| format!("Could not reach the local visual-labeling model: {error}"))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(120)))
            .map_err(|error| format!("Could not set the local model timeout: {error}"))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(|error| format!("Could not set the local model timeout: {error}"))?;
        write!(
            stream,
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.path,
            self.authority,
            request_body.len()
        )
        .and_then(|()| stream.write_all(&request_body))
        .map_err(|error| format!("Could not send the local visual-label request: {error}"))?;

        let mut response = Vec::new();
        stream
            .take((MAX_VISION_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut response)
            .map_err(|error| format!("Could not read the local visual-label response: {error}"))?;
        if response.len() > MAX_VISION_RESPONSE_BYTES {
            return Err("The local visual-label response exceeded its size limit.".into());
        }
        let response_body = http_response_body(&response)?;
        let response: Value = serde_json::from_slice(&response_body)
            .map_err(|_| "The local visual-label response was not valid JSON.".to_owned())?;
        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| "The local model did not return a text label response.".to_owned())?;
        parse_visual_labels(content)
    }
}

fn http_response_body(response: &[u8]) -> Result<Vec<u8>, String> {
    let boundary = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "The local visual-label response had invalid HTTP headers.".to_owned())?;
    let headers = std::str::from_utf8(&response[..boundary])
        .map_err(|_| "The local visual-label response had invalid HTTP headers.".to_owned())?;
    let mut lines = headers.lines();
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "The local visual-label response had no HTTP status.".to_owned())?;
    if !(200..300).contains(&status) {
        return Err(format!(
            "The local visual-label model returned HTTP {status}."
        ));
    }
    let body = &response[boundary + 4..];
    if headers
        .lines()
        .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"))
    {
        return decode_chunked(body);
    }
    if let Some(length) = headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    }) {
        if length > MAX_VISION_RESPONSE_BYTES || body.len() < length {
            return Err("The local visual-label response had an invalid body length.".into());
        }
        return Ok(body[..length].to_vec());
    }
    Ok(body.to_vec())
}

fn decode_chunked(input: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoded = Vec::new();
    let mut remaining = input;
    loop {
        let line_end = remaining
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| {
                "The local visual-label response had invalid chunk framing.".to_owned()
            })?;
        let size = std::str::from_utf8(&remaining[..line_end])
            .ok()
            .and_then(|line| line.split(';').next())
            .and_then(|size| usize::from_str_radix(size.trim(), 16).ok())
            .ok_or_else(|| {
                "The local visual-label response had invalid chunk framing.".to_owned()
            })?;
        remaining = &remaining[line_end + 2..];
        if size == 0 {
            return Ok(decoded);
        }
        if size > MAX_VISION_RESPONSE_BYTES.saturating_sub(decoded.len())
            || remaining.len() < size.saturating_add(2)
            || &remaining[size..size + 2] != b"\r\n"
        {
            return Err(
                "The local visual-label response exceeded its size limit or was truncated.".into(),
            );
        }
        decoded.extend_from_slice(&remaining[..size]);
        remaining = &remaining[size + 2..];
    }
}

fn base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or_default();
        let third = chunk.get(2).copied().unwrap_or_default();
        encoded.push(TABLE[(first >> 2) as usize] as char);
        encoded.push(TABLE[(((first & 0b11) << 4) | (second >> 4)) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            TABLE[(((second & 0b1111) << 2) | (third >> 6)) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            TABLE[(third & 0b11_1111) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

fn parse_visual_labels(response: &str) -> Result<Vec<String>, String> {
    let start = response
        .find('{')
        .ok_or_else(|| "The local model response did not contain label JSON.".to_owned())?;
    let end = response
        .rfind('}')
        .filter(|end| *end >= start)
        .ok_or_else(|| {
            "The local model response did not contain complete label JSON.".to_owned()
        })?;
    let value: Value = serde_json::from_str(&response[start..=end])
        .map_err(|_| "The local model response contained invalid label JSON.".to_owned())?;
    let labels = value["labels"]
        .as_array()
        .ok_or_else(|| "The local model response did not include a label list.".to_owned())?;
    let mut output = Vec::new();
    for label in labels.iter().filter_map(Value::as_str).take(5) {
        let label = compact_text(label).to_lowercase();
        if label.is_empty() || label.len() > 96 || output.contains(&label) {
            continue;
        }
        output.push(label);
    }
    Ok(output)
}

impl Toolchain {
    fn from_environment() -> Self {
        let ffmpeg =
            configured_path("FAMILY_ROOM_FFMPEG").unwrap_or_else(|| PathBuf::from("ffmpeg"));
        let ffprobe =
            configured_path("FAMILY_ROOM_FFPROBE").unwrap_or_else(|| PathBuf::from("ffprobe"));
        let tesseract = configured_path("FAMILY_ROOM_TESSERACT")
            .or_else(|| executable_on_path("tesseract").then(|| PathBuf::from("tesseract")));
        let whisper = configured_path("FAMILY_ROOM_WHISPER_CLI")
            .or_else(|| executable_on_path("whisper-cli").then(|| PathBuf::from("whisper-cli")));
        let whisper_model = env::var_os("FAMILY_ROOM_WHISPER_MODEL").map(PathBuf::from);
        let tesseract_language = env::var("FAMILY_ROOM_TESSERACT_LANG")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "eng".into());
        let vision_model = env::var("FAMILY_ROOM_VISION_MODEL_NAME")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "local-vision".into());
        let (vision_endpoint, vision_message) = match env::var("FAMILY_ROOM_VISION_URL") {
            Ok(value) => match LocalVisionEndpoint::parse(&value) {
                Ok(endpoint) => (
                    Some(endpoint),
                    "A loopback visual-labeling model is configured.".into(),
                ),
                Err(error) => (None, error),
            },
            Err(_) => (
                None,
                "Visual scene labels are off. Configure FAMILY_ROOM_VISION_URL with a local multimodal server to enable them.".into(),
            ),
        };
        Self {
            ffmpeg: OsStringPath(ffmpeg),
            ffprobe: OsStringPath(ffprobe),
            tesseract: tesseract.map(OsStringPath),
            whisper: whisper.map(OsStringPath),
            whisper_model,
            tesseract_language,
            vision_endpoint,
            vision_model,
            vision_message,
        }
    }

    fn ready_for_frames(&self) -> Result<bool, String> {
        verify_executable(&self.ffmpeg.0, "FFmpeg")?;
        verify_executable(&self.ffprobe.0, "FFprobe")?;
        if let Some(tesseract) = &self.tesseract {
            verify_executable(&tesseract.0, "Tesseract")?;
            Ok(true)
        } else if self.vision_endpoint.is_some() {
            Ok(false)
        } else {
            Err(
                "Frame indexing needs Tesseract or a configured local visual-labeling model."
                    .into(),
            )
        }
    }
}

/// Index one already-local video. `include_audio=false` never invokes the
/// speech recognizer, which keeps frame-only searches quick and predictable.
pub fn analyze_video(path: &Path, include_audio: bool) -> Result<VideoAnalysis, String> {
    analyze_video_scoped(path, include_audio, true)
}

pub fn analyze_video_scoped(
    path: &Path,
    include_audio: bool,
    include_frames: bool,
) -> Result<VideoAnalysis, String> {
    if !path.is_file() {
        return Err("The local video file is unavailable.".into());
    }
    if !include_audio && !include_frames {
        return Err("Choose speech, frame text, or both for local indexing.".into());
    }
    let tools = Toolchain::from_environment();
    let temp = TempDir::new()
        .map_err(|error| format!("Cannot create private index workspace: {error}"))?;
    let mut observations = Vec::new();
    let duration = video_duration(&tools, path)?;
    let interval_seconds = ((duration / MAX_FRAMES as f64).ceil() as u64).max(2);
    let frame_interval_ms = interval_seconds.saturating_mul(1_000);

    let (frame_observations, frames_indexed, visual_labels_indexed, visual_message) =
        if include_frames {
            match analyze_frames(&tools, path, temp.path(), duration, interval_seconds) {
                Ok(result) => result,
                Err(error) if !include_audio => return Err(error),
                Err(error) => (Vec::new(), false, false, error),
            }
        } else {
            (
                Vec::new(),
                false,
                false,
                "Frame analysis skipped for audio-only search.".into(),
            )
        };
    observations.extend(frame_observations);

    let (speech_observations, audio_indexed, audio_message) = if include_audio {
        analyze_audio(&tools, path, temp.path(), duration)
    } else {
        (
            Vec::new(),
            false,
            "Speech analysis was skipped for this frame-only search.".into(),
        )
    };
    observations.extend(speech_observations);
    observations.sort_by_key(|entry| (entry.start_ms, entry.modality.clone()));

    if !frames_indexed && !audio_indexed {
        return Err(if include_audio && include_frames {
            format!(
                "No local video index could be created. Frame analysis failed and {audio_message}"
            )
        } else if include_audio {
            audio_message
        } else {
            "No local frame index could be created.".into()
        });
    }

    Ok(VideoAnalysis {
        observations,
        audio_indexed,
        frames_indexed,
        visual_labels_indexed,
        frame_interval_ms,
        audio_message,
        visual_message,
    })
}

fn video_duration(tools: &Toolchain, path: &Path) -> Result<f64, String> {
    verify_executable(&tools.ffprobe.0, "FFprobe")?;
    let output = Command::new(&tools.ffprobe.0)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .map_err(|error| format!("Could not start FFprobe: {error}"))?;
    if !output.status.success() {
        return Err("FFprobe could not read this video's duration.".into());
    }
    let output = String::from_utf8_lossy(&output.stdout);
    let duration = output
        .trim()
        .parse::<f64>()
        .map_err(|_| "This video has no readable duration.")?;
    if !duration.is_finite() || duration <= 0.0 || duration > MAX_VIDEO_SECONDS {
        return Err("This video duration is outside the supported indexing range.".into());
    }
    Ok(duration)
}

fn analyze_frames(
    tools: &Toolchain,
    video: &Path,
    temp: &Path,
    duration: f64,
    interval_seconds: u64,
) -> Result<(Vec<VideoObservation>, bool, bool, String), String> {
    let run_ocr = tools.ready_for_frames()?;
    let frame_dir = temp.join("frames");
    std::fs::create_dir(&frame_dir).map_err(|error| error.to_string())?;
    let output_pattern = frame_dir.join("frame-%06d.jpg");
    let filter = format!(
        "fps=1/{interval_seconds},scale=w='min(960,iw)':h='min(960,ih)':force_original_aspect_ratio=decrease"
    );
    let status = Command::new(&tools.ffmpeg.0)
        .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(video)
        .args(["-vf", &filter, "-frames:v"])
        .arg(MAX_FRAMES.to_string())
        .args(["-q:v", "3", "-y"])
        .arg(&output_pattern)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| format!("Could not start FFmpeg: {error}"))?;
    if !status.success() {
        return Err("FFmpeg could not decode frames from this video.".into());
    }

    let mut frames = std::fs::read_dir(&frame_dir)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "jpg"))
        .collect::<Vec<_>>();
    frames.sort();
    let frame_count = frames.len().min(MAX_FRAMES as usize);
    frames.truncate(frame_count);
    let visual_sample_count = frame_count.min(MAX_VISUAL_LABEL_FRAMES);
    let visual_indices: HashSet<_> = if visual_sample_count == 0 {
        HashSet::new()
    } else {
        (0..visual_sample_count)
            .map(|sample| sample * frame_count / visual_sample_count)
            .collect()
    };
    let mut observations = Vec::new();
    let mut visual_error = None;
    for (index, frame) in frames.iter().enumerate() {
        let second = (index as u64).saturating_mul(interval_seconds);
        if second as f64 >= duration {
            break;
        }
        if run_ocr {
            let tesseract = tools
                .tesseract
                .as_ref()
                .ok_or_else(|| "Tesseract configuration was unavailable.".to_owned())?;
            let output = Command::new(&tesseract.0)
                .arg(frame)
                .arg("stdout")
                .args(["-l", &tools.tesseract_language, "--psm", "11", "tsv"])
                .output()
                .map_err(|error| format!("Could not start Tesseract: {error}"))?;
            if !output.status.success() {
                return Err("Tesseract could not read text from the decoded video frames.".into());
            }
            let (text, confidence_percent) =
                parse_tesseract_tsv(&String::from_utf8_lossy(&output.stdout));
            if !text.is_empty() {
                observations.push(VideoObservation {
                    modality: "screen_text".into(),
                    start_ms: second.saturating_mul(1_000),
                    end_ms: second.saturating_mul(1_000),
                    text,
                    confidence_percent,
                });
            }
        }

        if visual_indices.contains(&index) && visual_error.is_none() {
            if let Some(endpoint) = &tools.vision_endpoint {
                match endpoint.label_image(&tools.vision_model, frame) {
                    Ok(labels) if !labels.is_empty() => observations.push(VideoObservation {
                        modality: "visual_label".into(),
                        start_ms: second.saturating_mul(1_000),
                        end_ms: second.saturating_mul(1_000),
                        text: labels.join(" "),
                        confidence_percent: 55,
                    }),
                    Ok(_) => {}
                    Err(error) => visual_error = Some(error),
                }
            }
        }
    }
    let visual_labels_indexed = tools.vision_endpoint.is_some() && visual_error.is_none();
    let visual_message = match (&tools.vision_endpoint, visual_error) {
        (Some(_), Some(error)) => format!(
            "Local visual labels are incomplete ({error}); on-screen text remains available."
        ),
        (Some(_), None) => format!(
            "Local visual suggestions cover up to {visual_sample_count} sampled frames. Verify them in the original video."
        ),
        (None, _) => tools.vision_message.clone(),
    };
    let frames_indexed = run_ocr || visual_labels_indexed;
    if !frames_indexed {
        return Err(visual_message);
    }
    Ok((
        observations,
        frames_indexed,
        visual_labels_indexed,
        visual_message,
    ))
}

fn analyze_audio(
    tools: &Toolchain,
    video: &Path,
    temp: &Path,
    duration: f64,
) -> (Vec<VideoObservation>, bool, String) {
    let Some(whisper) = &tools.whisper else {
        return (
            Vec::new(),
            false,
            "Speech search needs whisper.cpp (`whisper-cli`) and a local model.".into(),
        );
    };
    let Some(model) = tools.whisper_model.as_ref().filter(|model| model.is_file()) else {
        return (
            Vec::new(),
            false,
            "Set FAMILY_ROOM_WHISPER_MODEL to a local whisper.cpp model file to index speech."
                .into(),
        );
    };
    if let Err(error) = verify_executable(&whisper.0, "whisper.cpp") {
        return (Vec::new(), false, error);
    }

    let mut all_observations = Vec::new();
    let duration_seconds = duration.ceil() as u64;
    let mut chunk_start = 0_u64;
    let mut chunk_number = 0_u64;
    while chunk_start < duration_seconds {
        let chunk_length = AUDIO_CHUNK_SECONDS.min(duration_seconds - chunk_start);
        let audio_path = temp.join(format!("audio-{chunk_number:04}.wav"));
        let extract = Command::new(&tools.ffmpeg.0)
            .args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
            .arg(video)
            .args(["-ss"])
            .arg(chunk_start.to_string())
            .args(["-t"])
            .arg(chunk_length.to_string())
            .args([
                "-map",
                "0:a:0",
                "-vn",
                "-ac",
                "1",
                "-ar",
                "16000",
                "-c:a",
                "pcm_s16le",
                "-y",
            ])
            .arg(&audio_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match extract {
            Ok(status) if status.success() => {}
            _ => {
                return (
                    Vec::new(),
                    false,
                    "This video has no extractable audio track, or audio extraction failed.".into(),
                )
            }
        }

        let prefix = temp.join(format!("transcript-{chunk_number:04}"));
        let output = Command::new(&whisper.0)
            .args(["-m"])
            .arg(model)
            .args(["-f"])
            .arg(&audio_path)
            .args(["-ovtt", "-of"])
            .arg(&prefix)
            .arg("-np")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match output {
            Ok(status) if status.success() => {
                let vtt = prefix.with_extension("vtt");
                let parsed = std::fs::read_to_string(&vtt)
                    .map_err(|error| error.to_string())
                    .and_then(|text| parse_webvtt(&text));
                match parsed {
                    Ok(mut observations) => {
                        offset_observations(&mut observations, chunk_start.saturating_mul(1_000));
                        if all_observations.len().saturating_add(observations.len()) > 15_000 {
                            return (Vec::new(), false, "The speech index reached its safety limit; frame search remains available.".into());
                        }
                        all_observations.extend(observations);
                    }
                    Err(_) => {
                        return (
                            Vec::new(),
                            false,
                            "whisper.cpp did not produce a readable timestamped transcript.".into(),
                        )
                    }
                }
                let _ = std::fs::remove_file(vtt);
            }
            _ => {
                return (
                    Vec::new(),
                    false,
                    "On-device speech transcription could not process this video.".into(),
                )
            }
        }
        let _ = std::fs::remove_file(audio_path);
        chunk_start = chunk_start.saturating_add(chunk_length);
        chunk_number = chunk_number.saturating_add(1);
    }
    (
        all_observations,
        true,
        "Speech was transcribed on this device.".into(),
    )
}

fn offset_observations(observations: &mut [VideoObservation], offset_ms: u64) {
    for observation in observations {
        observation.start_ms = observation.start_ms.saturating_add(offset_ms);
        observation.end_ms = observation.end_ms.saturating_add(offset_ms);
    }
}

fn compact_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_OBSERVATION_CHARS)
        .collect()
}

fn parse_tesseract_tsv(input: &str) -> (String, u8) {
    let mut words = Vec::new();
    let mut confidence_total = 0.0_f32;
    let mut confidence_count = 0_u32;
    for line in input.lines().skip(1) {
        let mut fields = line.splitn(12, '\t');
        for _ in 0..10 {
            if fields.next().is_none() {
                break;
            }
        }
        let Some(confidence) = fields.next().and_then(|value| value.parse::<f32>().ok()) else {
            continue;
        };
        let Some(word) = fields.next().map(str::trim).filter(|word| !word.is_empty()) else {
            continue;
        };
        if confidence < 0.0 || !confidence.is_finite() {
            continue;
        }
        words.push(word.to_owned());
        confidence_total += confidence;
        confidence_count += 1;
    }
    let text = compact_text(&words.join(" "));
    let confidence = if confidence_count == 0 {
        0
    } else {
        (confidence_total / confidence_count as f32)
            .round()
            .clamp(0.0, 100.0) as u8
    };
    (text, confidence)
}

/// Parse timestamped speech from a WebVTT transcript. Speaker labels are
/// removed; the index stores words and times, never inferred identities.
pub fn parse_webvtt(input: &str) -> Result<Vec<VideoObservation>, String> {
    let mut observations = Vec::new();
    let mut cue_start = None;
    let mut cue_end = None;
    let mut cue_text = Vec::new();
    let flush = |observations: &mut Vec<VideoObservation>,
                 start: Option<u64>,
                 end: Option<u64>,
                 text: &[String]| {
        let clean = compact_text(&text.join(" "));
        if let (Some(start_ms), Some(end_ms)) = (start, end) {
            if !clean.is_empty() && observations.len() < 15_000 {
                observations.push(VideoObservation {
                    modality: "speech".into(),
                    start_ms,
                    end_ms: end_ms.max(start_ms),
                    text: clean,
                    confidence_percent: 75,
                });
            }
        }
    };
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            flush(
                &mut observations,
                cue_start.take(),
                cue_end.take(),
                &cue_text,
            );
            cue_text.clear();
            continue;
        }
        if line == "WEBVTT"
            || line.starts_with("NOTE")
            || line.starts_with("STYLE")
            || line.starts_with("REGION")
        {
            continue;
        }
        if let Some((start, end)) = line.split_once(" --> ") {
            cue_start = Some(parse_timestamp(start)?);
            cue_end = Some(parse_timestamp(
                end.split_whitespace().next().unwrap_or(end),
            )?);
        } else if cue_start.is_some() {
            cue_text.push(strip_vtt_markup(line));
        }
    }
    flush(&mut observations, cue_start, cue_end, &cue_text);
    Ok(observations)
}

fn strip_vtt_markup(input: &str) -> String {
    let mut inside_tag = false;
    let mut clean = String::new();
    for character in input.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => clean.push(character),
            _ => {}
        }
    }
    clean.replace('&', " ").trim().into()
}

fn parse_timestamp(input: &str) -> Result<u64, String> {
    let parts = input.trim().split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err("Invalid WebVTT timestamp".into());
    }
    let seconds = parts
        .last()
        .unwrap()
        .parse::<f64>()
        .map_err(|_| "Invalid WebVTT timestamp")?;
    let minutes = parts[parts.len() - 2]
        .parse::<u64>()
        .map_err(|_| "Invalid WebVTT timestamp")?;
    let hours = if parts.len() == 3 {
        parts[0]
            .parse::<u64>()
            .map_err(|_| "Invalid WebVTT timestamp")?
    } else {
        0
    };
    if !seconds.is_finite() || !(0.0..60.0).contains(&seconds) || minutes >= 60 {
        return Err("Invalid WebVTT timestamp".into());
    }
    let total = hours
        .saturating_mul(3_600_000)
        .saturating_add(minutes.saturating_mul(60_000))
        .saturating_add((seconds * 1_000.0).round() as u64);
    Ok(total)
}

fn configured_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn executable_on_path(name: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| {
            let candidate = directory.join(name);
            candidate.is_file() || cfg!(windows) && directory.join(format!("{name}.exe")).is_file()
        })
    })
}

fn verify_executable(path: &Path, name: &str) -> Result<(), String> {
    let configured_as_path = path.components().count() > 1;
    if configured_as_path && !path.is_file() {
        return Err(format!(
            "{name} is configured but was not found at {}.",
            path.display()
        ));
    }
    if !configured_as_path && !executable_on_path(&path.to_string_lossy()) {
        return Err(format!(
            "{name} is not installed. Install it or set the matching FAMILY_ROOM_* path."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader},
        net::TcpListener,
        thread,
    };

    #[test]
    fn visual_model_endpoint_only_accepts_loopback_http() {
        let endpoint = LocalVisionEndpoint::parse("http://localhost:8080").unwrap();
        assert_eq!(endpoint.path, "/v1/chat/completions");
        assert!(LocalVisionEndpoint::parse("https://127.0.0.1:8080/v1/chat/completions").is_err());
        assert!(LocalVisionEndpoint::parse("http://192.168.1.5:8080/v1/chat/completions").is_err());
        assert!(
            LocalVisionEndpoint::parse("http://user@127.0.0.1:8080/v1/chat/completions").is_err()
        );
        assert!(LocalVisionEndpoint::parse("http://127.0.0.1:8080/\r\nHost: example.com").is_err());
    }

    #[test]
    fn visual_model_request_sends_a_local_frame_and_parses_bounded_labels() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let endpoint =
            LocalVisionEndpoint::parse(&format!("http://127.0.0.1:{port}/v1/chat/completions"))
                .unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut content_length = None;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                if let Some((name, value)) = line.trim_end().split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        content_length = value.trim().parse::<usize>().ok();
                    }
                }
            }
            let mut body = vec![0; content_length.unwrap()];
            reader.read_exact(&mut body).unwrap();
            let request: Value = serde_json::from_slice(&body).unwrap();
            let response_body = serde_json::to_vec(&json!({
                "choices": [{
                    "message": {"content": concat!("```json\n", r#"{"labels":["Red car","Driveway","red car"]}"#, "\n```")}
                }]
            }))
            .unwrap();
            write!(
                reader.get_mut(),
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                response_body.len()
            )
            .unwrap();
            reader.get_mut().write_all(&response_body).unwrap();
            request
        });
        let image = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(image.path(), b"frame-data").unwrap();

        let labels = endpoint.label_image("fixture-model", image.path()).unwrap();
        let request = server.join().unwrap();
        let content = request["messages"][0]["content"].as_array().unwrap();
        assert_eq!(request["model"], "fixture-model");
        let prompt = content[0]["text"].as_str().unwrap();
        assert!(prompt.contains("directly observable actions or interactions"));
        assert!(prompt.contains("Do not transcribe visible text"));
        assert!(prompt.contains("never guess"));
        assert_eq!(
            content[1]["image_url"]["url"],
            "data:image/jpeg;base64,ZnJhbWUtZGF0YQ=="
        );
        assert_eq!(labels, ["red car", "driveway"]);
    }

    #[test]
    fn chunked_local_model_response_is_decoded_with_a_bound() {
        assert_eq!(
            http_response_body(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n4\r\ntest\r\n0\r\n\r\n"
            )
            .unwrap(),
            b"test"
        );
        assert!(http_response_body(b"HTTP/1.1 500 Error\r\nContent-Length: 0\r\n\r\n").is_err());
        assert!(decode_chunked(b"100000\r\n").is_err());
    }

    #[test]
    fn webvtt_preserves_cue_times_and_removes_speaker_labels() {
        let result = parse_webvtt("WEBVTT\n\n00:00:01.200 --> 00:00:02.450\n<v Grandma>Look, the <b>birthday</b> cake!\n\n00:01:03.000 --> 00:01:04.100\nTurn around\n").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].start_ms, 1_200);
        assert_eq!(result[0].end_ms, 2_450);
        assert_eq!(result[0].text, "Look, the birthday cake!");
        assert_eq!(result[1].start_ms, 63_000);
        assert_eq!(result[0].modality, "speech");
    }

    #[test]
    fn malformed_webvtt_timestamp_fails_instead_of_inventing_evidence() {
        assert!(parse_webvtt("WEBVTT\n\nwrong --> 00:00:02.000\nhello").is_err());
    }

    #[test]
    fn rejects_an_empty_indexing_scope_before_starting_any_media_tool() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert_eq!(
            analyze_video_scoped(file.path(), false, false).unwrap_err(),
            "Choose speech, frame text, or both for local indexing."
        );
    }

    #[test]
    fn speech_chunk_offsets_keep_original_video_timestamps() {
        let mut observations =
            parse_webvtt("WEBVTT\n\n00:00:02.100 --> 00:00:03.000\nHappy birthday\n").unwrap();
        offset_observations(&mut observations, 900_000);
        assert_eq!(observations[0].start_ms, 902_100);
        assert_eq!(observations[0].end_ms, 903_000);
    }

    #[test]
    fn observation_text_is_bounded_and_whitespace_is_normalized() {
        let text = compact_text(&format!("hello  \n{}", "x".repeat(2_000)));
        assert!(text.len() <= MAX_OBSERVATION_CHARS);
        assert_eq!(compact_text(" birthday\n   party "), "birthday party");
    }

    #[test]
    fn tesseract_word_confidence_is_averaged_and_empty_rows_are_ignored() {
        let tsv = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n5\t1\t1\t1\t1\t1\t0\t0\t80\t20\t90\tHAPPY\n5\t1\t1\t1\t1\t2\t82\t0\t140\t20\t70\tBIRTHDAY\n5\t1\t1\t1\t1\t3\t0\t0\t0\t0\t-1\t\n";
        assert_eq!(parse_tesseract_tsv(tsv), ("HAPPY BIRTHDAY".into(), 80));
    }
}
