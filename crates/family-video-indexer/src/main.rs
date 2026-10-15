use family_video_indexer::analyze_video_scoped;
use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let mut input = None::<PathBuf>;
    let mut include_audio = true;
    let mut include_frames = true;
    let mut args = env::args_os().skip(1);
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--input") => {
                input = args.next().map(PathBuf::from);
            }
            Some("--frame-only") => include_audio = false,
            Some("--audio-only") => include_frames = false,
            Some("--help" | "-h") => {
                return Ok("Usage: family-video-indexer --input VIDEO [--frame-only | --audio-only]\nUses local FFmpeg and Tesseract, optional whisper.cpp, and optional loopback vision via FAMILY_ROOM_VISION_URL.".into())
            }
            _ => return Err("Unknown video indexer argument.".into()),
        }
    }
    let input = input.ok_or("Expected --input VIDEO")?;
    let analysis = analyze_video_scoped(&input, include_audio, include_frames)?;
    serde_json::to_string(&analysis).map_err(|error| error.to_string())
}
