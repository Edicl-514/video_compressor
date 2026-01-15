mod video;

use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use sysinfo::{System};
use nvml_wrapper::Nvml;
use std::time::Duration;

#[derive(serde::Serialize, Clone)]
struct SystemStats {
    cpu_usage: f32,
    memory_usage: f32,
    gpu_usage: f32,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn scan_directory(path: String) -> Result<video::ScanResult, String> {
    Ok(video::scan_videos(&path))
}

#[tauri::command]
async fn get_video_metadata(path: String) -> Result<video::VideoInfo, String> {
    // Determine ffprobe path
    let ffprobe_rel = PathBuf::from("../ffmpeg/bin/ffprobe.exe");
    let ffprobe_path = if ffprobe_rel.exists() {
         std::fs::canonicalize(&ffprobe_rel).unwrap_or(ffprobe_rel)
    } else {
        let root_rel = PathBuf::from("ffmpeg/bin/ffprobe.exe");
        if root_rel.exists() {
            std::fs::canonicalize(&root_rel).unwrap_or(root_rel)
        } else {
            PathBuf::from("d:/code/video_compressor/ffmpeg/bin/ffprobe.exe")
        }
    };

    video::get_metadata(&path, ffprobe_path.to_str().unwrap_or(""))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
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
                    
                    let mut gpu_usage = 0.0; 
                    if let Some(ref n) = nvml {
                        if let Ok(device) = n.device_by_index(0) {
                            if let Ok(utilization) = device.utilization_rates() {
                                gpu_usage = utilization.gpu as f32;
                            }
                        }
                    }

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
        .invoke_handler(tauri::generate_handler![greet, scan_directory, get_video_metadata])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
