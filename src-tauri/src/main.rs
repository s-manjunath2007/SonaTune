// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use parking_lot::Mutex;
use sonatune::device;
use sonatune::dsp::DspConfig;
use sonatune::loopback::{LiveAudioRouter, LiveAudioStatus};
use sonatune::media::{
    detect_ffmpeg, inspect_audio_file, inspect_video_file, process_and_export_audio,
    process_and_remux_video, AudioMetadata, AudioProcessingResult, MediaToolInfo, VideoMetadata,
};
use sonatune::player::{AudioPlayer, PlayerState};
use sonatune::presets::{get_builtin_presets, Preset};
use sonatune::settings::{AppSettings, SettingsManager};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, State};

struct AppState {
    settings_mgr: SettingsManager,
    current_settings: Mutex<AppSettings>,
    player: AudioPlayer,
    router: LiveAudioRouter,
    audio_cancel_flag: Arc<AtomicBool>,
    video_cancel_flag: Arc<AtomicBool>,
}

#[tauri::command]
fn get_app_settings(state: State<'_, AppState>) -> AppSettings {
    state.current_settings.lock().clone()
}

#[tauri::command]
fn save_app_settings(new_settings: AppSettings, state: State<'_, AppState>) -> Result<AppSettings, String> {
    state.settings_mgr.save(&new_settings)?;
    state.player.update_dsp_config(new_settings.current_config.clone());
    state.router.update_dsp_config(new_settings.current_config.clone());
    *state.current_settings.lock() = new_settings.clone();
    Ok(new_settings)
}

#[tauri::command]
fn reset_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let defaults = state.settings_mgr.reset()?;
    state.player.update_dsp_config(defaults.current_config.clone());
    state.router.update_dsp_config(defaults.current_config.clone());
    *state.current_settings.lock() = defaults.clone();
    Ok(defaults)
}

#[tauri::command]
fn get_presets(state: State<'_, AppState>) -> Vec<Preset> {
    let mut list = get_builtin_presets();
    let settings = state.current_settings.lock();
    list.extend(settings.custom_presets.clone());
    list
}

#[tauri::command]
fn save_custom_preset(preset: Preset, state: State<'_, AppState>) -> Result<Vec<Preset>, String> {
    let mut settings = state.current_settings.lock().clone();
    settings.custom_presets.retain(|p| p.id != preset.id);
    settings.custom_presets.push(preset);
    state.settings_mgr.save(&settings)?;
    *state.current_settings.lock() = settings.clone();

    let mut list = get_builtin_presets();
    list.extend(settings.custom_presets);
    Ok(list)
}

#[tauri::command]
fn delete_custom_preset(preset_id: String, state: State<'_, AppState>) -> Result<Vec<Preset>, String> {
    let mut settings = state.current_settings.lock().clone();
    settings.custom_presets.retain(|p| p.id != preset_id);
    state.settings_mgr.save(&settings)?;
    *state.current_settings.lock() = settings.clone();

    let mut list = get_builtin_presets();
    list.extend(settings.custom_presets);
    Ok(list)
}

#[tauri::command]
fn get_audio_devices() -> Result<Vec<device::AudioDeviceInfo>, String> {
    device::get_audio_output_devices()
}

#[tauri::command]
fn get_default_device() -> Result<Option<device::AudioDeviceInfo>, String> {
    device::get_default_output_device()
}

#[tauri::command]
fn check_media_tools() -> MediaToolInfo {
    detect_ffmpeg()
}

#[tauri::command]
fn inspect_audio(file_path: String) -> Result<AudioMetadata, String> {
    inspect_audio_file(&file_path)
}

#[tauri::command]
fn process_audio_file(
    input_path: String,
    output_path: String,
    config: DspConfig,
    state: State<'_, AppState>,
) -> Result<AudioProcessingResult, String> {
    state.audio_cancel_flag.store(false, Ordering::SeqCst);
    let res = process_and_export_audio(
        &input_path,
        &output_path,
        &config,
        state.audio_cancel_flag.clone(),
    )?;

    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        let mut s = state.current_settings.lock().clone();
        s.last_export_dir = Some(parent.to_string_lossy().to_string());
        let _ = state.settings_mgr.save(&s);
        *state.current_settings.lock() = s;
    }

    Ok(res)
}

#[tauri::command]
fn cancel_audio_processing(state: State<'_, AppState>) {
    state.audio_cancel_flag.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn inspect_video(file_path: String) -> Result<VideoMetadata, String> {
    inspect_video_file(&file_path)
}

#[tauri::command]
fn process_video_file(
    source_video: String,
    output_video: String,
    audio_stream_index: usize,
    config: DspConfig,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.video_cancel_flag.store(false, Ordering::SeqCst);
    let res = process_and_remux_video(
        &source_video,
        &output_video,
        audio_stream_index,
        &config,
        state.video_cancel_flag.clone(),
    )?;

    if let Some(parent) = std::path::Path::new(&output_video).parent() {
        let mut s = state.current_settings.lock().clone();
        s.last_export_dir = Some(parent.to_string_lossy().to_string());
        let _ = state.settings_mgr.save(&s);
        *state.current_settings.lock() = s;
    }

    Ok(res)
}

#[tauri::command]
fn cancel_video_processing(state: State<'_, AppState>) {
    state.video_cancel_flag.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn player_load_file(path: String, state: State<'_, AppState>) -> Result<PlayerState, String> {
    let cfg = state.current_settings.lock().current_config.clone();
    state.player.update_dsp_config(cfg);
    state.player.load_file(&path)
}

#[tauri::command]
fn player_play(state: State<'_, AppState>) -> Result<PlayerState, String> {
    state.player.play()
}

#[tauri::command]
fn player_pause(state: State<'_, AppState>) -> PlayerState {
    state.player.pause()
}

#[tauri::command]
fn player_seek(seconds: f64, state: State<'_, AppState>) -> PlayerState {
    state.player.seek(seconds)
}

#[tauri::command]
fn player_get_state(state: State<'_, AppState>) -> PlayerState {
    state.player.get_state()
}

#[tauri::command]
fn player_set_ab_mode(mode: String, state: State<'_, AppState>) -> PlayerState {
    state.player.set_a_b_mode(&mode);
    state.player.get_state()
}

#[tauri::command]
fn player_set_loudness_match(enabled: bool, gain_factor: f32, state: State<'_, AppState>) -> PlayerState {
    state.player.set_loudness_matched(enabled);
    state.player.set_loudness_gain(gain_factor);
    state.player.get_state()
}

#[tauri::command]
fn get_live_audio_status(state: State<'_, AppState>) -> LiveAudioStatus {
    state.router.get_status()
}

#[tauri::command]
fn start_live_audio(
    input_device: Option<String>,
    output_device: Option<String>,
    state: State<'_, AppState>,
) -> Result<LiveAudioStatus, String> {
    let cfg = state.current_settings.lock().current_config.clone();
    state.router.update_dsp_config(cfg);
    state.router.start_route(input_device, output_device)
}

#[tauri::command]
fn stop_live_audio(state: State<'_, AppState>) -> LiveAudioStatus {
    state.router.stop_route();
    state.router.get_status()
}

fn main() {
    let settings_mgr = SettingsManager::new();
    let initial_settings = settings_mgr.load();

    let initial_config = initial_settings.current_config.clone();
    let router = LiveAudioRouter::new();
    router.update_dsp_config(initial_config.clone());

    let player = AudioPlayer::new();
    player.update_dsp_config(initial_config);

    let app_state = AppState {
        settings_mgr,
        current_settings: Mutex::new(initial_settings),
        player,
        router,
        audio_cancel_flag: Arc::new(AtomicBool::new(false)),
        video_cancel_flag: Arc::new(AtomicBool::new(false)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .setup(|app| {
            eprintln!("[SonaTune] Application setup starting...");
            // Build system tray menu
            let open_item = MenuItem::with_id(app, "open", "Open SonaTune", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Exit SonaTune", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

            let tray_result = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("SonaTune — Desktop Audio Enhancement")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app);

            match tray_result {
                Ok(_) => eprintln!("[SonaTune] Tray icon initialized successfully"),
                Err(e) => eprintln!("[SonaTune] Tray icon skipped or failed: {e}"),
            }

            eprintln!("[SonaTune] Application setup complete!");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_settings,
            save_app_settings,
            reset_app_settings,
            get_presets,
            save_custom_preset,
            delete_custom_preset,
            get_audio_devices,
            get_default_device,
            check_media_tools,
            inspect_audio,
            process_audio_file,
            cancel_audio_processing,
            inspect_video,
            process_video_file,
            cancel_video_processing,
            player_load_file,
            player_play,
            player_pause,
            player_seek,
            player_get_state,
            player_set_ab_mode,
            player_set_loudness_match,
            get_live_audio_status,
            start_live_audio,
            stop_live_audio,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SonaTune application");
}
