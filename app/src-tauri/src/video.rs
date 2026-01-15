use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub resolution: String,
    pub bitrate: String,
    pub encoder: String,
    pub status: String, // "Pending", "Processing", "Done", "Error"
    pub progress: u8,
}

#[derive(Serialize)]
pub struct ScanResult {
    pub videos: Vec<VideoInfo>,
    pub errors: Vec<String>,
}

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov", "flv", "wmv", "webm", "m4v"];

pub fn scan_videos(directory: &str) -> ScanResult {
    let mut videos = Vec::new();
    let mut errors = Vec::new();

    for entry in WalkDir::new(directory) {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                            videos.push(VideoInfo {
                                name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                                path: path.to_string_lossy().to_string(),
                                size,
                                resolution: "...".to_string(),
                                bitrate: "...".to_string(),
                                encoder: "...".to_string(),
                                status: "Scanning".to_string(),
                                progress: 0,
                            });
                        }
                    }
                }
            }
            Err(e) => errors.push(format!("Error walking directory: {}", e)),
        }
    }

    ScanResult { videos, errors }
}

pub fn get_metadata(path: &str, ffprobe_path: &str) -> Result<VideoInfo, String> {
    get_video_info(Path::new(path), ffprobe_path)
}

fn get_video_info(path: &Path, ffprobe_path: &str) -> Result<VideoInfo, String> {
    let output = Command::new(ffprobe_path)
        .args(&[
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            path.to_str().ok_or("Invalid path")?,
        ])
        .output()
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        return Err(format!("ffprobe exited with status: {:?}", output.status));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse ffprobe json: {}", e))?;

    let format = parsed.get("format").ok_or("No format info")?;
    let streams = parsed.get("streams").and_then(|v| v.as_array()).ok_or("No streams info")?;

    let size_bytes: u64 = format["size"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    
    // Find video stream
    let video_stream = streams.iter().find(|s| {
        s["codec_type"].as_str() == Some("video")
    }).ok_or("No video stream found")?;

    let width = video_stream["width"].as_u64().unwrap_or(0);
    let height = video_stream["height"].as_u64().unwrap_or(0);
    let resolution = format!("{}x{}", width, height);

    let bitrate = format["bit_rate"]
        .as_str()
        .map(|s| {
            let b: f64 = s.parse().unwrap_or(0.0);
            format!("{:.1} Mbps", b / 1_000_000.0)
        })
        .unwrap_or_else(|| "N/A".to_string());

    let codec = video_stream["codec_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    Ok(VideoInfo {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        path: path.to_string_lossy().to_string(),
        size: size_bytes,
        resolution,
        bitrate,
        encoder: codec,
        status: "Pending".to_string(),
        progress: 0,
    })
}
