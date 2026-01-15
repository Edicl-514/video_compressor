mod video;

use std::path::PathBuf;
// use tauri::Manager; // Not strictly needed unless using app handle

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
        .invoke_handler(tauri::generate_handler![greet, scan_directory, get_video_metadata])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
