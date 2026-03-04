mod video;

use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State, Manager}; // Manager added for manage()
use sysinfo::{System};
use nvml_wrapper::Nvml;
use std::time::Duration;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::process::Command; // For cancel command

#[derive(serde::Serialize, Clone)]
struct SystemStats {
    cpu_usage: f32,
    memory_usage: f32,
    gpu_usage: f32,
}

struct ProcessingState {
    pids: Arc<Mutex<HashMap<String, u32>>>,
    cancelled_paths: Arc<Mutex<HashSet<String>>>,
    vmaf_state: Arc<Mutex<video::VmafState>>,
}

// Helper function to resolve FFmpeg path from bundled resources
fn resolve_ffmpeg_path(app: &AppHandle) -> String {
    println!("[DEBUG] Resolving ffmpeg path...");
    
    // Try bundled resource directory first
    if let Ok(resource_dir) = app.path().resource_dir() {
        println!("[DEBUG] Resource dir: {:?}", resource_dir);
        // Try multiple possible paths in bundled resources
        let bundled_candidates = vec![
            resource_dir.join("ffmpeg/bin/ffmpeg.exe"),  // Standard: bin subdirectory
            resource_dir.join("ffmpeg/ffmpeg.exe"),       // Alternative: direct in ffmpeg dir
        ];
        
        for bundled_path in bundled_candidates {
            println!("[DEBUG] Checking bundled path: {:?}", bundled_path);
            if bundled_path.exists() {
                if let Some(path_str) = bundled_path.to_str() {
                    println!("[DEBUG] Found bundled ffmpeg at: {}", path_str);
                    return path_str.to_string();
                }
            }
        }
    }

    // Fallback 1: Try relative path from dev build
    let ffmpeg_rel = PathBuf::from("../ffmpeg/bin/ffmpeg.exe");
    println!("[DEBUG] Checking relative path: {:?}", ffmpeg_rel);
    if ffmpeg_rel.exists() {
        if let Ok(canonical) = std::fs::canonicalize(&ffmpeg_rel) {
            if let Some(path_str) = canonical.to_str() {
                println!("[DEBUG] Found ffmpeg at: {}", path_str);
                return path_str.to_string();
            }
        }
    }

    // Fallback 2: Try project root relative path
    let root_rel = PathBuf::from("ffmpeg/bin/ffmpeg.exe");
    println!("[DEBUG] Checking root relative path: {:?}", root_rel);
    if root_rel.exists() {
        if let Ok(canonical) = std::fs::canonicalize(&root_rel) {
            if let Some(path_str) = canonical.to_str() {
                println!("[DEBUG] Found ffmpeg at: {}", path_str);
                return path_str.to_string();
            }
        }
    }

    // Fallback 3: Try one more level up
    let up_rel = PathBuf::from("../../ffmpeg/bin/ffmpeg.exe");
    println!("[DEBUG] Checking ../../ relative path: {:?}", up_rel);
    if up_rel.exists() {
        if let Ok(canonical) = std::fs::canonicalize(&up_rel) {
            if let Some(path_str) = canonical.to_str() {
                println!("[DEBUG] Found ffmpeg at: {}", path_str);
                return path_str.to_string();
            }
        }
    }

    // Fallback 4: Try alternative paths (direct ffmpeg folder)
    let alt_paths = vec![
        PathBuf::from("ffmpeg/ffmpeg.exe"),
        PathBuf::from("../ffmpeg/ffmpeg.exe"),
        PathBuf::from("../../ffmpeg/ffmpeg.exe"),
    ];
    
    for alt_path in alt_paths {
        println!("[DEBUG] Checking alternative path: {:?}", alt_path);
        if alt_path.exists() {
            if let Ok(canonical) = std::fs::canonicalize(&alt_path) {
                if let Some(path_str) = canonical.to_str() {
                    println!("[DEBUG] Found ffmpeg at: {}", path_str);
                    return path_str.to_string();
                }
            }
        }
    }

    // Last resort: hope it's in PATH
    println!("[DEBUG] FFmpeg not found, using PATH");
    "ffmpeg.exe".to_string()
}

// Helper function to resolve FFprobe path from bundled resources
fn resolve_ffprobe_path(app: &AppHandle) -> String {
    println!("[DEBUG] Resolving ffprobe path...");
    
    // Try bundled resource directory first
    if let Ok(resource_dir) = app.path().resource_dir() {
        println!("[DEBUG] Resource dir: {:?}", resource_dir);
        // Try multiple possible paths in bundled resources
        let bundled_candidates = vec![
            resource_dir.join("ffmpeg/bin/ffprobe.exe"),  // Standard: bin subdirectory
            resource_dir.join("ffmpeg/ffprobe.exe"),       // Alternative: direct in ffmpeg dir
        ];
        
        for bundled_path in bundled_candidates {
            println!("[DEBUG] Checking bundled path: {:?}", bundled_path);
            if bundled_path.exists() {
                if let Some(path_str) = bundled_path.to_str() {
                    println!("[DEBUG] Found bundled ffprobe at: {}", path_str);
                    return path_str.to_string();
                }
            }
        }
    }

    // Fallback 1: Try relative path from dev build
    let ffprobe_rel = PathBuf::from("../ffmpeg/bin/ffprobe.exe");
    println!("[DEBUG] Checking relative path: {:?}", ffprobe_rel);
    if ffprobe_rel.exists() {
        if let Ok(canonical) = std::fs::canonicalize(&ffprobe_rel) {
            if let Some(path_str) = canonical.to_str() {
                println!("[DEBUG] Found ffprobe at: {}", path_str);
                return path_str.to_string();
            }
        }
    }

    // Fallback 2: Try project root relative path
    let root_rel = PathBuf::from("ffmpeg/bin/ffprobe.exe");
    println!("[DEBUG] Checking root relative path: {:?}", root_rel);
    if root_rel.exists() {
        if let Ok(canonical) = std::fs::canonicalize(&root_rel) {
            if let Some(path_str) = canonical.to_str() {
                println!("[DEBUG] Found ffprobe at: {}", path_str);
                return path_str.to_string();
            }
        }
    }

    // Fallback 3: Try one more level up
    let up_rel = PathBuf::from("../../ffmpeg/bin/ffprobe.exe");
    println!("[DEBUG] Checking ../../ relative path: {:?}", up_rel);
    if up_rel.exists() {
        if let Ok(canonical) = std::fs::canonicalize(&up_rel) {
            if let Some(path_str) = canonical.to_str() {
                println!("[DEBUG] Found ffprobe at: {}", path_str);
                return path_str.to_string();
            }
        }
    }

    // Fallback 4: Try alternative paths (direct ffmpeg folder)
    let alt_paths = vec![
        PathBuf::from("ffmpeg/ffprobe.exe"),
        PathBuf::from("../ffmpeg/ffprobe.exe"),
        PathBuf::from("../../ffmpeg/ffprobe.exe"),
    ];
    
    for alt_path in alt_paths {
        println!("[DEBUG] Checking alternative path: {:?}", alt_path);
        if alt_path.exists() {
            if let Ok(canonical) = std::fs::canonicalize(&alt_path) {
                if let Some(path_str) = canonical.to_str() {
                    println!("[DEBUG] Found ffprobe at: {}", path_str);
                    return path_str.to_string();
                }
            }
        }
    }

    // Last resort: hope it's in PATH
    println!("[DEBUG] FFprobe not found, using PATH");
    "ffprobe.exe".to_string()
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_background_image(app: AppHandle) -> Option<Vec<u8>> {
    // 1. Try executable directory
    if let Ok(exe_dir) = app.path().executable_dir() {
        println!("Checking for background image in exe_dir: {:?}", exe_dir);
        for ext in &["png", "jpg", "jpeg"] {
            let path = exe_dir.join(format!("back.{}", ext));
            if path.exists() {
                println!("Found background image at: {:?}", path);
                return std::fs::read(path).ok();
            }
        }
    }

    // 2. Try current working directory (useful for dev)
    if let Ok(cwd) = std::env::current_dir() {
        println!("Checking for background image in cwd: {:?}", cwd);
        for ext in &["png", "jpg", "jpeg"] {
            let path = cwd.join(format!("back.{}", ext));
            if path.exists() {
                println!("Found background image at: {:?}", path);
                return std::fs::read(path).ok();
            }
        }
    }

    println!("No background image (back.png/jpg) found.");
    None
}

#[tauri::command]
async fn scan_directory(path: String) -> Result<video::ScanResult, String> {
    Ok(video::scan_videos(&path))
}

#[tauri::command]
async fn scan_multiple_paths(paths: Vec<String>) -> Result<video::ScanResult, String> {
    Ok(video::scan_multiple_paths(paths))
}

#[tauri::command]
async fn categorize_paths(paths: Vec<String>) -> Result<video::PathCategorization, String> {
    Ok(video::categorize_paths(paths))
}

#[tauri::command]
async fn get_video_metadata(app: AppHandle, path: String) -> Result<video::VideoInfo, String> {
    let ffprobe_path = resolve_ffprobe_path(&app);
    video::get_metadata(&path, &ffprobe_path)
}

#[tauri::command]
async fn detect_encoders(app: AppHandle) -> Result<video::DetectionReport, String> {
    let ffmpeg_path = resolve_ffmpeg_path(&app);
    Ok(video::detect_system_encoders(&ffmpeg_path, app))
}

#[tauri::command]
async fn start_processing(
    app: AppHandle,
    state: State<'_, ProcessingState>,
    input_path: String,
    output_path: String,
    config: video::CompressionConfig,
    duration_sec: f64
) -> Result<(), String> {
    let ffmpeg_path = resolve_ffmpeg_path(&app);
    let pids = state.pids.clone();
    let cancelled_paths = state.cancelled_paths.clone();
    let vmaf_state = state.vmaf_state.clone();

    tauri::async_runtime::spawn_blocking(move || {
        video::process_video(app, &ffmpeg_path, input_path, output_path, config, duration_sec, pids, cancelled_paths, vmaf_state)
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn cancel_processing(
    app: AppHandle,
    state: State<'_, ProcessingState>,
    path: String
) -> Result<(), String> {
    let pid_opt = {
        let map = state.pids.lock().map_err(|e| e.to_string())?;
        map.get(&path).cloned()
    };

    if let Some(pid) = pid_opt {
        // Mark as cancelled BEFORE killing
        if let Ok(mut set) = state.cancelled_paths.lock() {
            set.insert(path.clone());
        }

        #[cfg(target_os = "windows")]
        {
             use std::os::windows::process::CommandExt;
             let _ = Command::new("taskkill")
                .args(&["/F", "/PID", &pid.to_string()])
                .creation_flags(0x08000000)
                .output()
                .map_err(|e| e.to_string())?;
        }
        #[cfg(not(target_os = "windows"))]
        {
             let _ = Command::new("kill")
                .args(&["-9", &pid.to_string()])
                .output()
                .map_err(|e| e.to_string())?;
        }
    }

    // Connect to VMAF State to remove from queue if present
    let mut removed_from_queue = false;
    {
        if let Ok(mut v_state) = state.vmaf_state.lock() {
             if let Some(pos) = v_state.queue.iter().position(|t| t.input_path == path) {
                 v_state.queue.remove(pos);
                 removed_from_queue = true;
             }
        }
    }

    if removed_from_queue {
        // Emit Done status as requested (Status Done, No VMAF)
        let _ = app.emit("video-progress", video::ProgressPayload {
             path: path.clone(),
             progress: 100,
             status: "Done".to_string(),
             speed: 0.0,
             bitrate_kbps: 0.0,
             output_info: None, // Or we could try to fetch it, but None implies no update to info
        });
    }

    Ok(())
}

#[tauri::command]
async fn clear_cancelled_paths(
    state: State<'_, ProcessingState>,
) -> Result<(), String> {
    if let Ok(mut set) = state.cancelled_paths.lock() {
        set.clear();
    }
    Ok(())
}

#[tauri::command]
async fn clear_crf_history(
    state: State<'_, ProcessingState>,
) -> Result<(), String> {
    if let Ok(mut v_state) = state.vmaf_state.lock() {
        v_state.crf_history.clear();
    }
    Ok(())
}

#[tauri::command]
async fn compute_vmaf(
    app: AppHandle,
    state: State<'_, ProcessingState>,
    input_path: String,
    output_path: String,
    config: video::CompressionConfig,
    duration_sec: f64
) -> Result<(), String> {
    let ffmpeg_path = resolve_ffmpeg_path(&app);
    let ffprobe_path = resolve_ffprobe_path(&app);

    // Fetch output info
    let output_video_info = video::get_metadata(&output_path, &ffprobe_path).ok();

    let task = video::VmafTask {
        app: app.clone(),
        input_path: input_path.clone(),
        ffmpeg_path,
        ffprobe_path,
        reference_path: input_path.clone(),
        distorted_path: output_path,
        config,
        duration_sec,
        pids: state.pids.clone(),
        cancelled_paths: state.cancelled_paths.clone(),
        output_video_info,
    };

    {
        let mut v_state = state.vmaf_state.lock().map_err(|e| e.to_string())?;
        v_state.queue.push_back(task);
    }

    let _ = app.emit("video-progress", video::ProgressPayload {
        path: input_path.clone(),
        progress: 100,
        status: "Waiting for VMAF".to_string(),
        speed: 0.0,
        bitrate_kbps: 0.0,
        output_info: None,
    });

    video::schedule_next_vmaf(state.vmaf_state.clone());
    Ok(())
}

#[tauri::command]
async fn run_crf_search_command(
    app: AppHandle,
    state: State<'_, ProcessingState>,
    input_path: String,
    config: video::CompressionConfig,
    duration_sec: f64
) -> Result<(f32, f64), String> {
    let ffmpeg_path = resolve_ffmpeg_path(&app);
    
    let pids = state.pids.clone();
    let cancelled_paths = state.cancelled_paths.clone();
    let vmaf_state = state.vmaf_state.clone();

    tauri::async_runtime::spawn_blocking(move || {
        video::run_crf_search(
            app,
            &ffmpeg_path,
            input_path,
            &config,
            duration_sec,
            pids,
            cancelled_paths,
            vmaf_state
        )
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_compression_command(
    app: AppHandle,
    state: State<'_, ProcessingState>,
    input_path: String,
    output_path: String,
    config: video::CompressionConfig,
    duration_sec: f64,
    vmaf_derived_crf: Option<f32>,
    vmaf_search_score: Option<f64>
) -> Result<(), String> {
    let ffmpeg_path = resolve_ffmpeg_path(&app);
    
    let pids = state.pids.clone();
    let cancelled_paths = state.cancelled_paths.clone();
    let vmaf_state = state.vmaf_state.clone();

    tauri::async_runtime::spawn_blocking(move || {
        video::run_ffmpeg_compression_task(
            app,
            &ffmpeg_path,
            input_path,
            output_path,
            config,
            duration_sec,
            pids,
            cancelled_paths,
            vmaf_state,
            vmaf_derived_crf,
            vmaf_search_score
        )
    }).await.map_err(|e| e.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(ProcessingState {
                pids: Arc::new(Mutex::new(HashMap::new())),
                cancelled_paths: Arc::new(Mutex::new(HashSet::new())),
                vmaf_state: Arc::new(Mutex::new(video::VmafState {
                    queue: std::collections::VecDeque::new(),
                    running_task: None,
                    crf_history: HashMap::new(),
                })),
            });

            let handle = app.handle().clone();
            
            // Start resource monitoring thread
            std::thread::spawn(move || {
                let mut sys = System::new_all();
                let nvml = Nvml::init().ok();
                
                loop {
                    sys.refresh_cpu_all();
                    sys.refresh_memory();
                    
                    let cpu_usage = sys.global_cpu_usage();
                    let total_mem = sys.total_memory() as f32;
                    let used_mem = sys.used_memory() as f32;
                    let memory_usage = (used_mem / total_mem) * 100.0;
                    
                    let mut graphics_usage = 0.0;
                    let mut encoder_usage = 0.0;
                    let mut decoder_usage = 0.0;
                    if let Some(ref n) = nvml {
                        if let Ok(device) = n.device_by_index(0) {
                            if let Ok(utilization) = device.utilization_rates() {
                                graphics_usage = utilization.gpu as f32;
                            }

                            if let Ok(enc) = device.encoder_utilization() {
                                encoder_usage = enc.utilization as f32;
                            }

                            if let Ok(dec) = device.decoder_utilization() {
                                decoder_usage = dec.utilization as f32;
                            }
                        }
                    }

                    // choose the maximum of graphics (3D), encoder and decoder utilizations
                    let gpu_usage = graphics_usage.max(encoder_usage).max(decoder_usage);

                    let stats = SystemStats {
                        cpu_usage,
                        memory_usage,
                        gpu_usage,
                    };
                    
                    let _ = handle.emit("system-stats", stats);
                    
                    std::thread::sleep(Duration::from_millis(1000));
                }
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_background_image,
            scan_directory,
            scan_multiple_paths,
            categorize_paths,
            get_video_metadata,
            detect_encoders,
            start_processing,
            cancel_processing,
            clear_cancelled_paths,
            clear_crf_history,
            compute_vmaf,
            run_crf_search_command,
            run_compression_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
