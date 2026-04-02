pub mod app_detector;
pub mod audio;
pub mod llm;
pub mod model_manager;
pub mod output;
pub mod pipeline;
pub mod storage;
pub mod stt;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_store::StoreExt;
use tracing_subscriber::EnvFilter;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Default cloud API base URL. Override with the `API_BASE_URL` environment variable.
pub const DEFAULT_API_BASE_URL: &str = "https://www.opentypeless.com";

/// Read the cloud API base URL from the environment, falling back to the compiled default.
pub fn api_base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string())
}

/// State for Fn/Globe key hotkey (captured via CGEventTap, not tauri-plugin-global-shortcut).
struct FnHotkeyState {
    /// Whether the Fn hotkey is currently active.
    enabled: Arc<AtomicBool>,
    /// True while the user is recording a new hotkey in Settings — tap emits
    /// `hotkey:fn_detected` instead of driving the pipeline.
    recording: Arc<AtomicBool>,
}

/// Cached close_to_tray setting to avoid blocking I/O in the window close handler.
struct CloseToTrayCache(Arc<Mutex<bool>>);

/// Current translate hotkey string (empty = disabled). Updated whenever the user changes it.
struct TranslateHotkeyCache(Arc<Mutex<String>>);

/// Session token for cloud providers. Set by the frontend after Better Auth login.
/// The Rust pipeline reads this when creating cloud STT/LLM providers.
pub struct SessionTokenStore(pub Arc<Mutex<String>>);

/// Managed tray icon handle for dynamic menu/tooltip updates.
pub struct TrayHandle {
    pub tray: Mutex<tauri::tray::TrayIcon>,
}

/// Persisted window position and size.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct WindowState {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Build (or rebuild) the system tray menu based on current state.
fn build_tray_menu(
    app: &tauri::AppHandle,
    is_recording: bool,
    window_visible: bool,
) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let show_hide = MenuItem::with_id(
        app,
        "show_hide",
        if window_visible {
            "Hide Window"
        } else {
            "Show Window"
        },
        true,
        None::<&str>,
    )?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let record = MenuItem::with_id(
        app,
        "record",
        if is_recording {
            "Stop Recording"
        } else {
            "Start Recording"
        },
        true,
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let history = MenuItem::with_id(app, "history", "History", true, None::<&str>)?;
    let account = MenuItem::with_id(app, "account", "Account", true, None::<&str>)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let about = MenuItem::with_id(app, "about", "About ChangYan", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show_hide, &sep1, &record, &sep2, &settings, &history, &account, &sep3, &about, &quit,
        ],
    )?;
    Ok(menu)
}

/// Rebuild the tray menu and update tooltip based on pipeline state.
pub fn refresh_tray(app: &tauri::AppHandle) {
    let is_recording = app
        .try_state::<pipeline::PipelineHandle>()
        .map(|p| p.current_state() == pipeline::PipelineState::Recording)
        .unwrap_or(false);
    let window_visible = app
        .get_webview_window("main")
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    if let Some(tray_handle) = app.try_state::<TrayHandle>() {
        if let Ok(tray) = tray_handle.tray.lock() {
            if let Ok(menu) = build_tray_menu(app, is_recording, window_visible) {
                let _ = tray.set_menu(Some(menu));
            }
        }
    }
}

#[tauri::command]
async fn start_recording(state: tauri::State<'_, pipeline::PipelineHandle>) -> Result<(), String> {
    state.start().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_recording(state: tauri::State<'_, pipeline::PipelineHandle>) -> Result<(), String> {
    state.stop().await.map_err(|e| e.to_string())
}

/// Force the pipeline back to Idle from any state, cancelling any in-progress operation.
#[tauri::command]
fn force_idle(state: tauri::State<'_, pipeline::PipelineHandle>) {
    state.force_idle();
}

/// Start recording in translate mode. Sets translate_session flag so the LLM polishes with translation.
#[tauri::command]
async fn start_recording_translate(
    state: tauri::State<'_, pipeline::PipelineHandle>,
) -> Result<(), String> {
    state
        .translate_session
        .store(true, std::sync::atomic::Ordering::Relaxed);
    state.start().await.map_err(|e| e.to_string())
}

/// Debug: read back capsule window level, collection behavior, position and size at runtime.
#[cfg(target_os = "macos")]
#[tauri::command]
fn debug_capsule_window(app: tauri::AppHandle) -> String {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};
    let Some(capsule) = app.get_webview_window("capsule") else {
        let msg = "capsule window not found".to_string();
        tracing::warn!("[Capsule debug] {}", msg);
        return msg;
    };
    let pos = capsule.outer_position().ok();
    let size = capsule.outer_size().ok();
    let visible = capsule.is_visible().unwrap_or(false);
    let Ok(ns_ptr) = capsule.ns_window() else {
        let msg = format!("ns_window() failed — pos={pos:?} size={size:?} visible={visible}");
        tracing::warn!("[Capsule debug] {}", msg);
        return msg;
    };
    unsafe {
        let ns_window = &*(ns_ptr as *const NSWindow);
        let level = ns_window.level();
        let behavior = ns_window.collectionBehavior();
        let can_join = behavior.contains(NSWindowCollectionBehavior::CanJoinAllSpaces);
        let fs_aux = behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary);
        let msg = format!(
            "level={level} canJoinAllSpaces={can_join} fullScreenAuxiliary={fs_aux} \
             visible={visible} pos={pos:?} size={size:?}"
        );
        tracing::info!("[Capsule debug] {}", msg);
        msg
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn debug_capsule_window(_app: tauri::AppHandle) -> String {
    "debug_capsule_window: macOS only".to_string()
}

/// Shared implementation: re-apply window level/behavior and call orderFrontRegardless.
/// Force the window onto the currently active Space (including fullscreen Spaces)
/// using CoreGraphics Session private APIs. This operates at the compositor level,
/// below AppKit, and is how apps like Yabai/Rectangle handle fullscreen overlays.
#[cfg(target_os = "macos")]
unsafe fn cgs_move_to_active_space(ns_window: &objc2_app_kit::NSWindow) {
    extern "C" {
        fn _CGSDefaultConnection() -> i32;
        fn CGSGetActiveSpace(cid: i32) -> u64;
        fn CGSAddWindowsToSpaces(
            cid: i32,
            window_array: *const std::ffi::c_void,
            space_array: *const std::ffi::c_void,
        );
        fn CGSSetWindowTags(cid: i32, wid: i32, tags: *const i32, tag_size: i32) -> i32;
    }

    // CoreFoundation FFI (toll-free bridged with NS types)
    #[allow(non_upper_case_globals)]
    const kCFNumberSInt32Type: isize = 3;
    #[allow(non_upper_case_globals)]
    const kCFNumberSInt64Type: isize = 4;
    extern "C" {
        fn CFNumberCreate(
            allocator: *const std::ffi::c_void,
            the_type: isize,
            value_ptr: *const std::ffi::c_void,
        ) -> *const std::ffi::c_void;
        fn CFArrayCreate(
            allocator: *const std::ffi::c_void,
            values: *const *const std::ffi::c_void,
            num_values: isize,
            callbacks: *const std::ffi::c_void,
        ) -> *const std::ffi::c_void;
        fn CFRelease(cf: *const std::ffi::c_void);
        static kCFTypeArrayCallBacks: std::ffi::c_void;
    }

    let cid = _CGSDefaultConnection();
    let wid = ns_window.windowNumber() as i32;
    let active_space = CGSGetActiveSpace(cid);

    tracing::info!("[Capsule] CGS: cid={cid} wid={wid} activeSpace={active_space}");

    // 1) Set sticky tags (both bit 1 = 0x2 and bit 11 = 0x800)
    let tags: [i32; 2] = [(1 << 1) | (1 << 11), 0]; // 0x802
    let tag_result = CGSSetWindowTags(cid, wid, tags.as_ptr(), 32);
    tracing::info!("[Capsule] CGSSetWindowTags(0x802) result={tag_result}");

    // 2) Explicitly add window to the active space
    if active_space != 0 {
        let wid_ref = CFNumberCreate(
            std::ptr::null(),
            kCFNumberSInt32Type,
            &wid as *const i32 as *const _,
        );
        let sid_ref = CFNumberCreate(
            std::ptr::null(),
            kCFNumberSInt64Type,
            &active_space as *const u64 as *const _,
        );
        if !wid_ref.is_null() && !sid_ref.is_null() {
            let wid_arr = CFArrayCreate(
                std::ptr::null(),
                &wid_ref as *const *const _,
                1,
                &kCFTypeArrayCallBacks as *const _,
            );
            let sid_arr = CFArrayCreate(
                std::ptr::null(),
                &sid_ref as *const *const _,
                1,
                &kCFTypeArrayCallBacks as *const _,
            );
            if !wid_arr.is_null() && !sid_arr.is_null() {
                CGSAddWindowsToSpaces(cid, wid_arr, sid_arr);
                tracing::info!("[Capsule] CGSAddWindowsToSpaces done (space={active_space})");
                CFRelease(wid_arr);
                CFRelease(sid_arr);
            }
            CFRelease(wid_ref);
            CFRelease(sid_ref);
        }
    }
}

/// Safe to call from any thread — schedules on the main thread internally.
#[cfg(target_os = "macos")]
pub(crate) fn raise_capsule_window(app: &tauri::AppHandle) {
    let app2 = app.clone();
    app.run_on_main_thread(move || {
        // Guard: don't show the capsule if the pipeline has already returned to idle.
        // This prevents a race where a queued raise executes after hide().
        if let Some(pipeline) = app2.try_state::<crate::pipeline::PipelineHandle>() {
            if pipeline.current_state() == crate::pipeline::PipelineState::Idle {
                tracing::info!("[Capsule] raise skipped — pipeline is idle");
                return;
            }
        }

        use objc2_app_kit::{NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior};
        let Some(capsule) = app2.get_webview_window("capsule") else {
            return;
        };
        let Ok(ns_ptr) = capsule.ns_window() else {
            return;
        };
        unsafe {
            let ns_window = &*(ns_ptr as *const NSWindow);
            let before_level = ns_window.level();
            let before_behavior = ns_window.collectionBehavior();
            let is_visible = ns_window.isVisible();
            let is_on_active_space = ns_window.isOnActiveSpace();
            tracing::info!(
                "[Capsule] raise BEFORE: level={} behavior={:?} isVisible={} isOnActiveSpace={}",
                before_level,
                before_behavior,
                is_visible,
                is_on_active_space
            );
            let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle;
            ns_window.setCollectionBehavior(behavior);
            ns_window.setLevel(NSScreenSaverWindowLevel);
            crate::cgs_move_to_active_space(ns_window);
            ns_window.orderFrontRegardless();
            tracing::info!(
                "[Capsule] raise AFTER:  level={} behavior={:?} isVisible={} isOnActiveSpace={}",
                ns_window.level(),
                ns_window.collectionBehavior(),
                ns_window.isVisible(),
                ns_window.isOnActiveSpace()
            );
        }
    })
    .ok();
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn raise_capsule_window(_app: &tauri::AppHandle) {}

/// Register an observer for NSWorkspaceActiveSpaceDidChangeNotification so the capsule
/// is re-raised automatically whenever Mission Control changes the active space
/// (e.g., when another app enters or exits native fullscreen).
/// The observer is intentionally leaked so it stays active for the app's entire lifetime.
#[cfg(target_os = "macos")]
fn register_space_observer(app: &tauri::App) {
    use block2::RcBlock;
    use objc2_app_kit::{
        NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWorkspace,
    };
    use objc2_foundation::NSString;

    let app_handle = app.handle().clone();

    app.run_on_main_thread(move || unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let nc = workspace.notificationCenter();
        let name = NSString::from_str("NSWorkspaceActiveSpaceDidChangeNotification");

        let app_h = app_handle.clone();
        let block = RcBlock::new(
            move |_: std::ptr::NonNull<objc2_foundation::NSNotification>| {
                tracing::info!("[Capsule] NSWorkspaceActiveSpaceDidChangeNotification fired");

                // Only re-raise capsule when pipeline is active (not idle).
                // Otherwise the hidden capsule would flash into view on every space switch.
                if let Some(pipeline) = app_h.try_state::<crate::pipeline::PipelineHandle>() {
                    if pipeline.current_state() == crate::pipeline::PipelineState::Idle {
                        tracing::info!("[Capsule] space-observer: pipeline idle, skipping raise");
                        return;
                    }
                }

                if let Some(capsule) = app_h.get_webview_window("capsule") {
                    if let Ok(ns_ptr) = capsule.ns_window() {
                        let ns_window = &*(ns_ptr as *const NSWindow);
                        let before_level = ns_window.level();
                        let before_behavior = ns_window.collectionBehavior();
                        let is_visible = ns_window.isVisible();
                        let is_on_active_space = ns_window.isOnActiveSpace();
                        tracing::info!(
                            "[Capsule] space-observer BEFORE: level={} behavior={:?} isVisible={} isOnActiveSpace={}",
                            before_level, before_behavior, is_visible, is_on_active_space
                        );
                        let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
                            | NSWindowCollectionBehavior::FullScreenAuxiliary
                            | NSWindowCollectionBehavior::IgnoresCycle;
                        ns_window.setCollectionBehavior(behavior);
                        ns_window.setLevel(NSScreenSaverWindowLevel);
                        crate::cgs_move_to_active_space(ns_window);
                        ns_window.orderFrontRegardless();
                        tracing::info!(
                            "[Capsule] space-observer AFTER:  level={} behavior={:?} isVisible={} isOnActiveSpace={}",
                            ns_window.level(), ns_window.collectionBehavior(), ns_window.isVisible(), ns_window.isOnActiveSpace()
                        );
                    }
                } else {
                    tracing::warn!("[Capsule] space-observer: capsule window not found");
                }
            },
        );

        // queue=None: notification delivered on the posting thread.
        // NSWorkspaceActiveSpaceDidChangeNotification is always posted on the main thread,
        // so NSWindow calls inside the block are safe.
        let observer = nc.addObserverForName_object_queue_usingBlock(
            Some(&name),
            None,
            None,
            &block,
        );

        // Intentionally leak the observer — it stays registered for the app's lifetime.
        std::mem::forget(observer);
    })
    .ok();
}

/// Force the capsule window above fullscreen apps by re-applying level and calling orderFrontRegardless.
#[cfg(target_os = "macos")]
#[tauri::command]
fn bring_capsule_to_front(app: tauri::AppHandle) {
    raise_capsule_window(&app);
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
fn bring_capsule_to_front(app: tauri::AppHandle) {
    if let Some(capsule) = app.get_webview_window("capsule") {
        let _ = capsule.show();
    }
}

#[tauri::command]
async fn get_config(
    state: tauri::State<'_, storage::ConfigManager>,
) -> Result<storage::AppConfig, String> {
    state.load().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_config(
    state: tauri::State<'_, storage::ConfigManager>,
    close_tray_cache: tauri::State<'_, CloseToTrayCache>,
    config: storage::AppConfig,
) -> Result<(), String> {
    *close_tray_cache.0.lock().unwrap_or_else(|e| e.into_inner()) = config.close_to_tray;
    state.save(&config).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_stt_connection(
    api_key: String,
    provider: String,
    token_store: tauri::State<'_, SessionTokenStore>,
) -> Result<bool, String> {
    if provider.is_empty() {
        return Ok(false);
    }

    // Cloud provider: verify session token + Pro status via API
    if provider == "cloud" {
        let token = token_store
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if token.is_empty() {
            return Ok(false);
        }
        let api_base = api_base_url();
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/api/subscription/status", api_base))
            .header("Authorization", format!("Bearer {}", token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Ok(false);
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        return Ok(body["plan"].as_str() == Some("pro"));
    }

    if api_key.is_empty() {
        return Ok(false);
    }

    match provider.as_str() {
        "deepgram" => {
            let client = reqwest::Client::new();
            let resp = client
                .get("https://api.deepgram.com/v1/projects")
                .header("Authorization", format!("Token {}", api_key))
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(resp.status().is_success())
        }
        "assemblyai" => {
            let client = reqwest::Client::new();
            let resp = client
                .get("https://api.assemblyai.com/v2/transcript?limit=1")
                .header("Authorization", api_key)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(resp.status().is_success())
        }
        "glm-asr" | "openai-whisper" | "groq-whisper" | "siliconflow" => {
            // All four use Whisper-compatible file upload API
            let (endpoint, model, extra_fields): (&str, &str, &[(&str, &str)]) =
                match provider.as_str() {
                    "glm-asr" => (
                        "https://open.bigmodel.cn/api/paas/v4/audio/transcriptions",
                        "glm-asr-2512",
                        &[("stream", "false")][..],
                    ),
                    "openai-whisper" => (
                        "https://api.openai.com/v1/audio/transcriptions",
                        "whisper-1",
                        &[][..],
                    ),
                    "groq-whisper" => (
                        "https://api.groq.com/openai/v1/audio/transcriptions",
                        "whisper-large-v3-turbo",
                        &[][..],
                    ),
                    _ => (
                        "https://api.siliconflow.cn/v1/audio/transcriptions",
                        "FunAudioLLM/SenseVoiceSmall",
                        &[][..],
                    ),
                };

            let silent_pcm = vec![0u8; 3200]; // 0.1s at 16kHz 16-bit mono
            let wav = stt::whisper_compat::WhisperCompatProvider::build_wav(&silent_pcm, 16000);

            let file_part = reqwest::multipart::Part::bytes(wav)
                .file_name("test.wav")
                .mime_str("audio/wav")
                .map_err(|e| e.to_string())?;
            let mut form = reqwest::multipart::Form::new()
                .text("model", model.to_string())
                .part("file", file_part);
            for &(key, value) in extra_fields {
                form = form.text(key.to_string(), value.to_string());
            }

            let client = reqwest::Client::new();
            let resp = client
                .post(endpoint)
                .header("Authorization", format!("Bearer {}", api_key))
                .multipart(form)
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            Ok(resp.status().is_success())
        }
        _ => Err(format!("Unknown STT provider: {}", provider)),
    }
}

#[tauri::command]
async fn test_llm_connection(
    api_key: String,
    provider: String,
    base_url: String,
    model: String,
    token_store: tauri::State<'_, SessionTokenStore>,
) -> Result<bool, String> {
    if provider.is_empty() {
        return Ok(false);
    }

    // Cloud provider: verify session token + Pro status via API
    if provider == "cloud" {
        let token = token_store
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if token.is_empty() {
            return Ok(false);
        }
        let api_base = api_base_url();
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/api/subscription/status", api_base))
            .header("Authorization", format!("Bearer {}", token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Ok(false);
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        return Ok(body["plan"].as_str() == Some("pro"));
    }

    if api_key.is_empty() || base_url.is_empty() {
        return Ok(false);
    }

    // Validate base_url is a proper HTTP(S) URL
    let parsed = url::Url::parse(&base_url).map_err(|e| format!("Invalid base URL: {e}"))?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err("Base URL must use http or https scheme".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "hi"}],
        "max_tokens": 1
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(resp.status().is_success())
}

#[tauri::command]
async fn fetch_llm_models(api_key: String, base_url: String) -> Result<Vec<String>, String> {
    if base_url.is_empty() {
        return Ok(vec![]);
    }

    // Validate base_url is a proper HTTP(S) URL
    let parsed = url::Url::parse(&base_url).map_err(|e| format!("Invalid base URL: {e}"))?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err("Base URL must use http or https scheme".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/models", base_url.trim_end_matches('/'));

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Ok(vec![]);
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    // OpenAI-compatible: { data: [{ id: "model-name" }] }
    // Ollama-compatible: { models: [{ name: "model-name" }] }
    let mut models: Vec<String> = Vec::new();

    if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                models.push(id.to_string());
            }
        }
    } else if let Some(data) = body.get("models").and_then(|d| d.as_array()) {
        for item in data {
            if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                models.push(name.to_string());
            }
        }
    }

    models.sort();
    Ok(models)
}

#[tauri::command]
async fn bench_stt_connection(
    api_key: String,
    provider: String,
    token_store: tauri::State<'_, SessionTokenStore>,
) -> Result<u32, String> {
    if provider.is_empty() {
        return Err("No provider specified".to_string());
    }

    if provider == "cloud" {
        let token = token_store
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if token.is_empty() {
            return Err("Not signed in".to_string());
        }
        let api_base = api_base_url();
        let client = reqwest::Client::new();
        let t0 = std::time::Instant::now();
        let resp = client
            .get(format!("{}/api/subscription/status", api_base))
            .header("Authorization", format!("Bearer {}", token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let elapsed = t0.elapsed().as_millis() as u32;
        if !resp.status().is_success() {
            return Err("Request failed".to_string());
        }
        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        if body["plan"].as_str() != Some("pro") {
            return Err("Pro plan required".to_string());
        }
        return Ok(elapsed);
    }

    if api_key.is_empty() {
        return Err("API key is empty".to_string());
    }

    match provider.as_str() {
        "deepgram" => {
            let client = reqwest::Client::new();
            let t0 = std::time::Instant::now();
            let resp = client
                .get("https://api.deepgram.com/v1/projects")
                .header("Authorization", format!("Token {}", api_key))
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let elapsed = t0.elapsed().as_millis() as u32;
            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()));
            }
            Ok(elapsed)
        }
        "assemblyai" => {
            let client = reqwest::Client::new();
            let t0 = std::time::Instant::now();
            let resp = client
                .get("https://api.assemblyai.com/v2/transcript?limit=1")
                .header("Authorization", api_key)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let elapsed = t0.elapsed().as_millis() as u32;
            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()));
            }
            Ok(elapsed)
        }
        "glm-asr" | "openai-whisper" | "groq-whisper" | "siliconflow" => {
            let (endpoint, model, extra_fields): (&str, &str, &[(&str, &str)]) =
                match provider.as_str() {
                    "glm-asr" => (
                        "https://open.bigmodel.cn/api/paas/v4/audio/transcriptions",
                        "glm-asr-2512",
                        &[("stream", "false")][..],
                    ),
                    "openai-whisper" => (
                        "https://api.openai.com/v1/audio/transcriptions",
                        "whisper-1",
                        &[][..],
                    ),
                    "groq-whisper" => (
                        "https://api.groq.com/openai/v1/audio/transcriptions",
                        "whisper-large-v3-turbo",
                        &[][..],
                    ),
                    _ => (
                        "https://api.siliconflow.cn/v1/audio/transcriptions",
                        "FunAudioLLM/SenseVoiceSmall",
                        &[][..],
                    ),
                };

            let silent_pcm = vec![0u8; 3200]; // 0.1s at 16kHz 16-bit mono
            let wav = stt::whisper_compat::WhisperCompatProvider::build_wav(&silent_pcm, 16000);

            let file_part = reqwest::multipart::Part::bytes(wav)
                .file_name("test.wav")
                .mime_str("audio/wav")
                .map_err(|e| e.to_string())?;
            let mut form = reqwest::multipart::Form::new()
                .text("model", model.to_string())
                .part("file", file_part);
            for &(key, value) in extra_fields {
                form = form.text(key.to_string(), value.to_string());
            }

            let client = reqwest::Client::new();
            let t0 = std::time::Instant::now();
            let resp = client
                .post(endpoint)
                .header("Authorization", format!("Bearer {}", api_key))
                .multipart(form)
                .timeout(std::time::Duration::from_secs(15))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let elapsed = t0.elapsed().as_millis() as u32;
            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()));
            }
            Ok(elapsed)
        }
        _ => Err(format!("Unknown STT provider: {}", provider)),
    }
}

#[tauri::command]
async fn bench_llm_connection(
    api_key: String,
    provider: String,
    base_url: String,
    model: String,
    token_store: tauri::State<'_, SessionTokenStore>,
) -> Result<u32, String> {
    if provider.is_empty() {
        return Err("No provider specified".to_string());
    }

    if provider == "cloud" {
        let token = token_store
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if token.is_empty() {
            return Err("Not signed in".to_string());
        }
        let api_base = api_base_url();
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "messages": [{"role": "user", "content": "hi"}],
            "stream": false
        });
        let t0 = std::time::Instant::now();
        let resp = client
            .post(format!("{}/api/proxy/llm", api_base))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let elapsed = t0.elapsed().as_millis() as u32;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        return Ok(elapsed);
    }

    if api_key.is_empty() || base_url.is_empty() {
        return Err("API key or base URL is empty".to_string());
    }

    let parsed = url::Url::parse(&base_url).map_err(|e| format!("Invalid base URL: {e}"))?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return Err("Base URL must use http or https scheme".to_string());
    }

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "hi"}],
        "max_tokens": 1
    });

    let t0 = std::time::Instant::now();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let elapsed = t0.elapsed().as_millis() as u32;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    Ok(elapsed)
}

#[tauri::command]
async fn get_history(
    state: tauri::State<'_, storage::HistoryStore>,
    limit: u32,
    offset: u32,
) -> Result<Vec<storage::HistoryEntry>, String> {
    state.list(limit, offset).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_history(state: tauri::State<'_, storage::HistoryStore>) -> Result<(), String> {
    state.clear().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_dictionary(
    state: tauri::State<'_, storage::DictionaryStore>,
) -> Result<Vec<storage::DictionaryEntry>, String> {
    state.list().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_dictionary_entry(
    state: tauri::State<'_, storage::DictionaryStore>,
    word: String,
    pronunciation: Option<String>,
) -> Result<(), String> {
    let word = word.trim().to_string();
    if word.is_empty() {
        return Err("Word cannot be empty".to_string());
    }
    if word.len() > 100 {
        return Err("Word is too long (max 100 characters)".to_string());
    }
    if let Some(ref p) = pronunciation {
        if p.len() > 100 {
            return Err("Pronunciation is too long (max 100 characters)".to_string());
        }
    }
    state
        .add(&word, pronunciation.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_dictionary_entry(
    state: tauri::State<'_, storage::DictionaryStore>,
    id: i64,
) -> Result<(), String> {
    state.remove(id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_session_token(
    state: tauri::State<'_, SessionTokenStore>,
    token: String,
) -> Result<(), String> {
    *state.0.lock().unwrap_or_else(|e| e.into_inner()) = token;
    Ok(())
}

/// Checks whether Accessibility permission is granted for the current process.
#[tauri::command]
fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> u8;
        }
        unsafe { AXIsProcessTrusted() != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[tauri::command]
fn open_accessibility_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .spawn();
    }
}

#[tauri::command]
async fn set_auto_start(
    app: tauri::AppHandle,
    config_state: tauri::State<'_, storage::ConfigManager>,
    enabled: bool,
) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(|e| e.to_string())?;
    } else {
        autolaunch.disable().map_err(|e| e.to_string())?;
    }
    let mut config = config_state.load().await.map_err(|e| e.to_string())?;
    config.auto_start = enabled;
    config_state
        .save(&config)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn update_hotkey(
    app: tauri::AppHandle,
    config_state: tauri::State<'_, storage::ConfigManager>,
    hotkey: String,
) -> Result<(), String> {
    let fn_state = app.state::<FnHotkeyState>();
    // Recording is always done by the time update_hotkey is called
    fn_state.recording.store(false, Ordering::Relaxed);

    if hotkey == "Fn" {
        fn_state.enabled.store(true, Ordering::Relaxed);
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| e.to_string())?;
    } else {
        fn_state.enabled.store(false, Ordering::Relaxed);
        let new_shortcut =
            parse_hotkey(&hotkey).ok_or_else(|| format!("Invalid hotkey: {}", hotkey))?;
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| e.to_string())?;
        app.global_shortcut()
            .register(new_shortcut)
            .map_err(|e| e.to_string())?;
    }

    // Save updated hotkey to config
    let mut config = config_state.load().await.map_err(|e| e.to_string())?;
    config.hotkey = hotkey;
    config_state
        .save(&config)
        .await
        .map_err(|e| e.to_string())?;

    // Re-register translate hotkey if configured
    if !config.translate_hotkey.is_empty() {
        if let Some(t_shortcut) = parse_hotkey(&config.translate_hotkey) {
            let _ = app.global_shortcut().register(t_shortcut);
        }
    }

    Ok(())
}

/// Temporarily unregister all global shortcuts so the webview can capture key events.
/// Also sets recording=true so the CGEventTap forwards Fn/Globe key to the frontend.
#[tauri::command]
fn pause_hotkey(app: tauri::AppHandle) -> Result<(), String> {
    let fn_state = app.state::<FnHotkeyState>();
    fn_state.enabled.store(false, Ordering::Relaxed);
    fn_state.recording.store(true, Ordering::Relaxed);
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())
}

/// Register or update the translate hotkey without affecting the main hotkey.
#[tauri::command]
async fn update_translate_hotkey(
    app: tauri::AppHandle,
    config_state: tauri::State<'_, storage::ConfigManager>,
    translate_cache: tauri::State<'_, TranslateHotkeyCache>,
    hotkey: String,
) -> Result<(), String> {
    let fn_state = app.state::<FnHotkeyState>();
    fn_state.recording.store(false, Ordering::Relaxed);

    // Unregister all, then re-register main hotkey + new translate hotkey
    let _ = app.global_shortcut().unregister_all();

    let mut config = config_state.load().await.map_err(|e| e.to_string())?;

    // Re-register main hotkey
    if config.hotkey == "Fn" {
        fn_state.enabled.store(true, Ordering::Relaxed);
    } else {
        let main_shortcut = parse_hotkey(&config.hotkey).unwrap_or_else(default_shortcut);
        let _ = app.global_shortcut().register(main_shortcut);
    }

    // Register new translate hotkey (empty = clear)
    if !hotkey.is_empty() {
        let t_shortcut =
            parse_hotkey(&hotkey).ok_or_else(|| format!("Invalid hotkey: {}", hotkey))?;
        app.global_shortcut()
            .register(t_shortcut)
            .map_err(|e| e.to_string())?;
    }

    // Update in-memory cache so the shortcut handler can identify translate events
    *translate_cache.0.lock().unwrap_or_else(|e| e.into_inner()) = hotkey.clone();

    config.translate_hotkey = hotkey;
    config_state.save(&config).await.map_err(|e| e.to_string())
}

/// Re-register the current hotkey from config after recording is done.
/// Clears recording=false so the CGEventTap resumes normal pipeline control.
#[tauri::command]
async fn resume_hotkey(
    app: tauri::AppHandle,
    config_state: tauri::State<'_, storage::ConfigManager>,
) -> Result<(), String> {
    let config = config_state.load().await.map_err(|e| e.to_string())?;
    let fn_state = app.state::<FnHotkeyState>();
    fn_state.recording.store(false, Ordering::Relaxed);

    if config.hotkey == "Fn" {
        fn_state.enabled.store(true, Ordering::Relaxed);
    } else {
        fn_state.enabled.store(false, Ordering::Relaxed);
        let shortcut = parse_hotkey(&config.hotkey).unwrap_or_else(default_shortcut);
        // Ensure clean state, then register
        let _ = app.global_shortcut().unregister_all();
        app.global_shortcut()
            .register(shortcut)
            .map_err(|e| e.to_string())?;
    }

    // Re-register translate hotkey if configured
    if !config.translate_hotkey.is_empty() {
        if let Some(t_shortcut) = parse_hotkey(&config.translate_hotkey) {
            let _ = app.global_shortcut().register(t_shortcut);
        }
    }

    Ok(())
}

// ─── Hotkey parsing ───

fn default_shortcut() -> Shortcut {
    let default_hotkey = storage::AppConfig::default().hotkey;
    parse_hotkey(&default_hotkey)
        .unwrap_or_else(|| Shortcut::new(Some(Modifiers::CONTROL), Code::Slash))
}

/// Register a CGEventTap on the main RunLoop for Fn/Globe key detection.
///
/// **Why not rdev?** `rdev::listen` runs a CGEventTap on a background thread and calls
/// `TSMGetInputSourceProperty` (via its key-to-string conversion) for every event. That
/// function asserts it's on the main thread — causing an immediate crash on any keypress.
///
/// Our tap is listen-only and calls only `CGEventGetIntegerValueField` /
/// `CGEventGetFlags`, which are thread-safe CoreGraphics reads with no TSM dependency.
/// By adding the RunLoop source to the **main** RunLoop the callback also runs on the
/// main thread, satisfying any implicit threading requirements.
#[cfg(target_os = "macos")]
fn register_fn_key_tap(
    app_handle: tauri::AppHandle,
    enabled: Arc<AtomicBool>,
    recording: Arc<AtomicBool>,
) {
    use std::ffi::c_void;

    // ── CoreGraphics / CoreFoundation FFI ────────────────────────────────────
    #[allow(non_camel_case_types)]
    type CGEventRef = *mut c_void;
    #[allow(non_camel_case_types)]
    type CGEventTapProxy = *mut c_void;
    #[allow(non_camel_case_types)]
    type CFMachPortRef = *mut c_void;
    #[allow(non_camel_case_types)]
    type CFRunLoopSourceRef = *mut c_void;
    #[allow(non_camel_case_types)]
    type CFRunLoopRef = *mut c_void;
    #[allow(non_camel_case_types)]
    type CFStringRef = *const c_void;

    #[allow(non_camel_case_types)]
    type CGEventTapCallBack = unsafe extern "C" fn(
        proxy: CGEventTapProxy,
        event_type: u32,
        event: CGEventRef,
        user_info: *mut c_void,
    ) -> CGEventRef;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: CGEventTapCallBack,
            user_info: *mut c_void,
        ) -> CFMachPortRef;
        fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
        fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
        fn CGEventGetFlags(event: CGEventRef) -> u64;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFMachPortCreateRunLoopSource(
            allocator: *const c_void,
            port: CFMachPortRef,
            order: isize,
        ) -> CFRunLoopSourceRef;
        fn CFRunLoopGetMain() -> CFRunLoopRef;
        fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
        static kCFRunLoopCommonModes: CFStringRef;
    }

    // CGEventType constants
    const CG_EVENT_KEY_DOWN: u32 = 10;
    const CG_EVENT_KEY_UP: u32 = 11;
    const CG_EVENT_FLAGS_CHANGED: u32 = 12;
    // CGEventField: kCGKeyboardEventKeycode = 9
    const CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;
    // kCGEventFlagMaskSecondaryFn = Fn modifier bit
    const CG_FLAG_FN: u64 = 0x0080_0000;
    // kVK_Function = 63 (Intel/older), Globe key = 179 (Apple Silicon)
    const FN_KEYCODE: i64 = 63;
    const GLOBE_KEYCODE: i64 = 179;
    // Tap config: kCGSessionEventTap=1, kCGHeadInsertEventTap=0, kCGEventTapOptionListenOnly=1
    const TAP_TYPE: u32 = 1;
    const TAP_PLACE: u32 = 0;
    const TAP_LISTEN_ONLY: u32 = 1;
    let event_mask: u64 =
        (1u64 << CG_EVENT_KEY_DOWN) | (1u64 << CG_EVENT_KEY_UP) | (1u64 << CG_EVENT_FLAGS_CHANGED);

    // ── Processing thread ────────────────────────────────────────────────────
    // Fn/Globe key ALWAYS uses toggle semantics regardless of hotkey_mode:
    // - tap when idle   → start recording
    // - tap when recording → stop (finalize)
    // - tap when stuck  → force cancel back to idle
    // Releases are ignored entirely (Fn is a tap key, not a hold key).
    let (tx, rx) = std::sync::mpsc::channel::<bool>();
    std::thread::spawn({
        let app_handle = app_handle.clone();
        move || {
            for is_press in rx {
                if !is_press {
                    continue; // ignore Fn key releases
                }
                let h = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let pipeline = h.state::<pipeline::PipelineHandle>();
                    match pipeline.current_state() {
                        pipeline::PipelineState::Idle => {
                            if let Err(e) = pipeline.start().await {
                                tracing::error!("Fn key start: {e}");
                                let _ = h.emit("pipeline:error", e.to_string());
                            }
                        }
                        pipeline::PipelineState::Recording => {
                            if let Err(e) = pipeline.stop().await {
                                tracing::error!("Fn key stop: {e}");
                                let _ = h.emit("pipeline:error", e.to_string());
                            }
                        }
                        _ => {
                            // Stuck in Transcribing/Polishing/Outputting — cancel
                            pipeline.force_idle();
                        }
                    }
                });
            }
        }
    });

    // ── CGEventTap callback ──────────────────────────────────────────────────
    struct TapState {
        enabled: Arc<AtomicBool>,
        /// True while the user is recording a hotkey in Settings UI.
        recording: Arc<AtomicBool>,
        fn_pressed: AtomicBool,
        /// Milliseconds of last sent `true` event (for debounce).
        /// On Intel Macs, one physical Fn keypress can generate the sequence
        /// FlagsChanged(down) → FlagsChanged(up) → FlagsChanged(down) in rapid
        /// succession, causing a spurious second press event. We suppress any
        /// `true` send within 300 ms of the previous one.
        last_press_ms: std::sync::atomic::AtomicU64,
        tx: std::sync::mpsc::Sender<bool>,
        app_handle: tauri::AppHandle,
    }

    unsafe extern "C" fn tap_callback(
        _proxy: CGEventTapProxy,
        event_type: u32,
        event: CGEventRef,
        user_info: *mut c_void,
    ) -> CGEventRef {
        let state = &*(user_info as *const TapState);
        let keycode = CGEventGetIntegerValueField(event, CG_KEYBOARD_EVENT_KEYCODE);

        // ── Hotkey recording mode: detect Fn/Globe and notify the frontend ──
        if state.recording.load(Ordering::Relaxed) {
            let is_fn_press = match event_type {
                CG_EVENT_KEY_DOWN if keycode == GLOBE_KEYCODE => true,
                CG_EVENT_FLAGS_CHANGED if keycode == FN_KEYCODE => {
                    CGEventGetFlags(event) & CG_FLAG_FN != 0
                }
                _ => false,
            };
            if is_fn_press {
                let _ = state.app_handle.emit("hotkey:fn_detected", ());
            }
            return event;
        }

        // ── Normal operation: drive pipeline ────────────────────────────────
        if !state.enabled.load(Ordering::Relaxed) {
            return event;
        }

        // Helper: send a debounced "press" signal.
        // Returns true if the signal was actually sent (not throttled).
        let try_send_press = |state: &TapState| -> bool {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let last = state.last_press_ms.load(Ordering::Relaxed);
            if now_ms.saturating_sub(last) < 300 {
                return false; // within debounce window — suppress
            }
            state.last_press_ms.store(now_ms, Ordering::Relaxed);
            let _ = state.tx.send(true);
            true
        };

        match event_type {
            CG_EVENT_KEY_DOWN if keycode == GLOBE_KEYCODE => {
                if !state.fn_pressed.swap(true, Ordering::Relaxed) {
                    try_send_press(state);
                }
            }
            CG_EVENT_KEY_UP if keycode == GLOBE_KEYCODE => {
                state.fn_pressed.store(false, Ordering::Relaxed);
            }
            CG_EVENT_FLAGS_CHANGED if keycode == FN_KEYCODE => {
                // Fn key on Intel Macs generates FlagsChanged; flag bit indicates
                // pressed/released. Debounce prevents spurious OS double-events.
                let fn_down = CGEventGetFlags(event) & CG_FLAG_FN != 0;
                if fn_down {
                    if !state.fn_pressed.swap(true, Ordering::Relaxed) {
                        try_send_press(state);
                    }
                } else {
                    state.fn_pressed.store(false, Ordering::Relaxed);
                }
            }
            _ => {}
        }
        event
    }

    // ── Register tap on main RunLoop ─────────────────────────────────────────
    // Must be on the main thread so the callback (and any implicit framework calls)
    // run there — avoids TSM/HIToolbox thread-safety assertions.
    let app_handle2 = app_handle.clone();
    app_handle
        .run_on_main_thread(move || unsafe {
            let state = Box::new(TapState {
                enabled,
                recording,
                fn_pressed: AtomicBool::new(false),
                last_press_ms: std::sync::atomic::AtomicU64::new(0),
                tx,
                app_handle: app_handle2,
            });
            let state_ptr = Box::into_raw(state) as *mut c_void;

            let tap = CGEventTapCreate(
                TAP_TYPE,
                TAP_PLACE,
                TAP_LISTEN_ONLY,
                event_mask,
                tap_callback,
                state_ptr,
            );
            if tap.is_null() {
                tracing::warn!("[FnKey] CGEventTapCreate failed — Input Monitoring permission may not be granted");
                // Reclaim state to avoid leak
                drop(Box::from_raw(state_ptr as *mut TapState));
                return;
            }
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                tracing::warn!("[FnKey] CFMachPortCreateRunLoopSource failed");
                return;
            }
            CFRunLoopAddSource(CFRunLoopGetMain(), source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            // Both `tap` (CFMachPort) and `source` (CFRunLoopSource) are intentionally
            // leaked — they must stay alive for the entire app lifetime.
            tracing::info!("[FnKey] CGEventTap registered on main RunLoop");
        })
        .ok();
}

#[cfg(not(target_os = "macos"))]
fn register_fn_key_tap(
    _app_handle: tauri::AppHandle,
    _enabled: Arc<AtomicBool>,
    _recording: Arc<AtomicBool>,
) {
}

fn build_shortcut_handler(
    app_handle: tauri::AppHandle,
    translate_hotkey_cache: Arc<Mutex<String>>,
) -> impl Fn(&tauri::AppHandle, &Shortcut, tauri_plugin_global_shortcut::ShortcutEvent)
       + Send
       + Sync
       + 'static {
    move |_app, shortcut, event| {
        // Only act on key-press; key-release is ignored (toggle-only mode).
        if event.state != ShortcutState::Pressed {
            return;
        }

        // Determine if this shortcut is the translate hotkey
        let th = translate_hotkey_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let is_translate = if !th.is_empty() {
            if let Some(t) = parse_hotkey(&th) {
                t.key == shortcut.key && t.mods == shortcut.mods
            } else {
                false
            }
        } else {
            false
        };

        let handle = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            let pipeline = handle.state::<pipeline::PipelineHandle>();
            match pipeline.current_state() {
                pipeline::PipelineState::Idle => {
                    if is_translate {
                        pipeline
                            .translate_session
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                        // Notify frontend that this recording is a translate session
                        let _ = handle.emit("pipeline:translate_session", ());
                    }
                    if let Err(e) = pipeline.start().await {
                        tracing::error!("Failed to start recording: {}", e);
                        let _ = handle.emit("pipeline:error", e.to_string());
                    }
                }
                pipeline::PipelineState::Recording => {
                    if let Err(e) = pipeline.stop().await {
                        tracing::error!("Failed to stop recording: {}", e);
                        let _ = handle.emit("pipeline:error", e.to_string());
                    }
                }
                _ => {
                    // Stuck in Transcribing/Polishing/Outputting — cancel
                    pipeline.force_idle();
                }
            }
        });
    }
}

fn parse_hotkey(s: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Modifiers::empty();
    let key_str = parts.last()?;

    for &part in &parts[..parts.len() - 1] {
        match part.to_lowercase().as_str() {
            "alt" => modifiers |= Modifiers::ALT,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "meta" | "super" | "win" | "cmd" => modifiers |= Modifiers::META,
            _ => return None,
        }
    }

    let code = match key_str.to_lowercase().as_str() {
        "space" => Code::Space,
        "tab" => Code::Tab,
        "enter" | "return" => Code::Enter,
        "backspace" => Code::Backspace,
        "escape" | "esc" => Code::Escape,
        "delete" => Code::Delete,
        "insert" => Code::Insert,
        "home" => Code::Home,
        "end" => Code::End,
        "pageup" => Code::PageUp,
        "pagedown" => Code::PageDown,
        "arrowup" | "up" => Code::ArrowUp,
        "arrowdown" | "down" => Code::ArrowDown,
        "arrowleft" | "left" => Code::ArrowLeft,
        "arrowright" | "right" => Code::ArrowRight,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "/" | "slash" => Code::Slash,
        "\\" | "backslash" => Code::Backslash,
        "." | "period" => Code::Period,
        "," | "comma" => Code::Comma,
        ";" | "semicolon" => Code::Semicolon,
        "'" | "quote" => Code::Quote,
        "`" | "backquote" => Code::Backquote,
        "-" | "minus" => Code::Minus,
        "=" | "equal" => Code::Equal,
        "[" | "bracketleft" => Code::BracketLeft,
        "]" | "bracketright" => Code::BracketRight,
        _ => return None,
    };

    let mods = if modifiers.is_empty() {
        None
    } else {
        Some(modifiers)
    };
    Some(Shortcut::new(mods, code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hotkey_ctrl_slash() {
        let s = parse_hotkey("Ctrl+/");
        assert!(s.is_some());
        let s = s.unwrap();
        assert_eq!(s.mods, Modifiers::CONTROL);
        assert_eq!(s.key, Code::Slash);
    }

    #[test]
    fn test_parse_hotkey_ctrl_shift_a() {
        let s = parse_hotkey("Ctrl+Shift+A");
        assert!(s.is_some());
        let s = s.unwrap();
        assert_eq!(s.mods, Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(s.key, Code::KeyA);
    }

    #[test]
    fn test_parse_hotkey_case_insensitive() {
        let s = parse_hotkey("cTrL+/");
        assert!(s.is_some());
        let s = s.unwrap();
        assert_eq!(s.mods, Modifiers::CONTROL);
        assert_eq!(s.key, Code::Slash);
    }

    #[test]
    fn test_parse_hotkey_f_keys() {
        for (key, expected) in [("F1", Code::F1), ("F12", Code::F12)] {
            let s = parse_hotkey(&format!("Ctrl+{}", key));
            assert!(s.is_some(), "Failed to parse Ctrl+{}", key);
            assert_eq!(s.unwrap().key, expected);
        }
    }

    #[test]
    fn test_parse_hotkey_meta_modifier() {
        for name in ["Meta", "Super", "Win", "Cmd"] {
            let s = parse_hotkey(&format!("{}+A", name));
            assert!(s.is_some(), "Failed to parse {}+A", name);
            assert_eq!(s.unwrap().mods, Modifiers::SUPER);
        }
    }

    #[test]
    fn test_parse_hotkey_no_modifier() {
        let s = parse_hotkey("A");
        assert!(s.is_some());
        assert_eq!(s.unwrap().mods, Modifiers::empty());
    }

    #[test]
    fn test_parse_hotkey_invalid_key() {
        let s = parse_hotkey("Alt+InvalidKey");
        assert!(s.is_none());
    }

    #[test]
    fn test_parse_hotkey_empty_string() {
        let s = parse_hotkey("");
        assert!(s.is_none());
    }

    #[test]
    fn test_parse_hotkey_digits() {
        let s = parse_hotkey("Ctrl+0");
        assert!(s.is_some());
        assert_eq!(s.unwrap().key, Code::Digit0);

        let s = parse_hotkey("Ctrl+9");
        assert!(s.is_some());
        assert_eq!(s.unwrap().key, Code::Digit9);
    }

    #[test]
    fn test_parse_hotkey_navigation_keys() {
        for (key, expected) in [
            ("Enter", Code::Enter),
            ("Tab", Code::Tab),
            ("Escape", Code::Escape),
            ("Backspace", Code::Backspace),
            ("Delete", Code::Delete),
            ("Up", Code::ArrowUp),
            ("Down", Code::ArrowDown),
        ] {
            let s = parse_hotkey(&format!("Alt+{}", key));
            assert!(s.is_some(), "Failed to parse Alt+{}", key);
            assert_eq!(s.unwrap().key, expected);
        }
    }
}
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("changyan=debug".parse().expect("static directive is valid")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Deep-link URL forwarding is handled automatically by the
            // "deep-link" feature of single-instance plugin.
            // Just focus the main window so the user sees the result.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            // Open devtools only when the "devtools" feature is explicitly enabled
            #[cfg(feature = "devtools")]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
                if let Some(window) = app.get_webview_window("capsule") {
                    window.open_devtools();
                }
            }

            // Make the capsule float above fullscreen apps on macOS.
            // Standard NSWindow + canJoinAllSpaces does NOT work for native fullscreen Spaces
            // on Monterey/Intel. The fix: convert the NSWindow to an NSPanel at runtime and
            // set floatingPanel=true. Apple docs: "A floating panel will show above full-screen
            // content." NSPanel is a subclass of NSWindow, so all Tauri APIs keep working.
            #[cfg(target_os = "macos")]
            if let Some(capsule) = app.get_webview_window("capsule") {
                if let Ok(ns_window_ptr) = capsule.ns_window() {
                    use objc2_app_kit::{
                        NSPanel, NSScreenSaverWindowLevel, NSWindow,
                        NSWindowCollectionBehavior, NSWindowStyleMask,
                    };
                    use objc2::runtime::AnyClass;
                    unsafe {
                        // Convert NSWindow → NSPanel via runtime class swap.
                        // Safe because NSPanel is a direct subclass of NSWindow and adds
                        // only behavioral overrides (no extra ivars in practice).
                        if let Some(panel_class) = AnyClass::get(c"NSPanel") {
                            extern "C" {
                                fn object_setClass(
                                    obj: *mut std::ffi::c_void,
                                    cls: *const std::ffi::c_void,
                                ) -> *const std::ffi::c_void;
                            }
                            object_setClass(
                                ns_window_ptr as *mut std::ffi::c_void,
                                panel_class as *const AnyClass as *const std::ffi::c_void,
                            );

                            // Set panel-specific properties
                            let ns_panel = &*(ns_window_ptr as *const NSPanel);
                            ns_panel.setFloatingPanel(true);
                            ns_panel.setBecomesKeyOnlyIfNeeded(true);

                            // NonactivatingPanel is critical for floating panels in fullscreen
                            let ns_win = &*(ns_window_ptr as *const NSWindow);
                            let mask = ns_win.styleMask();
                            ns_win.setStyleMask(mask | NSWindowStyleMask::NonactivatingPanel);

                            tracing::info!("[Capsule] converted NSWindow → NSPanel (floatingPanel=true, nonactivating)");
                        } else {
                            tracing::warn!("[Capsule] NSPanel class not found — skipping panel conversion");
                        }

                        let ns_window = &*(ns_window_ptr as *const NSWindow);
                        let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
                            | NSWindowCollectionBehavior::FullScreenAuxiliary
                            | NSWindowCollectionBehavior::IgnoresCycle;
                        ns_window.setCollectionBehavior(behavior);
                        ns_window.setLevel(NSScreenSaverWindowLevel);

                        // Use CoreGraphics private API to tag window as "sticky"
                        // (appears on ALL workspaces including fullscreen Spaces).
                        // AppKit's canJoinAllSpaces does NOT work for fullscreen Spaces
                        // on Monterey/Intel. CGS operates at the compositor level and
                        // is used by apps like Rectangle, Hammerspoon, Tencent Meeting, etc.
                        crate::cgs_move_to_active_space(ns_window);

                        let actual_level = ns_window.level();
                        let actual_behavior = ns_window.collectionBehavior();
                        tracing::info!(
                            "[Capsule] window level={} (expected={}), collectionBehavior={:?}",
                            actual_level,
                            NSScreenSaverWindowLevel,
                            actual_behavior,
                        );

                        // Ensure capsule stays hidden after setup. The CGS calls above
                        // may implicitly make the window visible on some macOS versions.
                        ns_window.orderOut(None);
                    }
                } else {
                    tracing::warn!("[Capsule] ns_window() failed — cannot set level/behavior");
                }
            } else {
                tracing::warn!("[Capsule] window not found at setup");
            }

            // Register space-change observer so the capsule re-raises when entering/leaving fullscreen.
            #[cfg(target_os = "macos")]
            register_space_observer(app);

            // Log capsule position every time it becomes visible
            if let Some(capsule) = app.get_webview_window("capsule") {
                let handle = app.handle().clone();
                capsule.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Focused(true)) {
                        if let Some(w) = handle.get_webview_window("capsule") {
                            let pos = w.outer_position().ok();
                            let size = w.outer_size().ok();
                            let visible = w.is_visible().unwrap_or(false);
                            tracing::info!(
                                "[Capsule position] visible={visible} pos={pos:?} size={size:?}"
                            );
                        }
                    }
                });
            }

            let app_handle = app.handle().clone();

            // Initialize data directory and database
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("opentypeless.db");

            // Initialize stores
            let config_manager = storage::ConfigManager::new(app_handle.clone());
            let history_store = storage::HistoryStore::new(db_path.clone())
                .map_err(|e| anyhow::anyhow!("Failed to init history store: {}", e))?;
            let dictionary_store = storage::DictionaryStore::new(db_path)
                .map_err(|e| anyhow::anyhow!("Failed to init dictionary store: {}", e))?;
            let pipeline_handle = pipeline::PipelineHandle::new(app_handle.clone());

            // Load initial config to get hotkey
            let initial_config =
                tauri::async_runtime::block_on(config_manager.load()).unwrap_or_default();

            let fn_hotkey_enabled = Arc::new(AtomicBool::new(initial_config.hotkey == "Fn"));
            let fn_hotkey_recording = Arc::new(AtomicBool::new(false));

            app.manage(config_manager);
            app.manage(history_store);
            app.manage(dictionary_store);
            app.manage(pipeline_handle);
            app.manage(CloseToTrayCache(Arc::new(Mutex::new(
                initial_config.close_to_tray,
            ))));
            app.manage(SessionTokenStore(Arc::new(Mutex::new(String::new()))));
            app.manage(FnHotkeyState {
                enabled: fn_hotkey_enabled.clone(),
                recording: fn_hotkey_recording.clone(),
            });
            let translate_hotkey_arc = Arc::new(Mutex::new(initial_config.translate_hotkey.clone()));
            app.manage(TranslateHotkeyCache(translate_hotkey_arc.clone()));

            // Sync auto-start state with system
            {
                use tauri_plugin_autostart::ManagerExt;
                let autolaunch = app.handle().autolaunch();
                let is_enabled = autolaunch.is_enabled().unwrap_or(false);
                if initial_config.auto_start && !is_enabled {
                    let _ = autolaunch.enable();
                } else if !initial_config.auto_start && is_enabled {
                    let _ = autolaunch.disable();
                }
            }

            // Register global shortcut from config (skip for Fn key — handled via CGEventTap)
            let handler = build_shortcut_handler(app_handle.clone(), translate_hotkey_arc);
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(handler)
                    .build(),
            )?;
            if initial_config.hotkey != "Fn" {
                let shortcut =
                    parse_hotkey(&initial_config.hotkey).unwrap_or_else(default_shortcut);
                if let Err(e) = app.global_shortcut().register(shortcut) {
                    tracing::warn!(
                        "Failed to register shortcut '{}' (may be occupied): {e}",
                        initial_config.hotkey
                    );
                }
            }

            // Register translate hotkey if configured
            if !initial_config.translate_hotkey.is_empty() {
                if let Some(t_shortcut) = parse_hotkey(&initial_config.translate_hotkey) {
                    if let Err(e) = app.global_shortcut().register(t_shortcut) {
                        tracing::warn!(
                            "Failed to register translate shortcut '{}': {e}",
                            initial_config.translate_hotkey
                        );
                    }
                }
            }

            // Register CGEventTap for Fn/Globe key (runs on main RunLoop — no TSM threading issues)
            register_fn_key_tap(app_handle.clone(), fn_hotkey_enabled, fn_hotkey_recording);

            // System tray
            let tray_menu = build_tray_menu(&app_handle, false, true)
                .map_err(|e| anyhow::anyhow!("Failed to build tray menu: {}", e))?;

            let tray = TrayIconBuilder::new()
                .icon(
                    app.default_window_icon()
                        .expect("default window icon missing")
                        .clone(),
                )
                .menu(&tray_menu)
                .tooltip("ChangYan")
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show_hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let visible = window.is_visible().unwrap_or(false);
                            if visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                            refresh_tray(app);
                        }
                    }
                    "record" => {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let pipeline = handle.state::<pipeline::PipelineHandle>();
                            if pipeline.current_state() == pipeline::PipelineState::Idle {
                                if let Err(e) = pipeline.start().await {
                                    tracing::error!("Tray start recording failed: {}", e);
                                }
                            } else if pipeline.current_state() == pipeline::PipelineState::Recording
                            {
                                if let Err(e) = pipeline.stop().await {
                                    tracing::error!("Tray stop recording failed: {}", e);
                                }
                            }
                        });
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("tray:settings", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                            refresh_tray(app);
                        }
                    }
                    "history" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("tray:history", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                            refresh_tray(app);
                        }
                    }
                    "account" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("navigate", "#/account");
                            let _ = window.show();
                            let _ = window.set_focus();
                            refresh_tray(app);
                        }
                    }
                    "about" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("tray:about", ());
                            let _ = window.show();
                            let _ = window.set_focus();
                            refresh_tray(app);
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    let should_show = matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } | TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        }
                    );
                    if should_show {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                            refresh_tray(app);
                        }
                    }
                })
                .build(app)?;

            app.manage(TrayHandle {
                tray: Mutex::new(tray),
            });

            // Close-to-tray: intercept window close
            if let Some(main_window) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let close_to_tray = *handle
                            .state::<CloseToTrayCache>()
                            .0
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        if close_to_tray {
                            api.prevent_close();
                            // Save window state before hiding (skip if minimized)
                            if let Some(w) = handle.get_webview_window("main") {
                                if let (Ok(pos), Ok(size)) = (w.outer_position(), w.outer_size()) {
                                    if pos.x > -1000
                                        && pos.y > -1000
                                        && size.width >= 720
                                        && size.height >= 480
                                    {
                                        let ws = WindowState {
                                            x: pos.x,
                                            y: pos.y,
                                            width: size.width,
                                            height: size.height,
                                        };
                                        if let Ok(store) = handle.store("settings.json") {
                                            if let Ok(val) = serde_json::to_value(&ws) {
                                                store.set("window_state", val);
                                                let _ = store.save();
                                            }
                                        }
                                    }
                                }
                                let _ = w.hide();
                            }
                            refresh_tray(&handle);
                        }
                    }
                });
            }

            // Restore window state from previous session
            if let Ok(store) = app.handle().store("settings.json") {
                if let Some(val) = store.get("window_state") {
                    if let Ok(ws) = serde_json::from_value::<WindowState>(val.clone()) {
                        // Validate: skip if coordinates are off-screen (e.g. -32000 from minimized state)
                        if ws.x > -1000 && ws.y > -1000 && ws.width >= 720 && ws.height >= 480 {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.set_position(tauri::Position::Physical(
                                    tauri::PhysicalPosition::new(ws.x, ws.y),
                                ));
                                let _ = window.set_size(tauri::Size::Physical(
                                    tauri::PhysicalSize::new(ws.width, ws.height),
                                ));
                            }
                        }
                    }
                }
            }

            // Start minimized: only show window if not configured to start minimized
            if !initial_config.start_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            tracing::info!("ChangYan started");

            // P1-2: Pre-warm HTTP connection pool in background
            let warm_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                let pipeline = warm_handle.state::<pipeline::PipelineHandle>();
                pipeline.pre_warm().await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            get_config,
            update_config,
            test_stt_connection,
            test_llm_connection,
            bench_stt_connection,
            bench_llm_connection,
            fetch_llm_models,
            get_history,
            clear_history,
            get_dictionary,
            add_dictionary_entry,
            remove_dictionary_entry,
            update_hotkey,
            pause_hotkey,
            resume_hotkey,
            update_translate_hotkey,
            force_idle,
            start_recording_translate,
            set_auto_start,
            set_session_token,
            request_accessibility_permission,
            open_accessibility_settings,
            debug_capsule_window,
            bring_capsule_to_front,
            model_manager::get_sensevoice_model_status,
            model_manager::download_sensevoice_model,
            model_manager::delete_sensevoice_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
