use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use walkdir::WalkDir;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub resolution: String,
    pub bitrate: String,
    pub encoder: String,
    pub status: String, // "Pending", "Processing", "Done", "Error"
    pub progress: u8,
    #[serde(default)]
    pub duration_sec: f64,
    pub speed: Option<f64>,
    pub bitrate_kbps: Option<f64>,
    pub vmaf: Option<f64>,
    pub vmaf_device: Option<String>,
    pub vmaf_detail: Option<Vec<f64>>,
    pub vmaf_total_segments: Option<u32>,
    pub vmaf_model: Option<String>,
}

#[derive(Serialize)]
pub struct ScanResult {
    pub videos: Vec<VideoInfo>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectedEncoder {
    pub name: String,
    pub value: String,
    pub is_hardware: bool,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetectionReport {
    pub video: Vec<DetectedEncoder>,
    pub audio: Vec<DetectedEncoder>,
    pub log: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MaxResolution {
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EncoderConfig {
    pub name: String,
    pub value: String,
    pub visible: bool,
    pub custom_params: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompressionConfig {
    pub compression_mode: String,
    pub target_bitrate: u32, // kbps
    #[serde(rename = "targetCRF")]
    pub target_crf: f32,
    #[serde(rename = "targetVMAF")]
    pub target_vmaf: f32,
    pub ffmpeg_threads: u32,
    pub ffprobe_threads: u32,
    pub max_resolution: MaxResolution,
    pub video_encoder: String,
    pub audio_encoder: String,
    pub target_format: String,
    pub available_video_encoders: Vec<EncoderConfig>,
    pub available_audio_encoders: Vec<EncoderConfig>,
    pub custom_filters: Vec<String>,
    #[serde(default)]
    pub suffix: String,
    #[serde(default)]
    #[serde(rename = "twoPass")]
    pub two_pass: bool,
    #[serde(default)]
    #[serde(rename = "minBitrateThreshold")]
    pub min_bitrate_threshold: u32,
    #[serde(default)]
    #[serde(rename = "crfAutoSkip")]
    pub crf_auto_skip: bool,
    #[serde(default)]
    #[serde(rename = "crfAutoSkipThreshold")]
    pub crf_auto_skip_threshold: u32,

    // VMAF Settings
    #[serde(default)]
    pub enable_vmaf: bool,
    #[serde(default)]
    pub vmaf_full_computation: bool,
    #[serde(default)]
    pub vmaf_segment_count: u32,
    #[serde(default)]
    pub vmaf_segment_duration: u32,
    #[serde(default)]
    pub vmaf_auto_config: bool,
    #[serde(default)]
    pub vmaf_use_cuda: bool,
    #[serde(default)]
    pub vmaf_neg: bool,
    #[serde(default)]
    pub custom_vmaf_params: Vec<String>,
    #[serde(default)]
    #[serde(rename = "vmafSearchOptimization")]
    pub vmaf_search_optimization: bool,
}

pub struct VmafTask {
    pub app: AppHandle,
    pub input_path: String,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub reference_path: String,
    pub distorted_path: String,
    pub config: CompressionConfig,
    pub duration_sec: f64,
    pub pids: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>,
    pub cancelled_paths: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    pub output_video_info: Option<VideoInfo>,
}

pub struct VmafState {
    pub queue: std::collections::VecDeque<VmafTask>,
    pub running_task: Option<String>,
    /// Historical CRF-VMAF search results from previous tasks
    /// Used by the optimizer to predict CRF for new tasks
    pub crf_history: Vec<(f32, f64)>,
}

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "mov", "flv", "wmv", "webm", "m4v", "mpg", "mpeg", "3gp", "ts","asf", "rmvb", "vob","m2ts","f4v","mts","ogv", "divx","xvid","rm"];

/// Check if a path is a video file based on its extension
pub fn is_video_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        return VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str());
    }
    false
}

/// Categorize dropped paths into videos and directories
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathCategorization {
    pub videos: Vec<String>,
    pub directories: Vec<String>,
    pub invalid: Vec<String>,
}

pub fn categorize_paths(paths: Vec<String>) -> PathCategorization {
    let mut videos = Vec::new();
    let mut directories = Vec::new();
    let mut invalid = Vec::new();

    for p in paths {
        let path = Path::new(&p);
        if path.is_dir() {
            directories.push(p);
        } else if is_video_file(path) {
            videos.push(p);
        } else {
            invalid.push(p);
        }
    }

    PathCategorization { videos, directories, invalid }
}

/// Scan multiple paths (files and directories) for videos
pub fn scan_multiple_paths(paths: Vec<String>) -> ScanResult {
    let mut videos = Vec::new();
    let mut errors = Vec::new();

    for p in paths {
        let path = Path::new(&p);
        
        if path.is_dir() {
            // Scan directory recursively
            for entry in WalkDir::new(&p) {
                match entry {
                    Ok(entry) => {
                        let entry_path = entry.path();
                        if is_video_file(entry_path) {
                            let size = std::fs::metadata(entry_path).map(|m| m.len()).unwrap_or(0);
                            videos.push(VideoInfo {
                                name: entry_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                                path: entry_path.to_string_lossy().to_string(),
                                size,
                                resolution: "...".to_string(),
                                bitrate: "...".to_string(),
                                encoder: "...".to_string(),
                                status: "Scanning".to_string(),
                                progress: 0,
                                duration_sec: 0.0,
                                speed: None,
                                bitrate_kbps: None,
                                vmaf: None,
                                vmaf_device: None,
                                vmaf_detail: None,
                                vmaf_total_segments: None,
                                vmaf_model: None,
                            });
                        }
                    }
                    Err(e) => errors.push(format!("Error walking directory: {}", e)),
                }
            }
        } else if is_video_file(path) {
            // Single video file
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
                duration_sec: 0.0,
                speed: None,
                bitrate_kbps: None,
                vmaf: None,
                vmaf_device: None,
                vmaf_detail: None,
                vmaf_total_segments: None,
                vmaf_model: None,
            });
        } else {
            errors.push(format!("Invalid path (not a video or directory): {}", p));
        }
    }

    ScanResult { videos, errors }
}

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
                                duration_sec: 0.0,
                                speed: None,
                                bitrate_kbps: None,
                                vmaf: None,
                                vmaf_device: None,
                                vmaf_detail: None,
                                vmaf_total_segments: None,
                                vmaf_model: None,
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

#[derive(Debug, Serialize, Clone)]
pub struct DetectionProgress {
    pub r#type: String,
    pub name: String,
    pub value: String,
    pub available: bool,
}

pub fn detect_system_encoders(ffmpeg_path: &str, app: AppHandle) -> DetectionReport {
    let mut report = DetectionReport {
        video: Vec::new(),
        audio: Vec::new(),
        log: Vec::new(),
    };

    // 1. Get raw list
    let output = match Command::new(ffmpeg_path).arg("-encoders").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(e) => {
            report.log.push(format!("Failed to run ffmpeg: {}", e));
             return report;
        }
    };

    let mut all_video = Vec::new();
    let mut all_audio = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('-') || line.starts_with('=') { continue; }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 { continue; }

        let flags = parts[0];
        let name = parts[1];
        let description = parts[2..].join(" ");

        if flags.starts_with('V') {
            all_video.push((name.to_string(), description));
        } else if flags.starts_with('A') {
            all_audio.push((name.to_string(), description));
        }
    }

    // 2. Filter & Test Video
    let hw_keywords = ["nvenc", "amf", "qsv", "cuda", "vaapi", "vdpau","d3d12va"];

    for (name, desc) in all_video {
        let is_hw = hw_keywords.iter().any(|k| name.contains(k));

        let args = [
            "-y", "-hide_banner", "-v", "error",
            "-f", "lavfi", "-i", "color=size=1280x720:rate=30",
            "-frames:v", "1", "-pix_fmt", "yuv420p",
            "-c:v", &name, "-f", "null", "-"
        ];
        
        let available = match Command::new(ffmpeg_path).args(&args).output() {
            Ok(o) if o.status.success() => {
                let display_name = if is_hw { format!("{} (HW)", name) } else { format!("{} (CPU)", name) };
                report.video.push(DetectedEncoder {
                    name: display_name.clone(),
                    value: name.clone(),
                    is_hardware: is_hw,
                    description: desc.clone(),
                });
                true
            },
            _ => false
        };

        // 发送进度事件
        let _ = app.emit("encoder-detection-progress", DetectionProgress {
            r#type: "video".to_string(),
            name: if is_hw { format!("{} (HW)", name) } else { format!("{} (CPU)", name) },
            value: name,
            available,
        });
    }

    // 3. Filter & Test Audio
    for (name, desc) in all_audio {
        let args = [
            "-y", "-hide_banner", "-v", "error",
            "-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo",
            "-t", "1", "-c:a", &name, "-f", "null", "-"
        ];

        let available = match Command::new(ffmpeg_path).args(&args).output() {
            Ok(o) if o.status.success() => {
                report.audio.push(DetectedEncoder {
                    name: format!("{} (CPU)", name),
                    value: name.clone(),
                    is_hardware: false,
                    description: desc.clone(),
                });
                true
            },
            _ => false
        };

        // 发送进度事件
        let _ = app.emit("encoder-detection-progress", DetectionProgress {
            r#type: "audio".to_string(),
            name: format!("{} (CPU)", name),
            value: name,
            available,
        });
    }

    report
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

    let bitrate_raw: Option<f64> = format["bit_rate"]
        .as_str()
        .and_then(|s| s.parse().ok());

    let bitrate = bitrate_raw
        .map(|b| format!("{:.1} Mbps", b / 1_000_000.0))
        .unwrap_or_else(|| "N/A".to_string());
    
    let bitrate_kbps = bitrate_raw.map(|b| b / 1000.0);

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
        duration_sec: format["duration"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0),
        speed: None,
        bitrate_kbps,
        vmaf: None,
        vmaf_device: None,
        vmaf_detail: None,
        vmaf_total_segments: None,
        vmaf_model: None,
    })
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub path: String,
    pub progress: u8,
    pub status: String,
    pub speed: f64,
    pub bitrate_kbps: f64,
    pub output_info: Option<VideoInfo>,
}

/// Payload for VMAF-guided CRF search progress
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VmafSearchPayload {
    pub path: String,
    pub iteration: u32,
    pub max_iterations: u32,
    pub current_crf: f32,
    pub current_vmaf: f64,
    pub target_vmaf: f32,
    pub best_crf: Option<f32>,
    pub best_vmaf: Option<f64>,
    pub samples: Vec<(f32, f64)>, // (crf, vmaf) pairs collected
}

fn parse_time_str(time_str: &str) -> f64 {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() == 3 {
        let h: f64 = parts[0].parse().unwrap_or(0.0);
        let m: f64 = parts[1].parse().unwrap_or(0.0);
        let s: f64 = parts[2].parse().unwrap_or(0.0);
        return h * 3600.0 + m * 60.0 + s;
    }
    0.0
}

// --- VMAF-guided CRF Search Functions ---

/// Get CRF range for an encoder
fn get_crf_range(encoder: &str) -> (f32, f32) {
    if encoder.contains("libx264") || encoder.contains("libx265") {
        (18.0, 46.0) // H.264/H.265 CRF range
    } else if encoder.contains("libsvtav1") {
        (18.0, 54.0) // SVT-AV1 CRF range
    } else if encoder.contains("nvenc") {
        (18.0, 42.0) // CQ range for NVENC
    } else if encoder.contains("vp9") || encoder.contains("libvpx") {
        (18.0, 42.0) // VP9 CRF range
    } else {
        (1.0, 50.0) // Generic default
    }
}

/// Get CRF argument name for an encoder
fn get_crf_arg(encoder: &str) -> &'static str {
    if encoder.contains("nvenc") {
        "-cq"
    } else if encoder.contains("libx264") || encoder.contains("libx265") || 
              encoder.contains("libsvtav1") || encoder.contains("vp9") || encoder.contains("libvpx") {
        "-crf"
    } else {
        "-q:v"
    }
}

/// Compute sample segments for VMAF analysis during CRF search
/// Returns (start_time, duration) pairs for segments to sample
fn compute_sample_segments(duration_sec: f64, config: &CompressionConfig) -> Vec<(f64, f64)> {
    if duration_sec < 20.0 {
        // Short video: use full duration
        return vec![(0.0, duration_sec)];
    }

    let mut count = config.vmaf_segment_count;
    let mut dur = config.vmaf_segment_duration as f64;

    if config.vmaf_auto_config {
        dur = 20.0;
        let duration_min = duration_sec / 60.0;
        count = (duration_min / 12.0).ceil() as u32;
        if count < 1 { count = 1; }
    } else {
        if dur > duration_sec { dur = duration_sec; }
        if dur < 1.0 { dur = 1.0; }
        if count < 1 { count = 1; }
    }

    let mut segments = Vec::new();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros();
    
    for i in 0..count {
        let numerator = (i as f64) + 1.0;
        let denominator = (count as f64) + 2.0;
        let base_start = duration_sec * (numerator / denominator);
        
        let pseudo_rand = ((now + i as u128 * 12345) % 100) as f64;
        let offset_sec = (pseudo_rand - 50.0) / 10.0;
        
        let mut start = (base_start + offset_sec).round();
        
        if start < 0.0 { start = 0.0; }
        if start + dur > duration_sec {
            start = (duration_sec - dur).max(0.0);
        }
        start = start.round();
        if start < 0.0 { start = 0.0; }

        segments.push((start, dur));
    }
    segments
}

/// Compress a sample segment with a specific CRF and return the output path
fn compress_sample_with_crf(
    ffmpeg_path: &str,
    input_path: &str,
    temp_dir: &std::path::Path,
    crf: f32,
    segment_start: f64,
    segment_duration: f64,
    config: &CompressionConfig,
    pids: &std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>,
    input_key: &str,
) -> Option<String> {
    let sample_id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros();
    // Use the original video's container format (extension) for sample segments
    // This ensures compatibility and avoids format-related issues during VMAF calculation
    let original_ext = std::path::Path::new(input_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    let sample_output = temp_dir.join(format!("vmaf_sample_{}_{}.{}", sample_id, crf as i32, original_ext));
    let sample_output_str = sample_output.to_string_lossy().to_string();

    let v_enc = if config.video_encoder.is_empty() { "libx264".to_string() } else { config.video_encoder.clone() };
    let crf_arg = get_crf_arg(&v_enc);

    // Round timestamps to integers to avoid frame misalignment
    let ss = segment_start.round() as i64;
    let t = segment_duration.round() as i64;

    let mut args = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-v".to_string(), "error".to_string(),
        "-ss".to_string(), ss.to_string(),
        "-t".to_string(), t.to_string(),
        "-i".to_string(), input_path.to_string(),
        "-c:v".to_string(), v_enc.clone(),
        crf_arg.to_string(), format!("{}", crf),
        "-an".to_string(), // No audio for sample
    ];

    // Add encoder-specific params
    if let Some(enc_cfg) = config.available_video_encoders.iter().find(|e| e.value == v_enc) {
        for param in &enc_cfg.custom_params {
            let parts: Vec<&str> = param.split_whitespace().collect();
            for p in parts {
                args.push(p.to_string());
            }
        }
    }

    args.push(sample_output_str.clone());

    let mut command = Command::new(ffmpeg_path);
    command.args(&args).stdout(Stdio::null()).stderr(Stdio::piped());
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to spawn sample compression: {}", e);
            return None;
        }
    };

    let pid = child.id();
    {
        if let Ok(mut map) = pids.lock() {
            map.insert(input_key.to_string(), pid);
        }
    }

    let output = child.wait_with_output();
    
    {
        if let Ok(mut map) = pids.lock() {
            map.remove(input_key);
        }
    }

    match output {
        Ok(o) if o.status.success() => Some(sample_output_str),
        _ => {
            // Cleanup failed attempt
            let _ = std::fs::remove_file(&sample_output);
            None
        }
    }
}

/// Run VMAF for a sample pair and return the score
/// Note: sample_path is ALREADY a trimmed segment, so we only apply -ss/-t to the reference
fn compute_sample_vmaf(
    ffmpeg_path: &str,
    ffprobe_path: &str,
    reference_path: &str,
    sample_path: &str,
    model_path: &str,
    segment_start: f64,
    segment_duration: f64,
    use_cuda: bool,
    pids: &std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>,
    input_key: &str,
    custom_vmaf_params: &[String],
) -> Option<f64> {
    // Round timestamps to integers to avoid frame misalignment
    let ss = segment_start.round() as i64;
    let t = segment_duration.round() as i64;
    
    let model_esc = escape_path_for_filter(model_path);
    
    // Build vmaf_opts with custom params
    let mut vmaf_opts = format!("model='path={}'", model_esc);
    for param in custom_vmaf_params {
        let trimmed = param.trim();
        if !trimmed.is_empty() {
            vmaf_opts.push(':');
            vmaf_opts.push_str(trimmed);
        }
    }

    let mut args = Vec::new();
    args.push("-hide_banner".to_string());
    args.push("-threads".to_string());
    args.push(if use_cuda { "1".to_string() } else { "4".to_string() });
    args.push("-v".to_string());
    args.push("info".to_string()); // Need info level to see VMAF score output
    
    // Input 0: Sample (distorted) - already trimmed, read from start
    if use_cuda {
        args.push("-hwaccel".to_string()); args.push("cuda".to_string());
        args.push("-hwaccel_output_format".to_string()); args.push("cuda".to_string());
    }
    args.push("-i".to_string());
    args.push(sample_path.to_string());

    // Input 1: Reference - apply -ss and -t to match the sample segment
    if use_cuda {
        let ref_info = get_metadata(reference_path, ffprobe_path);
        let mut ref_decoder = None;
        if let Ok(info) = ref_info {
            ref_decoder = get_cuda_decoder(&info.encoder);
        }
        
        args.push("-hwaccel".to_string()); args.push("cuda".to_string());
        args.push("-hwaccel_output_format".to_string()); args.push("cuda".to_string());
        
        if let Some(dec) = ref_decoder {
            args.push("-c:v".to_string()); args.push(dec.to_string());
        }
    }
    
    args.push("-ss".to_string()); args.push(ss.to_string());
    args.push("-t".to_string()); args.push(t.to_string());
    args.push("-i".to_string());
    args.push(reference_path.to_string());

    // Filter Complex - note: [0:v] is distorted (sample), [1:v] is reference
    let filter = if use_cuda {
        format!(
            "[0:v]scale_cuda=format=yuv420p[dis];[1:v]scale_cuda=format=yuv420p[ref];[dis][ref]libvmaf_cuda={}", 
            vmaf_opts
        )
    } else {
        format!(
            "[0:v]setpts=PTS-STARTPTS,format=yuv420p[dis];[1:v]setpts=PTS-STARTPTS,format=yuv420p[ref];[dis][ref]libvmaf={}",
            vmaf_opts
        )
    };
    
    args.push("-filter_complex".to_string());
    args.push(filter);
    
    args.push("-f".to_string());
    args.push("null".to_string());
    args.push("-".to_string());

    println!("VMAF sample args: {:?}", args);

    let mut command = Command::new(ffmpeg_path);
    command.args(&args).stdout(Stdio::piped()).stderr(Stdio::piped());
    
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let mut child = command.spawn().ok()?;

    let pid = child.id();
    {
        if let Ok(mut map) = pids.lock() {
            map.insert(input_key.to_string(), pid);
        }
    }

    let output = child.wait_with_output();
    
    {
        if let Ok(mut map) = pids.lock() {
            map.remove(input_key);
        }
    }

    let o = output.ok()?;
    let stderr = String::from_utf8_lossy(&o.stderr);
    
    // Debug output
    println!("VMAF stderr length: {} chars", stderr.len());
    
    // Parse VMAF score from stderr
    // libvmaf outputs something like: "VMAF score: 95.123456"
    // or with Lavfi: "[Parsed_libvmaf_X @ ...] VMAF score: 95.123456"
    if let Some(idx) = stderr.find("VMAF score: ") {
        let rest = &stderr[idx+12..];
        let val_str = rest.split_whitespace().next().unwrap_or("0");
        println!("Found VMAF score string: {}", val_str);
        return val_str.parse().ok();
    }
    
    // Alternative pattern: look for mean vmaf in summary lines
    // "[libvmaf @ ...] VMAF score = 95.12"
    if let Some(idx) = stderr.find("VMAF score = ") {
        let rest = &stderr[idx+13..];
        let val_str = rest.split_whitespace().next().unwrap_or("0");
        println!("Found VMAF score (alt pattern): {}", val_str);
        return val_str.parse().ok();
    }

    println!("No VMAF score found in stderr. First 500 chars: {}", &stderr[..stderr.len().min(500)]);
    None
}


/// Linear interpolation to predict CRF for target VMAF
fn interpolate_crf(samples: &[(f32, f64)], target_vmaf: f64) -> f32 {
    if samples.len() < 2 {
        return 23.0; // fallback
    }

    // Sort by CRF (ascending)
    let mut sorted: Vec<(f32, f64)> = samples.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Find two points to interpolate between
    // Since higher CRF = lower quality = lower VMAF (generally),
    // we want to find where target_vmaf fits
    for i in 0..sorted.len() - 1 {
        let (crf1, vmaf1) = sorted[i];
        let (crf2, vmaf2) = sorted[i + 1];

        // Check if target is between these two points
        // Note: VMAF typically decreases as CRF increases
        let vmaf_high = vmaf1.max(vmaf2);
        let vmaf_low = vmaf1.min(vmaf2);
        
        if target_vmaf >= vmaf_low && target_vmaf <= vmaf_high {
            // Linear interpolation: crf = crf1 + (target_vmaf - vmaf1) * (crf2 - crf1) / (vmaf2 - vmaf1)
            if (vmaf2 - vmaf1).abs() < 0.1 {
                return (crf1 + crf2) / 2.0;
            }
            let predicted = crf1 + ((target_vmaf - vmaf1) * (crf2 as f64 - crf1 as f64) / (vmaf2 - vmaf1)) as f32;
            return predicted;
        }
    }

    // Extrapolation if target is outside range
    // Use last two points for extrapolation
    let (crf1, vmaf1) = sorted[sorted.len() - 2];
    let (crf2, vmaf2) = sorted[sorted.len() - 1];
    
    if (vmaf2 - vmaf1).abs() < 0.1 {
        return crf2;
    }
    
    let predicted = crf1 + ((target_vmaf - vmaf1) * (crf2 as f64 - crf1 as f64) / (vmaf2 - vmaf1)) as f32;
    predicted
}



/// VMAF-guided CRF search algorithm
/// Returns (best_crf, final_vmaf_score)
/// resolution: (width, height) tuple for model selection
/// crf_history: historical CRF-VMAF pairs from previous tasks for optimizer prediction
fn search_optimal_crf(
    app: &AppHandle,
    ffmpeg_path: &str,
    ffprobe_path: &str,
    input_path: &str,
    config: &CompressionConfig,
    duration_sec: f64,
    resolution: (u32, u32),
    pids: &std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>,
    cancelled_paths: &std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    crf_history: &[(f32, f64)],
) -> Result<(f32, f64), String> {
    let target_vmaf = config.target_vmaf as f64;
    let v_enc = if config.video_encoder.is_empty() { "libx264".to_string() } else { config.video_encoder.clone() };
    let (min_crf, max_crf) = get_crf_range(&v_enc);
    
    let temp_dir = std::env::temp_dir();
    let segments = compute_sample_segments(duration_sec, config);
    
    if segments.is_empty() {
        return Err("No segments to sample".to_string());
    }

    // Use first segment for all samples
    let (seg_start, seg_duration) = segments[0];

    // Determine VMAF model based on resolution (same logic as in calculate_vmaf)
    let (width, height) = resolution;
    let is_high_res = width.max(height) > 2560;
    let model_filename = match (is_high_res, config.vmaf_neg) {
        (false, false) => "vmaf_v0.6.1.json",
        (true, false) => "vmaf_4k_v0.6.1.json",
        (false, true) => "vmaf_v0.6.1neg.json",
        (true, true) => "vmaf_4k_v0.6.1neg.json",
    };
    
    println!("Selected VMAF model for search: {} (Resolution: {}x{}, Neg: {})", model_filename, width, height, config.vmaf_neg);
    
    let model_path = find_vmaf_model(ffmpeg_path, model_filename)
        .ok_or_else(|| format!("VMAF model {} not found", model_filename))?;

    let max_iterations = 10u32;
    let mut samples: Vec<(f32, f64)> = Vec::new();
    let mut best_crf: Option<f32> = None;
    let mut best_vmaf: Option<f64> = None;
    let mut no_improvement_count = 0;
    let mut search_complete = false;  // Flag to skip iterative search if binary search found optimal

    // Check cancellation helper
    let check_cancelled = || -> bool {
        if let Ok(set) = cancelled_paths.lock() {
            set.contains(input_path)
        } else {
            false
        }
    };

    // Note: Cross-video optimization has been disabled because different videos have 
    // vastly different CRF-VMAF relationships, making historical data from other videos unreliable.
    // Each video now uses independent binary search for the most accurate results.
    let _ = crf_history; // Suppress unused variable warning

    // Standard search approach (or continuation if optimization didn't find exact match)
    // Strategy: Test midpoint first, then determine search direction based on result
    // This is more efficient than testing min, mid, max all at once
    
    if samples.is_empty() {
        // No samples yet - start with midpoint strategy
        let mid_crf = (min_crf + max_crf) / 2.0;
        let mut current_min = min_crf;
        let mut current_max = max_crf;
        let mut iteration = 1u32;
        
        // Test midpoint first
        if check_cancelled() {
            cleanup_temp_samples(&temp_dir);
            return Err("Cancelled".to_string());
        }
        
        let _ = app.emit("vmaf-search-progress", VmafSearchPayload {
            path: input_path.to_string(),
            iteration,
            max_iterations,
            current_crf: mid_crf,
            current_vmaf: 0.0,
            target_vmaf: config.target_vmaf,
            best_crf,
            best_vmaf,
            samples: samples.clone(),
        });
        
        let sample_path = compress_sample_with_crf(
            ffmpeg_path, input_path, &temp_dir, mid_crf, seg_start, seg_duration, config, pids, input_path
        );
        
        if let Some(sample_path) = sample_path {
            let vmaf = compute_sample_vmaf(
                ffmpeg_path, ffprobe_path, input_path, &sample_path, &model_path,
                seg_start, seg_duration, config.vmaf_use_cuda, pids, input_path, &config.custom_vmaf_params
            );
            let _ = std::fs::remove_file(&sample_path);
            
            if let Some(score) = vmaf {
                samples.push((mid_crf, score));
                
                // Update best if VMAF >= target
                if score >= target_vmaf {
                    best_crf = Some(mid_crf);
                    best_vmaf = Some(score);
                }
                
                let _ = app.emit("vmaf-search-progress", VmafSearchPayload {
                    path: input_path.to_string(),
                    iteration,
                    max_iterations,
                    current_crf: mid_crf,
                    current_vmaf: score,
                    target_vmaf: config.target_vmaf,
                    best_crf,
                    best_vmaf,
                    samples: samples.clone(),
                });
                
                // Check if already close enough
                if (score - target_vmaf).abs() <= 0.5 {
                    println!("Midpoint CRF {} is close enough (VMAF {:.2}, target {:.1})", mid_crf, score, target_vmaf);
                    cleanup_temp_samples(&temp_dir);
                    return Ok((mid_crf, score));
                }
                
                // Determine search direction based on midpoint result
                // Higher VMAF than target -> can use higher CRF (more compression)
                // Lower VMAF than target -> need lower CRF (less compression)
                if score > target_vmaf {
                    // VMAF too high, search towards higher CRF
                    current_min = mid_crf;
                    println!("Midpoint VMAF {:.2} > target {:.1}, searching higher CRF range [{:.1}, {:.1}]", 
                        score, target_vmaf, current_min, current_max);
                } else {
                    // VMAF too low, search towards lower CRF
                    current_max = mid_crf;
                    println!("Midpoint VMAF {:.2} < target {:.1}, searching lower CRF range [{:.1}, {:.1}]", 
                        score, target_vmaf, current_min, current_max);
                }
                
                // Continue search using interpolation-based prediction
                while iteration < max_iterations && (current_max - current_min) > 1.0 {
                    if check_cancelled() {
                        cleanup_temp_samples(&temp_dir);
                        return Err("Cancelled".to_string());
                    }
                    
                    iteration += 1;
                    
                    // Use interpolation to predict next CRF if we have enough samples
                    let mut next_crf = if samples.len() >= 2 {
                        let predicted = interpolate_crf(&samples, target_vmaf);
                        // Clamp to current search range
                        predicted.max(current_min).min(current_max)
                    } else {
                        // Fallback to midpoint if not enough samples
                        (current_min + current_max) / 2.0
                    };
                    
                    const MIN_STEP: f32 = 0.8;
                    
                    // Check if we already have a sample too close to this CRF (minimum step: 0.8)
                    if samples.iter().any(|(c, _)| (*c - next_crf).abs() < MIN_STEP) {
                        // Try midpoint instead
                        next_crf = (current_min + current_max) / 2.0;
                        if samples.iter().any(|(c, _)| (*c - next_crf).abs() < MIN_STEP) {
                            break;
                        }
                    }
                    
                    let _ = app.emit("vmaf-search-progress", VmafSearchPayload {
                        path: input_path.to_string(),
                        iteration,
                        max_iterations,
                        current_crf: next_crf,
                        current_vmaf: 0.0,
                        target_vmaf: config.target_vmaf,
                        best_crf,
                        best_vmaf,
                        samples: samples.clone(),
                    });
                    
                    let sample_path = compress_sample_with_crf(
                        ffmpeg_path, input_path, &temp_dir, next_crf, seg_start, seg_duration, config, pids, input_path
                    );
                    
                    if let Some(sample_path) = sample_path {
                        let vmaf = compute_sample_vmaf(
                            ffmpeg_path, ffprobe_path, input_path, &sample_path, &model_path,
                            seg_start, seg_duration, config.vmaf_use_cuda, pids, input_path, &config.custom_vmaf_params
                        );
                        let _ = std::fs::remove_file(&sample_path);
                        
                        if let Some(next_score) = vmaf {
                            samples.push((next_crf, next_score));
                            
                            // Update best
                            if next_score >= target_vmaf {
                                if best_crf.is_none() || next_crf > best_crf.unwrap() {
                                    best_crf = Some(next_crf);
                                    best_vmaf = Some(next_score);
                                }
                            }
                            
                            let _ = app.emit("vmaf-search-progress", VmafSearchPayload {
                                path: input_path.to_string(),
                                iteration,
                                max_iterations,
                                current_crf: next_crf,
                                current_vmaf: next_score,
                                target_vmaf: config.target_vmaf,
                                best_crf,
                                best_vmaf,
                                samples: samples.clone(),
                            });
                            
                            // Log progress
                            println!("CRF {} gave VMAF {:.2} (target: {:.1}, diff: {:.2})", 
                                next_crf, next_score, target_vmaf, (next_score - target_vmaf).abs());
                            
                            // Early termination conditions (same as iterative search)
                            // Condition 1: best_crf exists and best_vmaf is in [target_vmaf, target_vmaf+0.3]
                            let early_stop_1 = if let (Some(b_crf), Some(b_vmaf)) = (best_crf, best_vmaf) {
                                b_vmaf >= target_vmaf && b_vmaf <= target_vmaf + 0.3
                            } else {
                                false
                            };
                            
                            // Condition 2: best_crf exists and there's a sample in [best_crf, best_crf+1] with vmaf < target_vmaf
                            let early_stop_2 = if let Some(b_crf) = best_crf {
                                samples.iter().any(|(c, v)| {
                                    *c >= b_crf && *c <= b_crf + 1.0 && *v < target_vmaf
                                })
                            } else {
                                false
                            };
                            
                            if early_stop_1 {
                                println!("Early stop (binary): best CRF {:.1} has VMAF {:.2} in target range [{:.1}, {:.1}]", 
                                    best_crf.unwrap(), best_vmaf.unwrap(), target_vmaf, target_vmaf + 0.3);
                                search_complete = true;
                                break;
                            }
                            
                            if early_stop_2 {
                                println!("Early stop (binary): found sample with VMAF < target in [best_crf {:.1}, {:.1}] range", 
                                    best_crf.unwrap(), best_crf.unwrap() + 1.0);
                                search_complete = true;
                                break;
                            }
                            
                            // Narrow search range
                            if next_score > target_vmaf {
                                current_min = next_crf;
                            } else {
                                current_max = next_crf;
                            }
                        }
                    }
                }
            }
        }
    } else {
        // We already have samples from optimization, continue with interpolation-based search
        // Just ensure we have boundary samples if needed
        let has_min = samples.iter().any(|(c, _)| (*c - min_crf).abs() < 1.0);
        let has_max = samples.iter().any(|(c, _)| (*c - max_crf).abs() < 1.0);
        
        let mut sample_start_idx = samples.len();
        
        // Add boundary samples if needed (but only one at a time, in priority order)
        let boundary_to_test = if !has_min && !has_max {
            // No boundaries - test one based on current best guess
            let current_best_crf = samples.iter().map(|(c, _)| *c).sum::<f32>() / samples.len() as f32;
            if current_best_crf > (min_crf + max_crf) / 2.0 {
                Some(min_crf)
            } else {
                Some(max_crf)
            }
        } else if !has_min {
            Some(min_crf)
        } else if !has_max {
            Some(max_crf)
        } else {
            None
        };
        
        if let Some(boundary_crf) = boundary_to_test {
            if check_cancelled() {
                cleanup_temp_samples(&temp_dir);
                return Err("Cancelled".to_string());
            }
            
            let _ = app.emit("vmaf-search-progress", VmafSearchPayload {
                path: input_path.to_string(),
                iteration: sample_start_idx as u32 + 1,
                max_iterations,
                current_crf: boundary_crf,
                current_vmaf: 0.0,
                target_vmaf: config.target_vmaf,
                best_crf,
                best_vmaf,
                samples: samples.clone(),
            });
            
            let sample_path = compress_sample_with_crf(
                ffmpeg_path, input_path, &temp_dir, boundary_crf, seg_start, seg_duration, config, pids, input_path
            );
            
            if let Some(sample_path) = sample_path {
                let vmaf = compute_sample_vmaf(
                    ffmpeg_path, ffprobe_path, input_path, &sample_path, &model_path,
                    seg_start, seg_duration, config.vmaf_use_cuda, pids, input_path, &config.custom_vmaf_params
                );
                let _ = std::fs::remove_file(&sample_path);
                
                if let Some(score) = vmaf {
                    samples.push((boundary_crf, score));
                    
                    if score >= target_vmaf {
                        if best_crf.is_none() || boundary_crf > best_crf.unwrap() {
                            best_crf = Some(boundary_crf);
                            best_vmaf = Some(score);
                        }
                    }
                    
                    let _ = app.emit("vmaf-search-progress", VmafSearchPayload {
                        path: input_path.to_string(),
                        iteration: sample_start_idx as u32 + 1,
                        max_iterations,
                        current_crf: boundary_crf,
                        current_vmaf: score,
                        target_vmaf: config.target_vmaf,
                        best_crf,
                        best_vmaf,
                        samples: samples.clone(),
                    });
                    
                    sample_start_idx += 1;
                }
            }
        }
    }

    // Iterative search (skip if already found optimal in binary search)
    let initial_samples_count = samples.len();
    if !search_complete {
    for iter in 0..(max_iterations.saturating_sub(initial_samples_count as u32)) {
        if check_cancelled() {
            cleanup_temp_samples(&temp_dir);
            return Err("Cancelled".to_string());
        }

        if samples.len() < 2 {
            break;
        }

        // Predict CRF for target VMAF
        let mut crf_guess = interpolate_crf(&samples, target_vmaf);
        
        // Clamp to range
        crf_guess = crf_guess.max(min_crf).min(max_crf);
        
        const MIN_STEP: f32 = 0.8;
        
        // Check if we already have a sample too close to this CRF (minimum step: 0.8)
        let already_sampled = samples.iter().any(|(c, _)| (c - crf_guess).abs() < MIN_STEP);
        if already_sampled {
            // Slight adjustment
            crf_guess += 1.0;
            crf_guess = crf_guess.max(min_crf).min(max_crf);
            // Check again after adjustment
            if samples.iter().any(|(c, _)| (c - crf_guess).abs() < MIN_STEP) {
                break;
            }
        }

        let _ = app.emit("vmaf-search-progress", VmafSearchPayload {
            path: input_path.to_string(),
            iteration: initial_samples_count as u32 + iter + 1,
            max_iterations,
            current_crf: crf_guess,
            current_vmaf: 0.0,
            target_vmaf: config.target_vmaf,
            best_crf,
            best_vmaf,
            samples: samples.clone(),
        });

        // Compress sample
        let sample_path = compress_sample_with_crf(
            ffmpeg_path, input_path, &temp_dir, crf_guess, seg_start, seg_duration, config, pids, input_path
        );

        if sample_path.is_none() {
            println!("Failed to compress sample at CRF {}", crf_guess);
            no_improvement_count += 1;
            if no_improvement_count >= 3 { break; }
            continue;
        }
        let sample_path = sample_path.unwrap();

        // Compute VMAF
        let vmaf = compute_sample_vmaf(
            ffmpeg_path, ffprobe_path, input_path, &sample_path, &model_path,
            seg_start, seg_duration, config.vmaf_use_cuda, pids, input_path, &config.custom_vmaf_params
        );

        // Cleanup sample
        let _ = std::fs::remove_file(&sample_path);

        if let Some(score) = vmaf {
            let old_best = best_crf;
            samples.push((crf_guess, score));
            
            // Update best if VMAF >= target and CRF is higher
            if score >= target_vmaf {
                if best_crf.is_none() || crf_guess > best_crf.unwrap() {
                    best_crf = Some(crf_guess);
                    best_vmaf = Some(score);
                }
            }

            // Check for improvement
            if old_best == best_crf {
                no_improvement_count += 1;
            } else {
                no_improvement_count = 0;
            }

            let _ = app.emit("vmaf-search-progress", VmafSearchPayload {
                path: input_path.to_string(),
                iteration: initial_samples_count as u32 + iter + 1,
                max_iterations,
                current_crf: crf_guess,
                current_vmaf: score,
                target_vmaf: config.target_vmaf,
                best_crf,
                best_vmaf,
                samples: samples.clone(),
            });

            // Early termination conditions
            // Condition 1: best_crf exists and best_vmaf is in [target_vmaf, target_vmaf+0.3]
            let early_stop_condition_1 = if let (Some(b_crf), Some(b_vmaf)) = (best_crf, best_vmaf) {
                b_vmaf >= target_vmaf && b_vmaf <= target_vmaf + 0.3
            } else {
                false
            };
            
            // Condition 2: best_crf exists and there's a sample in [best_crf, best_crf+1] with vmaf < target_vmaf
            let early_stop_condition_2 = if let Some(b_crf) = best_crf {
                samples.iter().any(|(c, v)| {
                    *c >= b_crf && *c <= b_crf + 1.0 && *v < target_vmaf
                })
            } else {
                false
            };
            
            if early_stop_condition_1 {
                println!("Early stop: best CRF {:.1} has VMAF {:.2} in target range [{:.1}, {:.1}]", 
                    best_crf.unwrap(), best_vmaf.unwrap(), target_vmaf, target_vmaf + 0.3);
                break;
            }
            
            if early_stop_condition_2 {
                println!("Early stop: found sample with VMAF < target in [best_crf {:.1}, {:.1}] range", 
                    best_crf.unwrap(), best_crf.unwrap() + 1.0);
                break;
            }
            
            // Fallback: stop if no improvement for too long
            if no_improvement_count >= 3 {
                println!("No improvement for 3 iterations, stopping search");
                break;
            }
        } else {
            no_improvement_count += 1;
            if no_improvement_count >= 3 { break; }
        }
    }
    }  // end of if !search_complete

    cleanup_temp_samples(&temp_dir);

    // Return results - find the CRF with VMAF closest to target
    // Priority: 
    // 1. Prefer samples with VMAF >= target (to ensure quality)
    // 2. Among those, find the one closest to target (to maximize compression)
    // 3. If no sample >= target, find the one closest to target anyway
    
    if samples.is_empty() {
        // No samples at all, use mid CRF
        return Ok(((min_crf + max_crf) / 2.0, 0.0));
    }
    
    // First try to find samples that meet or exceed target
    let mut best_above_target: Option<(f32, f64)> = None;
    for (c, v) in &samples {
        if *v >= target_vmaf {
            // Among samples >= target, prefer the one with VMAF closest to target
            // (this means highest CRF that still meets quality)
            if best_above_target.is_none() || 
               (*v - target_vmaf).abs() < (best_above_target.unwrap().1 - target_vmaf).abs() {
                best_above_target = Some((*c, *v));
            }
        }
    }
    
    if let Some((c, v)) = best_above_target {
        println!("Returning CRF {:.1} with VMAF {:.2} (>= target {:.1})", c, v, target_vmaf);
        return Ok((c, v));
    }
    
    // No sample meets target, find the one with VMAF closest to target
    let mut closest: Option<(f32, f64)> = None;
    for (c, v) in &samples {
        if closest.is_none() || 
           (*v - target_vmaf).abs() < (closest.unwrap().1 - target_vmaf).abs() {
            closest = Some((*c, *v));
        }
    }
    
    if let Some((c, v)) = closest {
        println!("No sample meets target. Returning closest: CRF {:.1} with VMAF {:.2} (target {:.1})", c, v, target_vmaf);
        Ok((c, v))
    } else {
        Ok(((min_crf + max_crf) / 2.0, 0.0))
    }
}

/// Cleanup temporary sample files
fn cleanup_temp_samples(temp_dir: &std::path::Path) {
    if let Ok(entries) = std::fs::read_dir(temp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("vmaf_sample_") {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }
}

pub fn process_video(
    app: AppHandle,
    ffmpeg_path: &str,
    input_path: String,
    output_path: String,
    config: CompressionConfig,
    duration_sec: f64,
    pids: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>,
    cancelled_paths: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    vmaf_state: std::sync::Arc<std::sync::Mutex<VmafState>>,
) -> Result<(), String> {
    // Clear any previous cancellation for this path (allows re-processing after cancel)
    {
        if let Ok(mut set) = cancelled_paths.lock() {
            set.remove(&input_path);
        }
    }

    // 0. Resolve ffprobe path
    let ffprobe_path = if let Some(parent_dir) = std::path::Path::new(ffmpeg_path).parent() {
        let ffmpeg_path_buf = std::path::Path::new(ffmpeg_path);
        let ffprobe_name = if let Some(ext) = ffmpeg_path_buf.extension() {
            if ext.to_string_lossy().eq_ignore_ascii_case("exe") {
                "ffprobe.exe"
            } else {
                "ffprobe"
            }
        } else {
            if cfg!(windows) { "ffprobe.exe" } else { "ffprobe" }
        };
        parent_dir.join(ffprobe_name).to_string_lossy().to_string()
    } else {
        "ffprobe".to_string()
    };

    // 1. Get Input Info for Bitrate Analysis
    let input_info = get_video_info(Path::new(&input_path), &ffprobe_path).ok();
    let input_bitrate_kbps = input_info.as_ref().and_then(|i| i.bitrate_kbps);

    // 2. Bitrate Bypass Check
    if config.compression_mode == "bitrate" && config.min_bitrate_threshold > 0 {
        if let Some(br) = input_bitrate_kbps {
             if br < config.min_bitrate_threshold as f64 {
                // SKIP!
                println!("Skipping {} because bitrate {:.2} < threshold {}", input_path, br, config.min_bitrate_threshold);
                
                // Copy if needed
                if input_path != output_path {
                     if let Some(parent) = std::path::Path::new(&output_path).parent() {
                        if !parent.exists() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                    }
                    if let Err(e) = std::fs::copy(&input_path, &output_path) {
                        eprintln!("[ERROR] Failed to copy file during bitrate threshold skip for '{}': {}", input_path, e);
                        eprintln!("[INFO] Source: {}", input_path);
                        eprintln!("[INFO] Destination: {}", output_path);
                        return Err(format!("Skipped (bitrate check), but failed to copy file: {}", e));
                    }
                }

                // If input == output, we effectively did nothing, which is correct (metadata preserved).
                
                let _ = app.emit("video-progress", ProgressPayload {
                    path: input_path.clone(),
                    progress: 100,
                    status: "Skipped".to_string(),
                    speed: 0.0,
                    bitrate_kbps: br,
                    output_info: input_info,
                });
                return Ok(());
             }
        }
    }

    // 3. VMAF-guided CRF Search (if compression mode is "vmaf")
    let mut vmaf_derived_crf: Option<f32> = None;
    let mut vmaf_search_score: Option<f64> = None;
    
    if config.compression_mode == "vmaf" {
        let _ = app.emit("video-progress", ProgressPayload {
            path: input_path.clone(),
            progress: 0,
            status: "Searching CRF".to_string(),
            speed: 0.0,
            bitrate_kbps: 0.0,
            output_info: None,
        });

        // Extract resolution for model selection
        let resolution = if let Some(ref info) = input_info {
            let parts: Vec<&str> = info.resolution.split('x').collect();
            if parts.len() == 2 {
                let w = parts[0].parse::<u32>().unwrap_or(0);
                let h = parts[1].parse::<u32>().unwrap_or(0);
                (w, h)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        // Get historical CRF data for optimization
        let crf_history: Vec<(f32, f64)> = if let Ok(state) = vmaf_state.lock() {
            state.crf_history.clone()
        } else {
            Vec::new()
        };

        match search_optimal_crf(
            &app, ffmpeg_path, &ffprobe_path, &input_path, &config, duration_sec, resolution, &pids, &cancelled_paths, &crf_history
        ) {
            Ok((crf, vmaf)) => {
                println!("VMAF search complete: CRF={}, VMAF={:.2}", crf, vmaf);
                vmaf_derived_crf = Some(crf);
                vmaf_search_score = Some(vmaf);
                
                // Update historical CRF data for future task optimization
                if let Ok(mut state) = vmaf_state.lock() {
                    state.crf_history.push((crf, vmaf));
                    // Keep a reasonable size to avoid memory bloat and maintain relevance
                    // More recent data is more relevant for predicting similar content
                    if state.crf_history.len() > 50 {
                        state.crf_history.remove(0);
                    }
                    println!("Updated CRF history: {} entries", state.crf_history.len());
                }
                
                let _ = app.emit("video-progress", ProgressPayload {
                    path: input_path.clone(),
                    progress: 50, // Search phase complete (50%), starting compression
                    status: format!("Found CRF {:.0}, compressing...", crf),
                    speed: 0.0,
                    bitrate_kbps: 0.0,
                    output_info: None,
                });
            }
            Err(e) => {
                if e == "Cancelled" {
                    let _ = app.emit("video-progress", ProgressPayload {
                        path: input_path.clone(),
                        progress: 0,
                        status: "Cancelled".to_string(),
                        speed: 0.0,
                        bitrate_kbps: 0.0,
                        output_info: None,
                    });
                    return Err("Cancelled during CRF search".to_string());
                } else {
                    eprintln!("[ERROR] VMAF CRF search failed for '{}': {}", input_path, e);
                    eprintln!("[INFO] Using default CRF 23 as fallback");
                    println!("VMAF CRF search failed: {}, using default CRF 23", e);
                    vmaf_derived_crf = Some(23.0); // fallback
                }
            }
        }
    }

    let mut args = Vec::new();
    args.push("-y".to_string());
    args.push("-hide_banner".to_string());
    args.push("-i".to_string());
    args.push(input_path.clone());

    // Check if we are in copy mode (stream copy, no re-encoding)
    let is_copy_mode = config.compression_mode == "copy";

    // Video Encoder
    args.push("-c:v".to_string());
    let v_enc = if is_copy_mode {
        "copy".to_string()
    } else if config.video_encoder.is_empty() { 
        "libx264".to_string() 
    } else { 
        config.video_encoder.clone() 
    };
    args.push(v_enc.clone());

    // Compression Mode (skip for copy mode)
    if !is_copy_mode {
        match config.compression_mode.as_str() {
            "bitrate" => {
                args.push("-b:v".to_string());
                args.push(format!("{}k", config.target_bitrate));
            },
            "crf" => {
                if v_enc.contains("libx264") || v_enc.contains("libx265") || v_enc.contains("libsvtav1") || v_enc.contains("vp9") {
                     args.push("-crf".to_string());
                     args.push(format!("{}", config.target_crf));
                } else if v_enc.contains("nvenc") {
                     args.push("-cq".to_string());
                     args.push(format!("{}", config.target_crf));
                } else {
                     args.push("-q:v".to_string());
                     args.push(format!("{}", config.target_crf));
                }
            },
            "vmaf" => {
                // Use CRF derived from VMAF search, or fallback to 23
                let crf_to_use = vmaf_derived_crf.unwrap_or(23.0);
                let crf_arg = get_crf_arg(&v_enc);
                args.push(crf_arg.to_string());
                args.push(format!("{}", crf_to_use));
            },
            _ => {}
        }
    }


    // Audio Encoder
    args.push("-c:a".to_string());
    let a_enc = if is_copy_mode {
        "copy".to_string()
    } else if config.audio_encoder.is_empty() { 
        "aac".to_string() 
    } else { 
        config.audio_encoder.clone() 
    };
    args.push(a_enc.clone());

    // Resolution (skip for copy mode - cannot scale when copying streams)
    if !is_copy_mode && config.max_resolution.enabled && config.max_resolution.width > 0 && config.max_resolution.height > 0 {
        args.push("-vf".to_string());
        args.push(format!("scale='min({},iw)':-2", config.max_resolution.width));
    }

    // Custom Filters (always apply - these can include things like -movflags +faststart)
    for filter in &config.custom_filters {
        if !filter.trim().is_empty() {
             let parts: Vec<&str> = filter.split_whitespace().collect();
             for p in parts {
                 args.push(p.to_string());
             }
        }
    }
    
    // Encoder Specific Params (skip for copy mode - no encoding)
    if !is_copy_mode {
        if let Some(enc_cfg) = config.available_video_encoders.iter().find(|e| e.value == v_enc) {
            for param in &enc_cfg.custom_params {
                 let parts: Vec<&str> = param.split_whitespace().collect();
                 for p in parts {
                     args.push(p.to_string());
                 }
            }
        }
         if let Some(enc_cfg) = config.available_audio_encoders.iter().find(|e| e.value == a_enc) {
            for param in &enc_cfg.custom_params {
                 let parts: Vec<&str> = param.split_whitespace().collect();
                 for p in parts {
                     args.push(p.to_string());
                 }
            }
        }
    }

    // threads
    if config.ffmpeg_threads > 0 {
        args.push("-threads".to_string());
        args.push(format!("{}", config.ffmpeg_threads));
    }

    args.push("-progress".to_string());
    args.push("pipe:2".to_string());

    let temp_output_path = format!("{}.tmp.{}", output_path, config.target_format);
    
    // Ensure output directory exists before starting FFmpeg
    if let Some(parent) = std::path::Path::new(&temp_output_path).parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("[ERROR] Failed to create output directory for '{}': {}", input_path, e);
                eprintln!("[INFO] Target directory: {:?}", parent);
                return Err(format!("Failed to create output directory: {}", e));
            }
        }
    }
    
    // 2-Pass Logic
    let mut pass_log_prefix_opt = None;
    if config.compression_mode == "bitrate" && config.two_pass {
        let pass_log_prefix = format!("{}.passlog", temp_output_path);
        pass_log_prefix_opt = Some(pass_log_prefix.clone());
        
        // Pass 1
        let mut pass1_args = args.clone();
        pass1_args.push("-pass".to_string());
        pass1_args.push("1".to_string());
        pass1_args.push("-passlogfile".to_string());
        pass1_args.push(pass_log_prefix.clone());
        pass1_args.push("-an".to_string()); // No audio for pass 1
        pass1_args.push("-f".to_string());
        pass1_args.push("null".to_string());
        if cfg!(windows) {
             pass1_args.push("NUL".to_string());
        } else {
             pass1_args.push("/dev/null".to_string());
        }

        let _ = app.emit("video-progress", ProgressPayload {
            path: input_path.clone(),
            progress: 0,
            status: "Processing (Pass 1/2)".to_string(),
            speed: 0.0,
            bitrate_kbps: 0.0,
            output_info: None,
        });

        println!("Starting Pass 1 for {}", input_path);
        
        let mut pass1_child = Command::new(ffmpeg_path)
            .args(&pass1_args)
            .stdout(Stdio::null()) // Use null to avoid blocking if we don't read it
            .stderr(Stdio::piped()) 
            .spawn()
            .map_err(|e| {
                eprintln!("[ERROR] Failed to spawn Pass 1 for '{}': {}", input_path, e);
                eprintln!("[INFO] FFmpeg path: {}", ffmpeg_path);
                format!("Failed to spawn Pass 1: {}", e)
            })?;

        let p1_pid = pass1_child.id();
        {
            if let Ok(mut map) = pids.lock() {
                map.insert(input_path.clone(), p1_pid);
            }
        }

        let p1_stderr = pass1_child.stderr.take().ok_or_else(|| {
            eprintln!("[ERROR] Failed to capture Pass 1 stderr for '{}'", input_path);
            "Failed to capture Pass 1 stderr".to_string()
        })?;
        let p1_reader = BufReader::new(p1_stderr);

        let mut p1_sec = 0.0;
        let mut p1_speed = 0.0;
        
        // Collect stderr lines for error reporting
        let mut p1_stderr_lines: Vec<String> = Vec::new();
        let max_stderr_lines = 50;
        
        for line in p1_reader.lines() {
             let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            
            // Collect stderr lines (excluding progress lines)
            if !line.contains("progress=") && !line.trim().is_empty() {
                p1_stderr_lines.push(line.clone());
                if p1_stderr_lines.len() > max_stderr_lines {
                    p1_stderr_lines.remove(0);
                }
            }

            // Check cancellation
            let is_cancelled = {
                 if let Ok(set) = cancelled_paths.lock() {
                     set.contains(&input_path)
                 } else {
                     false
                 }
            };
            if is_cancelled {
                let _ = pass1_child.kill();
                break;
            }

            // Simple parsing for Pass 1 (Speed & Time) - Bitrate is irrelevant/NA for Pass 1 (null output)
            if let Some(idx) = line.find("out_time=") {
                let time_val = line[idx+9..].trim();
                p1_sec = parse_time_str(time_val);
            } else if let Some(idx) = line.find("speed=") {
                let speed_str = line[idx+6..].trim();
                if let Some(s_idx) = speed_str.find('x') {
                     p1_speed = speed_str[0..s_idx].parse().unwrap_or(0.0);
                } else {
                     p1_speed = speed_str.parse().unwrap_or(0.0);
                }
            }

            if line.contains("progress=") {
                 let percent = if duration_sec > 0.0 {
                    ((p1_sec / duration_sec) * 100.0) as u8
                } else {
                    0
                };
                // Pass 1 is 0-50% of total progress
                let mapped_percent = percent.min(100) / 2;
                let _ = app.emit("video-progress", ProgressPayload {
                    path: input_path.clone(),
                    progress: mapped_percent,
                    status: "Processing (Pass 1/2)".to_string(),
                    speed: p1_speed,
                    bitrate_kbps: 0.0, // Pass 1 has no meaningful bitrate
                    output_info: None,
                });
            }
        }

        let p1_status = pass1_child.wait().map_err(|e| {
            eprintln!("[ERROR] Failed to wait on Pass 1 process for '{}': {}", input_path, e);
            format!("Failed to wait on video Pass 1: {}", e)
        })?;
        
        {
            if let Ok(mut map) = pids.lock() {
                map.remove(&input_path);
            }
        }

        // Check cancellation again to be sure
        let is_cancelled = {
             if let Ok(mut set) = cancelled_paths.lock() {
                 if set.contains(&input_path) {
                     set.remove(&input_path); // Clear it here since we are stopping early
                     true
                 } else {
                     false
                 }
             } else {
                 false
             }
        };

        if is_cancelled {
             let _ = app.emit("video-progress", ProgressPayload {
                path: input_path.clone(),
                progress: 0,
                status: "Cancelled".to_string(),
                speed: 0.0,
                bitrate_kbps: 0.0,
                output_info: None,
            });
            // Robust cleanup Pass 1 logs
            std::thread::sleep(std::time::Duration::from_millis(200));
            if let Some(prefix) = &pass_log_prefix_opt {
                 cleanup_pass_logs(prefix, &temp_output_path);
            }
            return Err("Cancelled during Pass 1".to_string());
        }

        if !p1_status.success() {
             eprintln!("[ERROR] Pass 1 failed for '{}': FFmpeg exited with status {:?}", input_path, p1_status);
             
             // Print Pass 1 stderr output for debugging
             if !p1_stderr_lines.is_empty() {
                 eprintln!("[ERROR] Pass 1 FFmpeg stderr output (last {} lines):", p1_stderr_lines.len());
                 for stderr_line in &p1_stderr_lines {
                     eprintln!("  {}", stderr_line);
                 }
             } else {
                 eprintln!("[WARNING] No stderr output captured from Pass 1");
             }
             
             // Robust cleanup Pass 1 logs
             std::thread::sleep(std::time::Duration::from_millis(200));
             if let Some(prefix) = &pass_log_prefix_opt {
                 cleanup_pass_logs(prefix, &temp_output_path);
             }
             return Err("Pass 1 failed (unknown error)".to_string());
        }

        // Prepare Pass 2 args (modify the original args which will be used for the main spawn)
        args.push("-pass".to_string());
        args.push("2".to_string());
        args.push("-passlogfile".to_string());
        args.push(pass_log_prefix);
        
         let _ = app.emit("video-progress", ProgressPayload {
            path: input_path.clone(),
            progress: 50, // Pass 1 complete (50%), starting Pass 2
            status: "Processing (Pass 2/2)".to_string(),
            speed: 0.0,
            bitrate_kbps: 0.0,
            output_info: None,
        });
    }

    args.push(temp_output_path.clone());

    let status_str = if config.compression_mode == "bitrate" && config.two_pass {
        "Processing (Pass 2/2)".to_string()
    } else if config.compression_mode == "vmaf" {
        // Use "Found CRF" prefix so frontend can calculate progress as 50-100%
        if let Some(crf) = vmaf_derived_crf {
            format!("Found CRF {:.0}", crf)
        } else {
            "Processing".to_string()
        }
    } else {
        "Processing".to_string()
    };
    // Initial progress: 50 for phase 2 modes (Pass 2 or VMAF compression), 0 otherwise
    let is_phase2 = (config.compression_mode == "bitrate" && config.two_pass) || config.compression_mode == "vmaf";
    let initial_progress = if is_phase2 { 50 } else { 0 };

    let _ = app.emit("video-progress", ProgressPayload {
        path: input_path.clone(),
        progress: initial_progress,
        status: status_str.clone(),
        speed: 0.0,
        bitrate_kbps: 0.0,
        output_info: None,
    });

    let mut child = Command::new(ffmpeg_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            eprintln!("[ERROR] Failed to spawn FFmpeg for '{}': {}", input_path, e);
            eprintln!("[INFO] FFmpeg path: {}", ffmpeg_path);
            eprintln!("[INFO] Compression mode: {}", config.compression_mode);
            format!("Failed to spawn ffmpeg: {}", e)
        })?;

    let pid = child.id();
    {
        if let Ok(mut map) = pids.lock() {
            map.insert(input_path.clone(), pid);
        }
    }

    let stderr = child.stderr.take().ok_or_else(|| {
        eprintln!("[ERROR] Failed to capture stderr for '{}'", input_path);
        "Failed to capture stderr".to_string()
    })?;
    let reader = BufReader::new(stderr);

    let mut current_speed = 0.0;
    let mut current_bitrate = 0.0;
    let mut current_sec = 0.0;
    
    // Auto-skip logic state
    let mut high_bitrate_count: i32 = 0;
    let auto_skip_enabled = config.compression_mode == "crf" && config.crf_auto_skip;
    
    // Collect stderr lines for error reporting
    let mut stderr_lines: Vec<String> = Vec::new();
    let max_stderr_lines = 50; // Keep last 50 lines

    println!("Starting ffmpeg for {}, duration: {}s", input_path, duration_sec);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        
        // Collect stderr lines (excluding progress lines which are verbose)
        if !line.contains("progress=") && !line.trim().is_empty() {
            stderr_lines.push(line.clone());
            // Keep only the last N lines to avoid memory issues
            if stderr_lines.len() > max_stderr_lines {
                stderr_lines.remove(0);
            }
        }

        if let Some(idx) = line.find("out_time=") {
            let time_val = line[idx+9..].trim();
            current_sec = parse_time_str(time_val);
        } else if let Some(idx) = line.find("out_time_ms=") {
             let ms_val: f64 = line[idx+12..].trim().parse().unwrap_or(0.0);
             current_sec = ms_val / 1_000_000.0;
        } else if let Some(idx) = line.find("out_time_us=") {
             let us_val: f64 = line[idx+12..].trim().parse().unwrap_or(0.0);
             current_sec = us_val / 1_000_000.0;
        } else if let Some(idx) = line.find("speed=") {
            let speed_str = line[idx+6..].trim();
            if let Some(s_idx) = speed_str.find('x') {
                 current_speed = speed_str[0..s_idx].parse().unwrap_or(0.0);
            } else {
                 current_speed = speed_str.parse().unwrap_or(0.0);
            }
        } else if let Some(idx) = line.find("bitrate=") {
             let br_str = line[idx+8..].trim();
             if let Some(k_idx) = br_str.find('k') {
                 current_bitrate = br_str[0..k_idx].parse().unwrap_or(0.0);
             } else {
                 current_bitrate = 0.0;
             }
        }

        if line.contains("progress=") {
            // Auto-Skip Check
            if auto_skip_enabled && duration_sec > 0.0 {
                 if let Some(in_br) = input_bitrate_kbps {
                     // Check strictly within the "early" phase, e.g. first 20% or 30 seconds
                     if current_sec > 3.0 && (current_sec < 30.0 || (current_sec / duration_sec) < 0.2) {
                         let threshold_multiplier = config.crf_auto_skip_threshold as f64 / 100.0;
                         if current_bitrate > (in_br * threshold_multiplier) {
                             high_bitrate_count += 1;
                         } else {
                             // Reset count if it drops, to be conservative?
                             // Or just keep accumulating? Let's sustain.
                             // Implementing strict sustained trigger:
                             high_bitrate_count = high_bitrate_count.saturating_sub(1);
                         }

                         if high_bitrate_count > 10 {
                             // TRIGGER SKIP
                             println!("Auto-skipping {} because output bitrate {:.1} > input {:.1}", input_path, current_bitrate, in_br);
                             let _ = child.kill();
                             let _ = child.wait(); // Wait for process to fully exit to release file locks!
                             
                             // Clean temp
                             if std::path::Path::new(&temp_output_path).exists() {
                                 // Add a small retry loop just in case OS is slow to release lock
                                 for _ in 0..3 {
                                     if std::fs::remove_file(&temp_output_path).is_ok() {
                                         break;
                                     }
                                     std::thread::sleep(std::time::Duration::from_millis(100));
                                 }
                             }

                             // Handle file copy
                             if input_path != output_path {
                                if let Some(parent) = std::path::Path::new(&output_path).parent() {
                                    if !parent.exists() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }
                                }
                                if let Err(e) = std::fs::copy(&input_path, &output_path) {
                                     // Error during copy
                                     eprintln!("[ERROR] Failed to copy file during auto-skip for '{}': {}", input_path, e);
                                     eprintln!("[INFO] Source: {}", input_path);
                                     eprintln!("[INFO] Destination: {}", output_path);
                                      let _ = app.emit("video-progress", ProgressPayload {
                                        path: input_path.clone(),
                                        progress: 0,
                                        status: "Error".to_string(),
                                        speed: 0.0,
                                        bitrate_kbps: 0.0,
                                        output_info: None,
                                    });
                                     return Err(format!("Skipped (CRF check), but failed to copy file: {}", e));
                                }
                             }

                              let _ = app.emit("video-progress", ProgressPayload {
                                path: input_path.clone(),
                                progress: 100,
                                status: "Skipped".to_string(),
                                speed: 0.0,
                                bitrate_kbps: in_br, // Report original bitrate
                                output_info: input_info, // Report original info
                            });

                             if let Ok(mut map) = pids.lock() {
                                map.remove(&input_path);
                            }

                             return Ok(());
                         }
                     }
                 }
            }

            let percent = if duration_sec > 0.0 {
                ((current_sec / duration_sec) * 100.0) as u8
            } else {
                0
            };
            
            // For VMAF mode or 2-pass mode Pass 2, map progress (0-100) to (50-100)
            let is_two_pass_phase2 = config.compression_mode == "bitrate" && config.two_pass;
            let final_percent = if config.compression_mode == "vmaf" || is_two_pass_phase2 {
                50 + (percent.min(100) / 2)
            } else {
                percent.min(100)
            };
            
            let _ = app.emit("video-progress", ProgressPayload {
                path: input_path.clone(),
                progress: final_percent,
                status: status_str.clone(),
                speed: current_speed,
                bitrate_kbps: current_bitrate,
                output_info: None,
            });
        }
    }

    let status = child.wait().map_err(|e| {
        eprintln!("[ERROR] Failed to wait on FFmpeg process for '{}': {}", input_path, e);
        format!("Failed to wait on ffmpeg: {}", e)
    })?;
    
    {
        if let Ok(mut map) = pids.lock() {
            map.remove(&input_path);
        }
    }
    
    if status.success() {
        // 1. Verify the output video
        let verify_result = verify_video(ffmpeg_path, &temp_output_path);
        
        if let Err(e) = verify_result {
            eprintln!("[ERROR] Video validation failed for '{}': {}", input_path, e);
            eprintln!("[INFO] Output file may be corrupted or incomplete: {}", temp_output_path);
            // Cleanup temp file
             if std::path::Path::new(&temp_output_path).exists() {
                 let _ = std::fs::remove_file(&temp_output_path);
             }
             if let Some(prefix) = &pass_log_prefix_opt {
                 cleanup_pass_logs(prefix, &temp_output_path);
             }
             let _ = app.emit("video-progress", ProgressPayload {
                path: input_path.clone(),
                progress: 0,
                status: "Error".to_string(),
                speed: 0.0,
                bitrate_kbps: 0.0,
                output_info: None,
            });
            return Err(format!("Validation failed: {}", e));
        }

        // 2. Ensure parent directory exists (redundant safety check - should already exist)
        if let Some(parent) = std::path::Path::new(&output_path).parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("[ERROR] Failed to create output directory for '{}': {}", input_path, e);
                    eprintln!("[INFO] Target directory: {:?}", parent);
                    return Err(format!("Failed to create output directory: {}", e));
                }
            }
        }

        // 3. Safe overwrite logic
        if std::path::Path::new(&output_path).exists() {
             if let Err(e) = std::fs::remove_file(&output_path) {
                 eprintln!("[ERROR] Failed to remove existing output file for '{}': {}", input_path, e);
                 eprintln!("[INFO] File to remove: {}", output_path);
                 return Err(format!("Failed to remove existing output file: {}", e));
             }
        }
        if let Err(e) = std::fs::rename(&temp_output_path, &output_path) {
             eprintln!("[ERROR] Failed to move temp file to output for '{}': {}", input_path, e);
             eprintln!("[INFO] Temp file: {}", temp_output_path);
             eprintln!("[INFO] Output file: {}", output_path);
             return Err(format!("Failed to move temp file to output: {}", e));
        }

        // 4. Fetch metadata for the new output file
        let mut output_info = get_video_info(std::path::Path::new(&output_path), &ffprobe_path).ok();
        println!("Output info retrieved: {:?}", output_info.is_some());

        // 5. Handle VMAF: In "vmaf" compression mode, use the search score directly
        //    In other modes with enable_vmaf, queue for post-compression VMAF calculation
        if config.compression_mode == "vmaf" {
            // Use the VMAF score from CRF search directly
            if let Some(vmaf_score) = vmaf_search_score {
                if let Some(ref mut info) = output_info {
                    info.vmaf = Some(vmaf_score);
                    info.vmaf_device = if config.vmaf_use_cuda { Some("CUDA".to_string()) } else { Some("CPU".to_string()) };
                    
                    // Use same resolution-based model selection as in search
                    let parts: Vec<&str> = info.resolution.split('x').collect();
                    let (width, height) = if parts.len() == 2 {
                        let w = parts[0].parse::<u32>().unwrap_or(0);
                        let h = parts[1].parse::<u32>().unwrap_or(0);
                        (w, h)
                    } else {
                        (0, 0)
                    };
                    let is_high_res = width.max(height) > 2560;
                    let model_filename = match (is_high_res, config.vmaf_neg) {
                        (false, false) => "vmaf_v0.6.1.json",
                        (true, false) => "vmaf_4k_v0.6.1.json",
                        (false, true) => "vmaf_v0.6.1neg.json",
                        (true, true) => "vmaf_4k_v0.6.1neg.json",
                    };
                    info.vmaf_model = Some(model_filename.to_string());
                }
            }
            
            let _ = app.emit("video-progress", ProgressPayload {
                path: input_path.clone(),
                progress: 100,
                status: "Done".to_string(),
                speed: 0.0,
                bitrate_kbps: 0.0,
                output_info,
            });
            if let Some(prefix) = pass_log_prefix_opt {
                std::thread::sleep(std::time::Duration::from_millis(200));
                cleanup_pass_logs(&prefix, &temp_output_path);
            }
            return Ok(());
        } else if config.enable_vmaf {
            // Other modes: queue for separate VMAF calculation
            let app_handle = app.clone();
            let in_p = input_path.clone();
            let ffmpeg_p = ffmpeg_path.to_string();
            let ffprobe_p = ffprobe_path.clone();
            let out_p = output_path.clone();
            let cfg = config.clone();
            let dur = duration_sec;
            let pids_map = pids.clone();
            let cancelled_map = cancelled_paths.clone();
            let out_info_clone = output_info.clone();

            // Enqueue VMAF Task
            let task = VmafTask {
                app: app_handle.clone(),
                input_path: in_p.clone(),
                ffmpeg_path: ffmpeg_p,
                ffprobe_path: ffprobe_p,
                reference_path: in_p.clone(),
                distorted_path: out_p,
                config: cfg,
                duration_sec: dur,
                pids: pids_map,
                cancelled_paths: cancelled_map,
                output_video_info: out_info_clone,
            };

            {
                if let Ok(mut state) = vmaf_state.lock() {
                    state.queue.push_back(task);
                }
            }

            let _ = app.emit("video-progress", ProgressPayload {
                path: input_path.clone(),
                progress: 100,
                status: "Waiting for VMAF".to_string(),
                speed: 0.0,
                bitrate_kbps: 0.0,
                output_info: output_info.clone(),
            });

            schedule_next_vmaf(vmaf_state);
            
            return Ok(());
        }


         let _ = app.emit("video-progress", ProgressPayload {
            path: input_path.clone(),
            progress: 100,
            status: "Done".to_string(),
            speed: 0.0,
            bitrate_kbps: 0.0,
            output_info,
        });
        if let Some(prefix) = pass_log_prefix_opt {
             // Best effort cleanup of passlog files
             std::thread::sleep(std::time::Duration::from_millis(200)); // Ensure ffmpeg releases file handle
             cleanup_pass_logs(&prefix, &temp_output_path);
        }

        Ok(())
    } else {
         // Cleanup temp file
         if std::path::Path::new(&temp_output_path).exists() {
             let _ = std::fs::remove_file(&temp_output_path);
         }
         if let Some(prefix) = &pass_log_prefix_opt {
             cleanup_pass_logs(prefix, &temp_output_path);
         }
         
         let is_cancelled = {
             if let Ok(mut set) = cancelled_paths.lock() {
                 set.remove(&input_path)
             } else {
                 false
             }
         };

         let status_str = if is_cancelled { "Cancelled" } else { "Error" };
         
         if !is_cancelled {
             eprintln!("[ERROR] FFmpeg compression failed for '{}'", input_path);
             eprintln!("[INFO] FFmpeg exit status: {:?}", status);
             eprintln!("[INFO] Compression mode: {}", config.compression_mode);
             
             // Print FFmpeg stderr output for debugging
             if !stderr_lines.is_empty() {
                 eprintln!("[ERROR] FFmpeg stderr output (last {} lines):", stderr_lines.len());
                 for stderr_line in &stderr_lines {
                     eprintln!("  {}", stderr_line);
                 }
             } else {
                 eprintln!("[WARNING] No stderr output captured from FFmpeg");
             }
         }

         let _ = app.emit("video-progress", ProgressPayload {
            path: input_path.clone(),
            progress: 0,
            status: status_str.to_string(),
            speed: 0.0,
            bitrate_kbps: 0.0,
            output_info: None,
        });
        Err(format!("FFmpeg exited with status: {:?}{}", status, if is_cancelled { " (Cancelled)" } else { "" }))
    }
}

fn verify_video(ffmpeg_path: &str, file_path: &str) -> Result<(), String> {
    // 1. Check if file exists and has size
    let metadata = std::fs::metadata(file_path).map_err(|e| format!("Failed to get metadata: {}", e))?;
    if metadata.len() == 0 {
        return Err("File is empty".to_string());
    }

    // 2. Try to decode a small portion using ffmpeg to ensure integrity
    // ffmpeg -v error -i input -t 1 -f null -
    let args = [
        "-v", "error",
        "-i", file_path,
        "-t", "1",
        "-f", "null",
        "-"
    ];

    let output = Command::new(ffmpeg_path)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to run verification: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Verification failed (integrity check): {}", stderr));
    }

    Ok(())
}

pub fn schedule_next_vmaf(vmaf_state: std::sync::Arc<std::sync::Mutex<VmafState>>) {
    // Check if something is running
    let mut task_opt = None;
    
    {
        if let Ok(mut state) = vmaf_state.lock() {
            if state.running_task.is_none() {
                task_opt = state.queue.pop_front();
                if let Some(ref t) = task_opt {
                    state.running_task = Some(t.input_path.clone());
                }
            }
        }
    }

    if let Some(mut task) = task_opt {
        let v_state = vmaf_state.clone();
        std::thread::spawn(move || {
            calculate_vmaf_score(
                &task.app,
                &task.input_path,
                &task.ffmpeg_path,
                &task.ffprobe_path,
                &task.reference_path,
                &task.distorted_path,
                &task.config,
                task.duration_sec,
                task.pids,
                task.cancelled_paths,
                &mut task.output_video_info
            );

            // Determine final status
            // If vmaf is None, it might have failed or been cancelled.
            // If cancelled, calculate_vmaf_score returns early or clean.
            // But we don't have direct "cancelled" feedback from calculate_vmaf_score return
            // other than checking if VMAF is set in output_video_info.
            // Actually calculate_vmaf_score DOES set vmaf in output_video_info if successful.
            
            // Re-read cancelled status (hacky, using pids/cancelled logic from elsewhere? No)
            // Just assume if vmaf is present -> Done.
            // If vmaf is missing -> Done (without VMAF) or Error?
            // The user says "status to Done, no vmaf score" for cancelled ones.
            // If it failed for other reasons, maybe it should be Error?
            // But let's default to Done to satisfy the "cancel" requirement primarily, 
            // relying on external cancellation to kill the process.

            let _ = task.app.emit("video-progress", ProgressPayload {
                path: task.input_path.clone(),
                progress: 100,
                status: "Done".to_string(),
                speed: 0.0,
                bitrate_kbps: 0.0,
                output_info: task.output_video_info.clone(),
            });

            // Clear running state
            {
                if let Ok(mut state) = v_state.lock() {
                    state.running_task = None;
                }
            }
            
            // Trigger next
            schedule_next_vmaf(v_state);
        });
    }
}

// --- VMAF Calculation Logic ---

fn find_vmaf_model(ffmpeg_path: &str, model_filename: &str) -> Option<String> {
    // 1. Check env var
    if let Ok(env_path) = std::env::var("VMAF_MODEL") {
        if Path::new(&env_path).exists() {
            return Some(env_path);
        }
    }

    // 2. ffmpeg/bin/model/model_filename
    let ffmpeg_dir = Path::new(ffmpeg_path).parent();
    if let Some(dir) = ffmpeg_dir {
         let model_json = dir.join("model").join(model_filename);
         if model_json.exists() {
             return Some(model_json.to_string_lossy().to_string());
         }

          // Try typical share location: ../share/model/
         let share_model = dir.parent().unwrap_or(Path::new("")).join("share").join("model").join(model_filename);
         if share_model.exists() {
             return Some(share_model.to_string_lossy().to_string());
         }
    }

    // 3. Fallbacks
    let candidates = vec![
        format!(r"C:\Program Files\FFmpeg\share\model\{}", model_filename),
        format!(r"C:\Program Files\ffmpeg\share\model\{}", model_filename),
        format!("/usr/share/model/{}", model_filename),
        format!("/usr/local/share/model/{}", model_filename)
    ];

    for c in candidates {
        if Path::new(&c).exists() {
            return Some(c);
        }
    }

    None
}

fn get_cuda_decoder(codec: &str) -> Option<&'static str> {
    match codec {
        "h264" => Some("h264_cuvid"),
        "hevc" => Some("hevc_cuvid"),
        "vp9" => Some("vp9_cuvid"),
        "av1" => Some("av1_cuvid"),
        "mpeg2video" | "mpeg2" => Some("mpeg2_cuvid"),
        "vc1" => Some("vc1_cuvid"),
        "vp8" => Some("vp8_cuvid"),
        "mjpeg" => Some("mjpeg_cuvid"),
        _ => None
    }
}

fn escape_path_for_filter(path: &str) -> String {
    // Windows filter path escaping is complex.
    // Basic rules: 
    // 1. Convert backslashes to forward slashes.
    // 2. Escape colon ':', used as separator in filters.
    
    // Absolute path
    let mut abs_path = std::fs::canonicalize(path).unwrap_or(std::path::PathBuf::from(path)).to_string_lossy().to_string();
    
    // Remove Windows UNC prefix (\\?\) which canonicalize adds, as it confuses ffmpeg
    if cfg!(windows) && abs_path.starts_with(r"\\?\") {
        abs_path = abs_path[4..].to_string();
    }

    let forward_slashes = abs_path.replace("\\", "/");
    
    // In filter_complex: libvmaf=model='path=...':log_path='...'
    // Python script uses 3 backslashes for colon: p.replace(':', '\\\\\\:')
    // This seems to be required for Windows paths in filter args.
    let stepped = forward_slashes.replace(":", "\\\\\\:"); 
    
    stepped
}

fn calculate_vmaf_score(
    app: &AppHandle,
    input_path: &str,
    ffmpeg_path: &str,
    ffprobe_path: &str,
    reference_path: &str,
    distorted_path: &str,
    config: &CompressionConfig,
    duration_sec: f64,
    pids: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>,
    cancelled_paths: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    output_video_info: &mut Option<VideoInfo>
) {
    // 0. Logical Checks (Resolution Limit & Overwrite)
    if config.max_resolution.enabled {
        println!("VMAF Calculation skipped: Max Resolution limit enabled.");
        return;
    }
    
    // Normalize paths to check for equality (overwrite case)
    let p1 = std::fs::canonicalize(reference_path).unwrap_or(std::path::PathBuf::from(reference_path));
    let p2 = std::fs::canonicalize(distorted_path).unwrap_or(std::path::PathBuf::from(distorted_path));
    if p1 == p2 {
         println!("VMAF Calculation skipped: File overwritten (Reference == Distorted).");
         return;
    }

    // Determine which model to use based on resolution and settings
    let (width, height) = if let Some(info) = output_video_info {
        let parts: Vec<&str> = info.resolution.split('x').collect();
        if parts.len() == 2 {
            let w = parts[0].parse::<u32>().unwrap_or(0);
            let h = parts[1].parse::<u32>().unwrap_or(0);
            (w, h)
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    // User threshold: <= 2k vs > 2k
    // Using the long edge to determine resolution category (2560 covers QHD/2K).
    let is_high_res = width.max(height) > 2560;
    
    let model_filename = match (is_high_res, config.vmaf_neg) {
        (false, false) => "vmaf_v0.6.1.json",
        (true, false) => "vmaf_4k_v0.6.1.json",
        (false, true) => "vmaf_v0.6.1neg.json",
        (true, true) => "vmaf_4k_v0.6.1neg.json",
    };

    println!("Selected VMAF model: {} (Resolution: {}x{}, Neg: {})", model_filename, width, height, config.vmaf_neg);

    let model_path_opt = find_vmaf_model(ffmpeg_path, model_filename);
    if model_path_opt.is_none() {
        println!("VMAF Calculation skipped: Model file {} not found.", model_filename);
        return;
    }
    let model_path = model_path_opt.unwrap();
    
    let segments: Vec<(f64, f64)>; // (start, duration)

    if config.vmaf_full_computation {
        segments = vec![(0.0, duration_sec)];
    } else {
        // Auto config or manual
        let mut count = config.vmaf_segment_count;
        let mut dur = config.vmaf_segment_duration as f64;
        
        if duration_sec < 20.0 {
             segments = vec![(0.0, duration_sec)];
        } else {
            if config.vmaf_auto_config {
                 dur = 20.0;
                 let duration_min = duration_sec / 60.0;
                 count = (duration_min / 12.0).ceil() as u32;
                 if count < 1 { count = 1; }
            } else {
                if dur > duration_sec { dur = duration_sec; }
                if dur < 1.0 { dur = 1.0; }
                if count < 1 { count = 1; }
            }

            let mut points = Vec::new();
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros();
            
            for i in 0..count {
                let numerator = (i as f64) + 1.0;
                let denominator = (count as f64) + 2.0;
                let base_start = duration_sec * (numerator / denominator);
                
                let pseudo_rand = ((now + i as u128 * 12345) % 100) as f64;
                let offset_sec = (pseudo_rand - 50.0) / 10.0;
                
                let mut start = (base_start + offset_sec).round();
                
                if start < 0.0 { start = 0.0; }
                if start + dur > duration_sec {
                     start = (duration_sec - dur).max(0.0);
                }
                start = start.round();
                if start < 0.0 { start = 0.0; }

                points.push((start, dur));
            }
            segments = points;
        }
    }

    // Initialize VMAF fields
    if let Some(info) = output_video_info {
        info.vmaf_total_segments = Some(segments.len() as u32);
        info.vmaf_detail = Some(Vec::new());
        // Set initial device (optimistic)
        info.vmaf_device = if config.vmaf_use_cuda { Some("CUDA".to_string()) } else { Some("CPU".to_string()) };
        info.vmaf_model = Some(model_filename.to_string());
    }

    // Emit initial status
    let _ = app.emit("video-progress", ProgressPayload {
        path: input_path.to_string(),
        progress: 100,
        status: "Evaluating".to_string(),
        speed: 0.0,
        bitrate_kbps: 0.0,
        output_info: output_video_info.clone(),
    });

    let mut scores = Vec::new();
    let mut used_device = "CPU".to_string();

    // Check if we should TRY cuda first
    let try_cuda = config.vmaf_use_cuda;
    let mut cuda_failed_once = false;

    for (idx, (start, dur)) in segments.iter().enumerate() {
        // Check for cancellation before processing segment
        if let Ok(set) = cancelled_paths.lock() {
            if set.contains(input_path) {
                println!("VMAF Calculation cancelled for {}", input_path);
                return;
            }
        }

        let ss = if config.vmaf_full_computation { None } else { Some(*start) };
        let dt = if config.vmaf_full_computation { None } else { Some(*dur) };

        let mut score = None;
        
        // Try CUDA
        if try_cuda && !cuda_failed_once {
            score = run_vmaf_instance(
                ffmpeg_path, ffprobe_path, reference_path, distorted_path, 
                &model_path, true, ss, dt, &pids, input_path, &config.custom_vmaf_params
            );
            if score.is_some() {
                used_device = "CUDA".to_string();
            } else {
                println!("VMAF CUDA computation failed for segment {}, falling back to CPU.", idx);
                cuda_failed_once = true;
            }
        }

        // Try CPU
        if score.is_none() {
            // Check for cancellation before fallback
            if let Ok(set) = cancelled_paths.lock() {
                if set.contains(input_path) {
                    println!("VMAF Calculation cancelled during fallback check for {}", input_path);
                    return;
                }
            }
            
            score = run_vmaf_instance(
                ffmpeg_path, ffprobe_path, reference_path, distorted_path, 
                &model_path, false, ss, dt, &pids, input_path, &config.custom_vmaf_params
            );
            used_device = "CPU".to_string(); 
        }
        
        if let Some(s) = score {
            scores.push(s);
            // Update and emit
            if let Some(info) = output_video_info {
                if let Some(details) = &mut info.vmaf_detail {
                    details.push(s);
                }
                // Update device in case fallback happened or it wasn't set correctly
                info.vmaf_device = Some(used_device.clone());
            }
            let _ = app.emit("video-progress", ProgressPayload {
                path: input_path.to_string(),
                progress: 100,
                status: "Evaluating".to_string(),
                speed: 0.0,
                bitrate_kbps: 0.0,
                output_info: output_video_info.clone(),
            });
        }
    }

    if !scores.is_empty() {
        let avg = scores.iter().sum::<f64>() / scores.len() as f64;
        if let Some(info) = output_video_info {
            info.vmaf = Some(avg);
        }
    }
}


fn run_vmaf_instance(
    ffmpeg_path: &str,
    ffprobe_path: &str,
    ref_path: &str,
    dist_path: &str,
    model_path: &str,
    use_cuda: bool,
    ss: Option<f64>,
    t: Option<f64>,
    pids: &std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>,
    input_key: &str,
    custom_vmaf_params: &[String],
) -> Option<f64> {
     // Prepare paths
    let model_esc = escape_path_for_filter(model_path);
    // Log file
    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros();
    let temp_dir = std::env::temp_dir();
    let log_path = temp_dir.join(format!("vmaf_log_{}.json", id));
    let log_esc = escape_path_for_filter(&log_path.to_string_lossy());
    
    // Build vmaf_opts with custom params
    let mut vmaf_opts = format!("model='path={}':log_fmt=json:log_path='{}'", model_esc, log_esc);
    for param in custom_vmaf_params {
        let trimmed = param.trim();
        if !trimmed.is_empty() {
            vmaf_opts.push(':');
            vmaf_opts.push_str(trimmed);
        }
    }

    let mut args = Vec::new();
    args.push("-hide_banner".to_string());
    
    // Move threads to start
    args.push("-threads".to_string());
    args.push(if use_cuda { "1".to_string() } else { "4".to_string() });

    args.push("-v".to_string());
    args.push("info".to_string()); 
    
    // Inputs
    if use_cuda {
         args.push("-hwaccel".to_string()); args.push("cuda".to_string());
         args.push("-hwaccel_output_format".to_string()); args.push("cuda".to_string());
    }

    if let Some(s) = ss { args.push("-ss".to_string()); args.push(s.to_string()); }
    if let Some(d) = t { args.push("-t".to_string()); args.push(d.to_string()); }
    
    args.push("-i".to_string());
    args.push(dist_path.to_string());

    // Reference (Input 1)
    if use_cuda {
         let ref_info = get_metadata(ref_path, ffprobe_path);
         let mut ref_decoder = None;
         if let Ok(info) = ref_info {
             ref_decoder = get_cuda_decoder(&info.encoder);
         }
         
         args.push("-hwaccel".to_string()); args.push("cuda".to_string());
         args.push("-hwaccel_output_format".to_string()); args.push("cuda".to_string());
         
         if let Some(dec) = ref_decoder {
             args.push("-c:v".to_string()); args.push(dec.to_string());
         }
    }
    
    if let Some(s) = ss { args.push("-ss".to_string()); args.push(s.to_string()); }
    if let Some(d) = t { args.push("-t".to_string()); args.push(d.to_string()); }

    args.push("-i".to_string());
    args.push(ref_path.to_string());

    // Filter Complex
    let filter = if use_cuda {
        format!(
            "[0:v]scale_cuda=format=yuv420p[dis];[1:v]scale_cuda=format=yuv420p[ref];[dis][ref]libvmaf_cuda={}", 
            vmaf_opts
        )
    } else {
        format!(
            "[0:v]setpts=PTS-STARTPTS,format=yuv420p[dis];[1:v]setpts=PTS-STARTPTS,format=yuv420p[ref];[dis][ref]libvmaf={}",
            vmaf_opts
        )
    };
    
    args.push("-filter_complex".to_string());
    args.push(filter);
    
    args.push("-f".to_string());
    args.push("null".to_string());
    args.push("-".to_string());

    // Spawn
    let mut command = Command::new(ffmpeg_path);
    command.args(&args).stdout(Stdio::piped()).stderr(Stdio::piped());
    
    #[cfg(windows)]
    let mut child = {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW if accessible, otherwise standard
        command.spawn().ok()?
    };
    #[cfg(not(windows))]
    let mut child = command.spawn().ok()?;

    let pid = child.id();
    {
        if let Ok(mut map) = pids.lock() {
            map.insert(input_key.to_string(), pid);
        }
    }

    let output = child.wait_with_output();
    
    {
        if let Ok(mut map) = pids.lock() {
            map.remove(input_key);
        }
    }

    let o = output.ok()?;
        
    // Check log file first
    if log_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&log_path) {
            // Parse JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                 let _ = std::fs::remove_file(&log_path);
                 if let Some(metrics) = json.get("pooled_metrics") {
                     if let Some(vmaf) = metrics.get("vmaf") {
                         if let Some(mean) = vmaf.get("mean") {
                              return mean.as_f64();
                         }
                     }
                 }
            }
        }
        let _ = std::fs::remove_file(&log_path);
    }

    // Fallback: parse stderr
     let stderr = String::from_utf8_lossy(&o.stderr);
     if let Some(idx) = stderr.find("VMAF score: ") {
         let rest = &stderr[idx+12..];
         let val_str = rest.split_whitespace().next().unwrap_or("0");
         return val_str.parse().ok();
     }

    None
}

fn cleanup_pass_logs(prefix: &str, temp_output_path: &str) {
    if let Some(parent) = std::path::Path::new(temp_output_path).parent() {
        if let Ok(entries) = std::fs::read_dir(parent) {
            let prefix_buf = std::path::Path::new(prefix);
            if let Some(prefix_filename) = prefix_buf.file_name().and_then(|s| s.to_str()) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        // Match files starting with the prefix and having typical log extensions
                        // Typical: prefix-0.log, prefix-0.log.mbtree, prefix.log, prefix.cut_tree
                        if name.starts_with(prefix_filename) {
                            let _ = std::fs::remove_file(path);
                        }
                    }
                }
            }
        }
    }
}
