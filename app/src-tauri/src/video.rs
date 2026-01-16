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
    #[serde(rename = "minBitrateThreshold")]
    pub min_bitrate_threshold: u32,
    #[serde(default)]
    #[serde(rename = "crfAutoSkip")]
    pub crf_auto_skip: bool,
    #[serde(default)]
    #[serde(rename = "crfAutoSkipThreshold")]
    pub crf_auto_skip_threshold: u32,
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
                                duration_sec: 0.0,
                                speed: None,
                                bitrate_kbps: None,
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

pub fn detect_system_encoders(ffmpeg_path: &str) -> DetectionReport {
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
    let params_cpu = [
        "libx264", "x264", "libx265", "x265", "libvpx", "libvpx-vp9", "vp9", 
        "libaom-av1", "av1", "libsvtav1", "mpeg4", "libmpeg4", "wmv1", "wmv2", "mpeg2video", "msmpeg4v2"
    ];
    let hw_keywords = ["nvenc", "amf", "qsv", "cuda", "vaapi", "vdpau"];

    for (name, desc) in all_video {
        let is_hw = hw_keywords.iter().any(|k| name.contains(k));
        let is_target_cpu = params_cpu.contains(&name.as_str());

        if is_hw || is_target_cpu {
             let args = [
                "-y", "-hide_banner", "-v", "error",
                "-f", "lavfi", "-i", "color=size=1280x720:rate=30",
                "-frames:v", "1", "-pix_fmt", "yuv420p",
                "-c:v", &name, "-f", "null", "-"
            ];
            
            match Command::new(ffmpeg_path).args(&args).output() {
                Ok(o) if o.status.success() => {
                    report.video.push(DetectedEncoder {
                        name: if is_hw { format!("{} (HW)", name) } else { format!("{} (CPU)", name) },
                        value: name,
                        is_hardware: is_hw,
                        description: desc,
                    });
                },
                _ => {}
            }
        }
    }

    // 3. Filter & Test Audio
     let target_audio = [
        "aac", "aac_mf", "libfdk_aac", "libmp3lame", "mp3_mf", "libopus", "opus", 
        "flac", "alac", "ac3", "eac3", "wmav2", "wmav1", "mp2", "pcm_s16le", "libvorbis"
    ];

    for (name, desc) in all_audio {
        if target_audio.contains(&name.as_str()) {
             let args = [
                "-y", "-hide_banner", "-v", "error",
                "-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo",
                "-t", "1", "-c:a", &name, "-f", "null", "-"
            ];

             match Command::new(ffmpeg_path).args(&args).output() {
                Ok(o) if o.status.success() => {
                    report.audio.push(DetectedEncoder {
                        name: format!("{} (CPU)", name),
                        value: name,
                        is_hardware: false,
                        description: desc,
                    });
                },
                _ => {}
            }
        }
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
    })
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    path: String,
    progress: u8,
    status: String,
    speed: f64,
    bitrate_kbps: f64,
    output_info: Option<VideoInfo>,
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

pub fn process_video(
    app: AppHandle,
    ffmpeg_path: &str,
    input_path: String,
    output_path: String,
    config: CompressionConfig,
    duration_sec: f64,
    pids: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, u32>>>,
    cancelled_paths: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>
) -> Result<(), String> {
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

    let mut args = Vec::new();
    args.push("-y".to_string());
    args.push("-hide_banner".to_string());
    args.push("-i".to_string());
    args.push(input_path.clone());

    // Video Encoder
    args.push("-c:v".to_string());
    let v_enc = if config.video_encoder.is_empty() { "libx264".to_string() } else { config.video_encoder };
    args.push(v_enc.clone());

    // Compression Mode
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
            args.push("-crf".to_string());
             args.push("23".to_string()); 
        },
        _ => {}
    }

    // Audio Encoder
    args.push("-c:a".to_string());
    let a_enc = if config.audio_encoder.is_empty() { "aac".to_string() } else { config.audio_encoder };
    args.push(a_enc.clone());

    // Resolution
    if config.max_resolution.enabled && config.max_resolution.width > 0 && config.max_resolution.height > 0 {
        args.push("-vf".to_string());
        args.push(format!("scale='min({},iw)':-2", config.max_resolution.width));
    }

    // Custom Filters
    for filter in config.custom_filters {
        if !filter.trim().is_empty() {
             let parts: Vec<&str> = filter.split_whitespace().collect();
             for p in parts {
                 args.push(p.to_string());
             }
        }
    }
    
    // Encoder Specific Params
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

    // threads
    if config.ffmpeg_threads > 0 {
        args.push("-threads".to_string());
        args.push(format!("{}", config.ffmpeg_threads));
    }

    args.push("-progress".to_string());
    args.push("pipe:2".to_string());

    let temp_output_path = format!("{}.tmp.{}", output_path, config.target_format);
    
    args.push(temp_output_path.clone());

    let _ = app.emit("video-progress", ProgressPayload {
        path: input_path.clone(),
        progress: 0,
        status: "Processing".to_string(),
        speed: 0.0,
        bitrate_kbps: 0.0,
        output_info: None,
    });

    let mut child = Command::new(ffmpeg_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn ffmpeg: {}", e))?;

    let pid = child.id();
    {
        if let Ok(mut map) = pids.lock() {
            map.insert(input_path.clone(), pid);
        }
    }

    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let reader = BufReader::new(stderr);

    let mut current_speed = 0.0;
    let mut current_bitrate = 0.0;
    let mut current_sec = 0.0;
    
    // Auto-skip logic state
    let mut high_bitrate_count: i32 = 0;
    let auto_skip_enabled = config.compression_mode == "crf" && config.crf_auto_skip;

    println!("Starting ffmpeg for {}, duration: {}s", input_path, duration_sec);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

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
            
            let _ = app.emit("video-progress", ProgressPayload {
                path: input_path.clone(),
                progress: percent.min(100),
                status: "Processing".to_string(),
                speed: current_speed,
                bitrate_kbps: current_bitrate,
                output_info: None,
            });
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait on ffmpeg: {}", e))?;
    
    {
        if let Ok(mut map) = pids.lock() {
            map.remove(&input_path);
        }
    }
    
    if status.success() {
        // 1. Verify the output video
        let verify_result = verify_video(ffmpeg_path, &temp_output_path);
        
        if let Err(e) = verify_result {
            // Cleanup temp file
             if std::path::Path::new(&temp_output_path).exists() {
                 let _ = std::fs::remove_file(&temp_output_path);
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

        // 2. Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(&output_path).parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return Err(format!("Failed to create output directory: {}", e));
                }
            }
        }

        // 3. Safe overwrite logic
        if std::path::Path::new(&output_path).exists() {
             if let Err(e) = std::fs::remove_file(&output_path) {
                 return Err(format!("Failed to remove existing output file: {}", e));
             }
        }
        if let Err(e) = std::fs::rename(&temp_output_path, &output_path) {
             return Err(format!("Failed to move temp file to output: {}", e));
        }

        // 4. Fetch metadata for the new output file
        let output_info = get_video_info(std::path::Path::new(&output_path), &ffprobe_path).ok();

         let _ = app.emit("video-progress", ProgressPayload {
            path: input_path.clone(),
            progress: 100,
            status: "Done".to_string(),
            speed: 0.0,
            bitrate_kbps: 0.0,
            output_info,
        });
        Ok(())
    } else {
         // Cleanup temp file
         if std::path::Path::new(&temp_output_path).exists() {
             let _ = std::fs::remove_file(&temp_output_path);
         }
         
         let is_cancelled = {
             if let Ok(mut set) = cancelled_paths.lock() {
                 set.remove(&input_path)
             } else {
                 false
             }
         };

         let status_str = if is_cancelled { "Cancelled" } else { "Error" };

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
