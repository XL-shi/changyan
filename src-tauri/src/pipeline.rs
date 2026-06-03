use anyhow::Result;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};
use std::sync::atomic::{AtomicI32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tauri::Manager;
use tokio::sync::Notify;

// ─── Interaction activity heuristic ───
// Tracks recent keyboard typing and mouse clicks observed by the CGEventTap in lib.rs.
// Used as a last-resort focus signal for apps that are opaque to AX queries.

/// PID of the frontmost app at the time of the last observed text key press.
pub(crate) static LAST_TEXT_KEY_APP_PID: AtomicI32 = AtomicI32::new(0);
/// Milliseconds since UNIX epoch of the last observed text key press.
pub(crate) static LAST_TEXT_KEY_MS: AtomicU64 = AtomicU64::new(0);
/// PID of the frontmost app at the time of the last observed mouse click.
pub(crate) static LAST_CLICK_APP_PID: AtomicI32 = AtomicI32::new(0);
/// Milliseconds since UNIX epoch of the last observed mouse click.
pub(crate) static LAST_CLICK_MS: AtomicU64 = AtomicU64::new(0);

/// Called from the CGEventTap callback whenever a printable key is pressed.
pub(crate) fn record_text_key_activity(app_pid: i32) {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    LAST_TEXT_KEY_APP_PID.store(app_pid, Ordering::Relaxed);
    LAST_TEXT_KEY_MS.store(now_ms, Ordering::Relaxed);
}

/// Called from the CGEventTap callback whenever a mouse button is pressed.
/// A click is a weaker signal than typing — it suggests the user may have
/// activated a text input but we can't be certain without AX.
pub(crate) fn record_click_activity(app_pid: i32) {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    LAST_CLICK_APP_PID.store(app_pid, Ordering::Relaxed);
    LAST_CLICK_MS.store(now_ms, Ordering::Relaxed);
}

use crate::app_detector;
use crate::audio::{AudioCaptureHandle, AudioConfig};
use crate::llm::{self, LlmConfig, PolishRequest};
use crate::output::{self, OutputMode};
use crate::storage;
use crate::stt::{self, SttConfig, TranscriptEvent};
use crate::SessionTokenStore;

// ─── Emoji filtering ───

/// Strip emoji characters from text. LLM models sometimes add emoji despite
/// being instructed not to; this is a hard post-processing filter.
fn strip_emoji(text: &str) -> String {
    text.chars()
        .filter(|&c| {
            let cp = c as u32;
            !matches!(cp,
                // Supplementary Multilingual Plane emoji (most common)
                0x1F000..=0x1FFFF |
                // Miscellaneous Symbols (☀ ☁ ❄ ♻ etc.)
                0x2600..=0x26FF |
                // Dingbats (✂ ✈ ✔ ❌ etc.)
                0x2700..=0x27BF |
                // Emoji variation selector
                0xFE0F |
                // Combining enclosing keycap (1️⃣ etc.)
                0x20E3
            )
        })
        .collect()
}

// ─── Timing constants ───

/// On macOS, verify whether the process has been granted Accessibility (Assistive Access)
/// permission. enigo uses CGEventPost under the hood, which requires this permission;
/// without it all synthesised key events are silently dropped by the OS.
/// Returns true on all non-macOS platforms (no permission needed).
fn is_accessibility_trusted() -> bool {
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

/// Check whether a text input field is currently focused in the frontmost application.
///
/// macOS strategy:
///   1. NSWorkspace: if no non-ChangYan app is frontmost → false (show clipboard popup).
///   2. AXUIElementCreateApplication(front_pid): query focused element of that specific app.
///      Per-app query works for Electron apps (VS Code etc.) where system-wide query fails.
///   3. If AX returns a focused element, check its role for text input types.
///   4. If AX fails for any reason (sandboxed app, no AX role exposed), default to true
///      (paste directly) — a non-ChangYan app IS frontmost, so paste is the right call.
fn is_input_focused() -> bool {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CStr;
        use std::os::raw::{c_char, c_long, c_void};

        type AXUIElementRef = *const c_void;
        type CFTypeRef = *const c_void;

        #[allow(non_upper_case_globals)]
        const kCFStringEncodingUTF8: u32 = 0x08000100;
        // Correct AX error constants (from AXError.h)
        const K_AX_ERROR_API_DISABLED: i32 = -25211;
        const K_AX_ERROR_NO_VALUE: i32 = -25212;

        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
            fn AXUIElementCreateSystemWide() -> AXUIElementRef;
            fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut i32) -> i32;
            fn AXUIElementCopyAttributeValue(
                element: AXUIElementRef,
                attribute: *const c_void,
                value: *mut CFTypeRef,
            ) -> i32;
            fn CFRelease(cf: CFTypeRef);
            fn CFStringCreateWithCString(
                alloc: *const c_void,
                c_str: *const c_char,
                encoding: u32,
            ) -> *const c_void;
            fn CFStringGetCString(
                the_string: *const c_void,
                buffer: *mut c_char,
                buffer_size: c_long,
                encoding: u32,
            ) -> bool;
        }

        // Step 1: NSWorkspace — is a non-ChangYan app currently frontmost?
        use objc2_app_kit::NSWorkspace;
        let workspace = NSWorkspace::sharedWorkspace();
        let front_app = workspace.frontmostApplication();
        let front_pid = match front_app {
            None => {
                tracing::debug!("[Focus] no frontmost app");
                return false;
            }
            Some(app) => app.processIdentifier(),
        };

        if front_pid == std::process::id() as i32 {
            tracing::debug!("[Focus] ChangYan is frontmost → clipboard popup");
            return false;
        }

        // Step 2: Query focused element from the frontmost app directly.
        // Per-app query is more reliable than system-wide for Electron/sandboxed apps.
        unsafe {
            let app_el = AXUIElementCreateApplication(front_pid);
            if app_el.is_null() {
                tracing::info!("[Focus] non-ChangYan frontmost (pid={}) → paste", front_pid);
                return true;
            }

            let focused_attr = CFStringCreateWithCString(
                std::ptr::null(),
                b"AXFocusedUIElement\0".as_ptr() as *const c_char,
                kCFStringEncodingUTF8,
            );
            let mut focused: CFTypeRef = std::ptr::null();
            let err = AXUIElementCopyAttributeValue(app_el, focused_attr, &mut focused);
            CFRelease(focused_attr);
            // app_el kept alive — needed for the window-level fallback below.

            if err == K_AX_ERROR_API_DISABLED {
                CFRelease(app_el);
                tracing::info!(
                    "[Focus] AX disabled for pid={} → paste (non-ChangYan frontmost)",
                    front_pid
                );
                return true;
            }

            if err == K_AX_ERROR_NO_VALUE || focused.is_null() {
                // App-level AXFocusedUIElement missing. Try three fallbacks in order:
                // 1. Window-level: app.AXFocusedWindow → window.AXFocusedUIElement
                // 2. System-wide: AXUIElementCreateSystemWide().AXFocusedUIElement
                // Both are needed because different Electron variants expose focus at
                // different levels (or not at all).

                // --- Level 2: window-level -----------------------------------------
                let win_result: Option<bool> = 'win_check: {
                    let win_attr = CFStringCreateWithCString(
                        std::ptr::null(),
                        b"AXFocusedWindow\0".as_ptr() as *const c_char,
                        kCFStringEncodingUTF8,
                    );
                    let mut win: CFTypeRef = std::ptr::null();
                    let win_err = AXUIElementCopyAttributeValue(app_el, win_attr, &mut win);
                    CFRelease(win_attr);
                    if win_err != 0 || win.is_null() {
                        if !win.is_null() {
                            CFRelease(win);
                        }
                        tracing::info!(
                            "[Focus] pid={} AXFocusedWindow err={} → trying system-wide",
                            front_pid,
                            win_err
                        );
                        break 'win_check None;
                    }
                    let elem_attr = CFStringCreateWithCString(
                        std::ptr::null(),
                        b"AXFocusedUIElement\0".as_ptr() as *const c_char,
                        kCFStringEncodingUTF8,
                    );
                    let mut elem: CFTypeRef = std::ptr::null();
                    let elem_err =
                        AXUIElementCopyAttributeValue(win as AXUIElementRef, elem_attr, &mut elem);
                    CFRelease(elem_attr);
                    CFRelease(win);
                    if elem_err != 0 || elem.is_null() {
                        if !elem.is_null() {
                            CFRelease(elem);
                        }
                        tracing::info!(
                            "[Focus] pid={} window.AXFocusedUIElement err={} → trying system-wide",
                            front_pid,
                            elem_err
                        );
                        break 'win_check None;
                    }
                    let role_attr = CFStringCreateWithCString(
                        std::ptr::null(),
                        b"AXRole\0".as_ptr() as *const c_char,
                        kCFStringEncodingUTF8,
                    );
                    let mut win_role: CFTypeRef = std::ptr::null();
                    let role_err = AXUIElementCopyAttributeValue(
                        elem as AXUIElementRef,
                        role_attr,
                        &mut win_role,
                    );
                    CFRelease(role_attr);
                    CFRelease(elem);
                    if role_err != 0 || win_role.is_null() {
                        if !win_role.is_null() {
                            CFRelease(win_role);
                        }
                        // Element exists but role unreadable → safe to paste.
                        break 'win_check Some(true);
                    }
                    let mut rbuf = [0u8; 128];
                    let ok = CFStringGetCString(
                        win_role,
                        rbuf.as_mut_ptr() as *mut c_char,
                        128,
                        kCFStringEncodingUTF8,
                    );
                    CFRelease(win_role);
                    if !ok {
                        break 'win_check Some(true);
                    }
                    let role = CStr::from_ptr(rbuf.as_ptr() as *const c_char)
                        .to_str()
                        .unwrap_or("");
                    let is_text = matches!(
                        role,
                        "AXTextField" | "AXTextArea" | "AXComboBox" | "AXSearchField" | "AXWebArea"
                    );
                    tracing::info!(
                        "[Focus] pid={} window-level role='{}' → {}",
                        front_pid,
                        role,
                        if is_text { "paste" } else { "clipboard popup" }
                    );
                    Some(is_text)
                };
                CFRelease(app_el);

                if let Some(v) = win_result {
                    return v;
                }

                // --- Level 3: system-wide ------------------------------------------
                // Some Electron apps expose focus only through the system-wide element.
                let sys_focused_result: Option<bool> = 'sys_check: {
                    let sys = AXUIElementCreateSystemWide();
                    if sys.is_null() {
                        break 'sys_check None;
                    }
                    let sys_attr = CFStringCreateWithCString(
                        std::ptr::null(),
                        b"AXFocusedUIElement\0".as_ptr() as *const c_char,
                        kCFStringEncodingUTF8,
                    );
                    let mut sys_elem: CFTypeRef = std::ptr::null();
                    let sys_err = AXUIElementCopyAttributeValue(sys, sys_attr, &mut sys_elem);
                    CFRelease(sys_attr);
                    CFRelease(sys);
                    if sys_err != 0 || sys_elem.is_null() {
                        if !sys_elem.is_null() {
                            CFRelease(sys_elem);
                        }
                        tracing::info!(
                            "[Focus] pid={} system-wide no element (err={})",
                            front_pid,
                            sys_err
                        );
                        break 'sys_check None;
                    }
                    // Verify element belongs to our target app.
                    let mut elem_pid: i32 = -1;
                    AXUIElementGetPid(sys_elem, &mut elem_pid);
                    if elem_pid != front_pid {
                        tracing::info!(
                            "[Focus] pid={} system-wide element belongs to pid={}",
                            front_pid,
                            elem_pid
                        );
                        CFRelease(sys_elem);
                        break 'sys_check None;
                    }
                    let sys_role_attr = CFStringCreateWithCString(
                        std::ptr::null(),
                        b"AXRole\0".as_ptr() as *const c_char,
                        kCFStringEncodingUTF8,
                    );
                    let mut sys_role: CFTypeRef = std::ptr::null();
                    let sys_role_err = AXUIElementCopyAttributeValue(
                        sys_elem as AXUIElementRef,
                        sys_role_attr,
                        &mut sys_role,
                    );
                    CFRelease(sys_role_attr);
                    CFRelease(sys_elem);
                    if sys_role_err != 0 || sys_role.is_null() {
                        if !sys_role.is_null() {
                            CFRelease(sys_role);
                        }
                        // Element from our app exists but no role → assume text input.
                        tracing::info!(
                            "[Focus] pid={} system-wide element no role → paste",
                            front_pid
                        );
                        break 'sys_check Some(true);
                    }
                    let mut sbuf = [0u8; 128];
                    let ok = CFStringGetCString(
                        sys_role,
                        sbuf.as_mut_ptr() as *mut c_char,
                        128,
                        kCFStringEncodingUTF8,
                    );
                    CFRelease(sys_role);
                    if !ok {
                        break 'sys_check Some(true);
                    }
                    let sys_role_str = CStr::from_ptr(sbuf.as_ptr() as *const c_char)
                        .to_str()
                        .unwrap_or("");
                    let is_text = matches!(
                        sys_role_str,
                        "AXTextField" | "AXTextArea" | "AXComboBox" | "AXSearchField" | "AXWebArea"
                    );
                    tracing::info!(
                        "[Focus] pid={} system-wide role='{}' → {}",
                        front_pid,
                        sys_role_str,
                        if is_text { "paste" } else { "clipboard popup" }
                    );
                    Some(is_text)
                };

                if let Some(v) = sys_focused_result {
                    return v;
                }

                // --- Level 4: interaction-activity heuristic -----------------------
                // AX gave us nothing at any level. Fall back to recent user activity:
                //   • Typing (30 s): high confidence — text cursor is almost certainly active.
                //   • Mouse click (120 s): weaker signal — user clicked somewhere in the app,
                //     likely activating a text input (common workflow: click → voice).
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let last_key_pid = LAST_TEXT_KEY_APP_PID.load(Ordering::Relaxed);
                let last_key_ms = LAST_TEXT_KEY_MS.load(Ordering::Relaxed);
                if last_key_pid == front_pid && last_key_ms > 0 {
                    let age = now_ms.saturating_sub(last_key_ms);
                    if age < 30_000 {
                        tracing::info!(
                            "[Focus] pid={} keyboard-heuristic: typed {}ms ago → paste",
                            front_pid,
                            age
                        );
                        return true;
                    }
                }

                let last_click_pid = LAST_CLICK_APP_PID.load(Ordering::Relaxed);
                let last_click_ms = LAST_CLICK_MS.load(Ordering::Relaxed);
                if last_click_pid == front_pid && last_click_ms > 0 {
                    let age = now_ms.saturating_sub(last_click_ms);
                    if age < 120_000 {
                        tracing::info!(
                            "[Focus] pid={} click-heuristic: clicked {}ms ago → paste",
                            front_pid,
                            age
                        );
                        return true;
                    }
                }

                tracing::info!(
                    "[Focus] pid={} all AX levels failed, no recent interaction → clipboard popup",
                    front_pid
                );
                return false;
            }

            CFRelease(app_el);

            if err != 0 {
                // Other AX error (e.g. app doesn't expose AX at all) → assume paste is right.
                tracing::info!("[Focus] AX err={} for pid={} → paste", err, front_pid);
                return true;
            }

            // Step 3: Check the focused element's role.
            let role_attr = CFStringCreateWithCString(
                std::ptr::null(),
                b"AXRole\0".as_ptr() as *const c_char,
                kCFStringEncodingUTF8,
            );
            let mut role_ref: CFTypeRef = std::ptr::null();
            let role_err =
                AXUIElementCopyAttributeValue(focused as AXUIElementRef, role_attr, &mut role_ref);
            CFRelease(role_attr);
            CFRelease(focused);

            if role_err != 0 || role_ref.is_null() {
                // Can't read role but element IS focused → paste.
                tracing::info!("[Focus] no AXRole (err={}) but focused → paste", role_err);
                return true;
            }

            let mut buf = [0u8; 128];
            let ok = CFStringGetCString(
                role_ref,
                buf.as_mut_ptr() as *mut c_char,
                128,
                kCFStringEncodingUTF8,
            );
            CFRelease(role_ref);

            if !ok {
                return true;
            }

            // SAFETY: CFStringGetCString null-terminates the buffer on success.
            let role_str = CStr::from_ptr(buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap_or("");

            let result = matches!(
                role_str,
                "AXTextField" | "AXTextArea" | "AXComboBox" | "AXSearchField" | "AXWebArea"
            );
            tracing::info!(
                "[Focus] pid={} role='{}' → {}",
                front_pid,
                role_str,
                if result { "paste" } else { "clipboard popup" }
            );
            result
        }
    }

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::RECT;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
            GUITHREADINFO,
        };

        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return false;
            }
            let thread_id = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            if thread_id == 0 {
                return false;
            }
            let mut gti = GUITHREADINFO {
                cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                flags: 0,
                hwndActive: std::ptr::null_mut(),
                hwndFocus: std::ptr::null_mut(),
                hwndCapture: std::ptr::null_mut(),
                hwndMenuOwner: std::ptr::null_mut(),
                hwndMoveSize: std::ptr::null_mut(),
                hwndCaret: std::ptr::null_mut(),
                rcCaret: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
            };
            if GetGUIThreadInfo(thread_id, &mut gti) == 0 {
                return false;
            }
            if !gti.hwndCaret.is_null() {
                return true;
            }
            if gti.hwndFocus.is_null() {
                return false;
            }
            let mut buf = [0u16; 256];
            let len = GetClassNameW(gti.hwndFocus, buf.as_mut_ptr(), buf.len() as i32);
            if len <= 0 {
                return false;
            }
            let class = String::from_utf16_lossy(&buf[..len as usize]).to_lowercase();
            class.contains("edit") || class.contains("richedit") || class.contains("scintilla")
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// Delay before capturing selected text to ensure hotkey modifiers are released.
const SELECTED_TEXT_CAPTURE_DELAY_MS: u64 = 60;
/// Delay after simulating Ctrl+C to let the clipboard update.
const CLIPBOARD_COPY_SETTLE_MS: u64 = 100;
/// Interval for polling audio volume during recording.
const VOLUME_POLL_INTERVAL_MS: u64 = 50;
/// Timeout for STT finalization after recording stops.
const STT_FINALIZE_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    Idle,
    Recording,
    Transcribing,
    Polishing,
    Outputting,
}

impl PipelineState {
    fn as_u8(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Recording => 1,
            Self::Transcribing => 2,
            Self::Polishing => 3,
            Self::Outputting => 4,
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Recording,
            2 => Self::Transcribing,
            3 => Self::Polishing,
            4 => Self::Outputting,
            _ => Self::Idle,
        }
    }
}

pub struct PipelineHandle {
    app_handle: tauri::AppHandle,
    state: Arc<AtomicU8>,
    audio_handle: Arc<Mutex<Option<AudioCaptureHandle>>>,
    audio_volume: Arc<Mutex<f32>>,
    accumulated_text: Arc<Mutex<String>>,
    stt_done: Arc<Mutex<Arc<Notify>>>,
    /// Set by force_idle() to signal stop() it should abort without output.
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    preloaded_config: Arc<Mutex<Option<storage::AppConfig>>>,
    preloaded_app_ctx: Arc<Mutex<Option<app_detector::AppContext>>>,
    preloaded_dictionary: Arc<Mutex<Option<Vec<String>>>>,
    preloaded_selected_text: Arc<Mutex<Option<String>>>,
    /// Whether a text input was focused in the target app at recording-start time.
    /// Captured before the Capsule window is raised so osascript sees the correct app.
    preloaded_input_focused: Arc<std::sync::atomic::AtomicBool>,
    recording_start: Arc<Mutex<Option<std::time::Instant>>>,
    shared_client: reqwest::Client,
    /// Set to true when the translate hotkey triggers recording; cleared after run_once reads it.
    pub translate_session: Arc<std::sync::atomic::AtomicBool>,
    /// Last raw STT text from the most recent recording. Used by translate_last().
    pub last_raw_text: Arc<Mutex<String>>,
}

impl PipelineHandle {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            app_handle,
            state: Arc::new(AtomicU8::new(PipelineState::Idle.as_u8())),
            audio_handle: Arc::new(Mutex::new(None)),
            audio_volume: Arc::new(Mutex::new(0.0)),
            accumulated_text: Arc::new(Mutex::new(String::new())),
            stt_done: Arc::new(Mutex::new(Arc::new(Notify::new()))),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            preloaded_config: Arc::new(Mutex::new(None)),
            preloaded_app_ctx: Arc::new(Mutex::new(None)),
            preloaded_dictionary: Arc::new(Mutex::new(None)),
            preloaded_selected_text: Arc::new(Mutex::new(None)),
            preloaded_input_focused: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            recording_start: Arc::new(Mutex::new(None)),
            shared_client: reqwest::Client::new(),
            translate_session: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_raw_text: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Cancel any in-progress pipeline operation and immediately return to Idle.
    /// Safe to call from any thread and any pipeline state.
    pub fn force_idle(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        // Notify stt_done so any waiting stop() unblocks immediately
        self.stt_done
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .notify_one();
        // Drop audio capture
        *self.audio_handle.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.set_state(PipelineState::Idle);
    }

    fn set_state(&self, new_state: PipelineState) {
        self.state.store(new_state.as_u8(), Ordering::SeqCst);
        let _ = self.app_handle.emit("pipeline:state", new_state);

        // Show/hide capsule window based on state
        if new_state == PipelineState::Recording {
            // Raise capsule immediately when recording starts — do this on the Rust side
            // (before the frontend round-trip) so the window is at the correct level
            // in fullscreen spaces the moment recording begins.
            crate::raise_capsule_window(&self.app_handle);
        } else if new_state == PipelineState::Idle {
            // Capsule hide is managed by the frontend (useCapsuleResize win.hide()).
            // Moving the hide here caused a race: the Rust hide ran after the frontend's
            // win.show() for the copy dialog, leaving the copy dialog invisible.
        }

        // Update tray tooltip + menu to reflect pipeline state
        if let Some(tray_handle) = self.app_handle.try_state::<crate::TrayHandle>() {
            let tooltip = match new_state {
                PipelineState::Recording => "ChangYan - Recording...",
                PipelineState::Transcribing => "ChangYan - Transcribing...",
                PipelineState::Polishing => "ChangYan - Polishing...",
                PipelineState::Outputting => "ChangYan - Outputting...",
                PipelineState::Idle => "ChangYan",
            };
            if let Ok(t) = tray_handle.tray.lock() {
                let _ = t.set_tooltip(Some(tooltip));
            }
        }
        crate::refresh_tray(&self.app_handle);
    }

    pub fn current_state(&self) -> PipelineState {
        PipelineState::from_u8(self.state.load(Ordering::SeqCst))
    }

    /// Capture selected text from the foreground app by simulating Ctrl+C / Cmd+C.
    /// Must be called when no hotkey modifier keys are physically held down.
    /// Called from async context via block_in_place, so std::thread::sleep is acceptable.
    fn capture_selected_text(&self) -> Option<String> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        let backup = clipboard.get_text().ok();

        if let Ok(mut enigo) = Enigo::new(&EnigoSettings::default()) {
            #[cfg(target_os = "macos")]
            let modifier = Key::Meta;
            #[cfg(not(target_os = "macos"))]
            let modifier = Key::Control;

            let pressed = enigo.key(modifier, Direction::Press).is_ok();
            if pressed {
                let _ = enigo.key(Key::Unicode('c'), Direction::Click);
                let _ = enigo.key(modifier, Direction::Release);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(CLIPBOARD_COPY_SETTLE_MS));

        let selected = clipboard.get_text().ok();

        // Always restore clipboard
        if let Some(ref b) = backup {
            let _ = clipboard.set_text(b);
        }

        tracing::info!(
            "Selected text capture: backup_len={}, selected_len={}",
            backup.as_deref().map(|s| s.len()).unwrap_or(0),
            selected.as_deref().map(|s| s.len()).unwrap_or(0)
        );

        // On macOS, if Cmd+C had no effect (e.g., no Accessibility permission),
        // the clipboard is unchanged, so selected == backup — return None to avoid
        // passing stale clipboard content to the LLM as if it were selected text.
        match &selected {
            Some(s) if !s.trim().is_empty() => {
                if backup.as_deref() == Some(s.as_str()) {
                    tracing::debug!(
                        "Selected text equals clipboard backup — Cmd+C had no effect, ignoring"
                    );
                    None
                } else {
                    Some(s.clone())
                }
            }
            _ => None,
        }
    }

    async fn load_config(&self) -> storage::AppConfig {
        self.app_handle
            .state::<storage::ConfigManager>()
            .load()
            .await
            .unwrap_or_default()
    }

    pub async fn start(&self) -> Result<()> {
        // Atomic CAS: only one caller can transition Idle → Recording
        if self
            .state
            .compare_exchange(
                PipelineState::Idle.as_u8(),
                PipelineState::Recording.as_u8(),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            return Ok(());
        }
        // Capture focus BEFORE emitting Recording state.
        // Emitting triggers the frontend to call bring_capsule_to_front which raises the capsule
        // window and makes ChangYan the frontmost app. Any focus check after that would see
        // the capsule (no text input) and always return false.
        let input_focused_now = tokio::task::spawn_blocking(is_input_focused)
            .await
            .unwrap_or(true);
        self.preloaded_input_focused
            .store(input_focused_now, Ordering::SeqCst);
        tracing::info!("[Focus] preloaded input_focused={}", input_focused_now);

        let _ = self
            .app_handle
            .emit("pipeline:state", PipelineState::Recording);
        // Update tray for recording state
        if let Some(tray_handle) = self.app_handle.try_state::<crate::TrayHandle>() {
            if let Ok(t) = tray_handle.tray.lock() {
                let _ = t.set_tooltip(Some("ChangYan - Recording..."));
            }
        }
        crate::refresh_tray(&self.app_handle);

        // Reset cancel flag for this new recording session
        self.cancelled.store(false, Ordering::SeqCst);

        // Clear accumulated text
        self.accumulated_text
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();

        // P0-2: Load config BEFORE starting audio capture — fail fast on missing API key
        let config_data = self.load_config().await;
        *self
            .preloaded_config
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(config_data.clone());
        *self
            .preloaded_app_ctx
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(app_detector::detect_current_app());
        let dict_words = self
            .app_handle
            .state::<storage::DictionaryStore>()
            .words()
            .await;
        *self
            .preloaded_dictionary
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(dict_words);

        tracing::debug!(
            "Pipeline using config: stt_provider={}, stt_key_len={}, stt_lang={}",
            config_data.stt_provider,
            config_data.stt_api_key.len(),
            config_data.stt_language
        );

        // Guard: empty API key — bail before starting audio (skip for cloud provider)
        if config_data.stt_api_key.is_empty()
            && config_data.stt_provider != "cloud"
            && config_data.stt_provider != "sensevoice-local"
        {
            let _ = self.app_handle.emit(
                "pipeline:error",
                "STT API key is not configured. Please set it in Settings → Speech Recognition.",
            );
            *self
                .preloaded_config
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_app_ctx
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_dictionary
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            self.set_state(PipelineState::Idle);
            return Ok(());
        }

        // P0-3: Pre-connect STT provider before spawning task
        let stt_api_key = if config_data.stt_provider == "cloud" {
            self.app_handle
                .state::<SessionTokenStore>()
                .0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else {
            config_data.stt_api_key.clone()
        };

        let stt_config = SttConfig {
            api_key: stt_api_key,
            language: if config_data.stt_language == "multi" {
                None
            } else {
                Some(config_data.stt_language.clone())
            },
            smart_format: true,
            sample_rate: 16000,
        };

        let app_data_dir = self.app_handle.path().app_data_dir().ok();
        let mut provider = stt::create_provider(
            &config_data.stt_provider,
            Some(self.shared_client.clone()),
            app_data_dir,
        );
        if let Err(e) = provider.connect(&stt_config).await {
            tracing::error!("STT connect failed: {}", e);
            let _ = self
                .app_handle
                .emit("pipeline:error", format!("STT connection failed: {e}"));
            *self
                .preloaded_config
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_app_ctx
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .preloaded_dictionary
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            self.set_state(PipelineState::Idle);
            return Ok(());
        }

        // Start audio capture on dedicated thread
        let config = AudioConfig::default();
        let (handle, mut audio_rx) = AudioCaptureHandle::start(config)?;

        // Store the audio handle's volume reference
        let audio_vol = handle.get_volume();
        *self.audio_volume.lock().unwrap_or_else(|e| e.into_inner()) = audio_vol;
        *self.audio_handle.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);

        *self
            .recording_start
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());

        // Volume monitoring task
        let app_handle = self.app_handle.clone();
        let audio_handle_ref = self.audio_handle.clone();
        let state_ref = self.state.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(VOLUME_POLL_INTERVAL_MS)).await;
                let current = PipelineState::from_u8(state_ref.load(Ordering::SeqCst));
                if current != PipelineState::Recording {
                    break;
                }
                let vol = audio_handle_ref
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                    .map(|h| h.get_volume())
                    .unwrap_or(0.0);
                let _ = app_handle.emit("audio:volume", vol);
            }
        });

        // Selected text will be captured in stop() after hotkey is released,
        // so Ctrl+C simulation won't conflict with held keys.
        *self
            .preloaded_selected_text
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;

        // STT streaming task — provider is already connected
        let app_handle = self.app_handle.clone();
        let accumulated = self.accumulated_text.clone();
        // Create a fresh Notify for this recording session to avoid leftover
        // permits from the previous recording polluting the next stop() wait.
        let fresh_stt_done = Arc::new(Notify::new());
        *self.stt_done.lock().unwrap_or_else(|e| e.into_inner()) = fresh_stt_done.clone();
        let stt_done = fresh_stt_done;

        tokio::spawn(async move {
            // Forward audio to STT and receive transcripts
            loop {
                tokio::select! {
                    chunk = audio_rx.recv() => {
                        match chunk {
                            Some(data) => {
                                let _ = provider.send_audio(&data).await;
                            }
                            None => {
                                // Audio channel closed — disconnect and capture final transcript.
                                // Timeout after 30s to avoid hanging if the provider stalls.
                                let disconnect_result = tokio::time::timeout(
                                    std::time::Duration::from_secs(30),
                                    provider.disconnect(),
                                )
                                .await;
                                match disconnect_result {
                                    Ok(Ok(Some(text))) => {
                                        let mut acc = accumulated.lock().unwrap_or_else(|e| e.into_inner());
                                        acc.push_str(&text);
                                        let current = acc.clone();
                                        drop(acc);
                                        let _ = app_handle.emit("stt:final", &current);
                                    }
                                    Ok(Ok(None)) => {}
                                    Ok(Err(e)) => {
                                        tracing::error!("STT disconnect error: {}", e);
                                        let _ = app_handle.emit("pipeline:error", format!("STT error: {e}"));
                                    }
                                    Err(_) => {
                                        tracing::warn!("STT disconnect timed out after 30s");
                                    }
                                }
                                break;
                            }
                        }
                    }
                    transcript = provider.recv_transcript() => {
                        match transcript {
                            Ok(Some(TranscriptEvent::Partial { text })) => {
                                let _ = app_handle.emit("stt:partial", &text);
                            }
                            Ok(Some(TranscriptEvent::Final { text, .. })) => {
                                let mut acc = accumulated.lock().unwrap_or_else(|e| e.into_inner());
                                acc.push_str(&text);
                                acc.push(' ');
                                let current = acc.clone();
                                drop(acc);
                                let _ = app_handle.emit("stt:final", &current);
                            }
                            Ok(Some(TranscriptEvent::Error { message })) => {
                                tracing::error!("STT error: {}", message);
                                let _ = app_handle.emit("pipeline:error", format!("STT error: {message}"));
                            }
                            Err(e) => {
                                tracing::error!("STT recv error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Signal that STT processing is complete
            stt_done.notify_one();
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        // Atomic CAS: only one caller can transition Recording → Transcribing.
        // If CAS fails because state is still Idle (start() hasn't set it yet due to async
        // scheduling), wait briefly and retry once — this fixes the race where a quick
        // press+release causes stop() to run before start() completes its own CAS.
        let cas_ok = self
            .state
            .compare_exchange(
                PipelineState::Recording.as_u8(),
                PipelineState::Transcribing.as_u8(),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok();
        if !cas_ok {
            if self.state.load(Ordering::SeqCst) == PipelineState::Idle.as_u8() {
                // start() may still be in flight; give it a moment then retry
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
            if self
                .state
                .compare_exchange(
                    PipelineState::Recording.as_u8(),
                    PipelineState::Transcribing.as_u8(),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_err()
            {
                return Ok(());
            }
        }
        let _ = self
            .app_handle
            .emit("pipeline:state", PipelineState::Transcribing);
        // Update tray for transcribing state
        if let Some(tray_handle) = self.app_handle.try_state::<crate::TrayHandle>() {
            if let Ok(t) = tray_handle.tray.lock() {
                let _ = t.set_tooltip(Some("ChangYan - Transcribing..."));
            }
        }
        crate::refresh_tray(&self.app_handle);

        let stop_start = std::time::Instant::now();

        // Capture selected text now — hotkey is released so Ctrl+C won't conflict.
        // Small delay to ensure hotkey modifiers are fully released (especially in toggle mode).
        let config_data = self
            .preloaded_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .unwrap_or_default();
        let selected_text = if config_data.selected_text_enabled {
            tokio::time::sleep(std::time::Duration::from_millis(
                SELECTED_TEXT_CAPTURE_DELAY_MS,
            ))
            .await;
            tokio::task::block_in_place(|| self.capture_selected_text())
        } else {
            None
        };
        tracing::info!(
            "Selected text result: len={}",
            selected_text.as_deref().map(|s| s.len()).unwrap_or(0)
        );
        *self
            .preloaded_selected_text
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = selected_text;

        // Stop audio capture (this drops the channel, signaling STT task to stop)
        {
            let mut handle = self.audio_handle.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref mut h) = *handle {
                h.stop();
            }
            *handle = None;
        }

        // P2-1: Pre-build LLM resources while waiting for STT
        let preloaded_config = self
            .preloaded_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let config = match preloaded_config {
            Some(c) => c,
            None => self.load_config().await,
        };
        let app_ctx = self
            .preloaded_app_ctx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .unwrap_or_else(app_detector::detect_current_app);
        let dictionary_words = self
            .preloaded_dictionary
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .unwrap_or_default();
        let selected_text = self
            .preloaded_selected_text
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();

        // Always use batch output: keyboard mode uses output_text() after full LLM
        // response arrives. Streaming chunk-by-chunk clipboard paste was unreliable
        // on Windows — each Ctrl+V is async and the next set_text() could overwrite
        // the clipboard before the target app processed the previous paste, producing
        // garbled output that differed from what History recorded.

        // Pre-build LLM provider and Enigo while STT is still processing
        // Translate sessions always need the LLM even when polish is disabled.
        let is_translate_session = self.translate_session.load(Ordering::Relaxed);
        tracing::info!(
            "[Pipeline] stop: polish_enabled={} is_translate={} llm_key_len={}",
            config.polish_enabled,
            is_translate_session,
            config.llm_api_key.len()
        );
        let pre_llm = if (config.polish_enabled || is_translate_session)
            && (!config.llm_api_key.is_empty() || config.llm_provider == "cloud")
        {
            let llm_api_key = if config.llm_provider == "cloud" {
                self.app_handle
                    .state::<SessionTokenStore>()
                    .0
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone()
            } else {
                config.llm_api_key.clone()
            };

            let llm_config = LlmConfig {
                api_key: llm_api_key,
                model: config.llm_model.clone(),
                base_url: config.llm_base_url.clone(),
                max_tokens: 4096,
                temperature: 0.3,
            };
            let provider =
                llm::create_provider(&config.llm_provider, Some(self.shared_client.clone()));
            Some((llm_config, provider))
        } else {
            None
        };

        // Wait for STT task to finish (handles both streaming and file-based providers)
        // Timeout after 120s to support long recordings
        let stt_done = self
            .stt_done
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        tokio::select! {
            _ = stt_done.notified() => {
                tracing::debug!("STT task completed");
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(STT_FINALIZE_TIMEOUT_SECS)) => {
                tracing::warn!("STT task timed out after {}s, using accumulated text so far", STT_FINALIZE_TIMEOUT_SECS);
            }
        }

        let stt_elapsed = stop_start.elapsed();
        tracing::info!(
            "[Pipeline Timing] STT finalize: {}ms",
            stt_elapsed.as_millis()
        );

        // If force_idle() was called while we were waiting, bail out silently.
        if self.cancelled.load(Ordering::SeqCst) {
            tracing::info!("[Pipeline] cancelled by force_idle, skipping output");
            return Ok(());
        }

        let raw_text = self
            .accumulated_text
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .trim()
            .to_string();

        if raw_text.is_empty() {
            self.set_state(PipelineState::Idle);
            return Ok(());
        }

        // Store raw text so the translate hotkey can translate it later
        *self.last_raw_text.lock().unwrap_or_else(|e| e.into_inner()) = raw_text.clone();

        let final_text;
        let llm_elapsed;

        // Polish with LLM (resources already pre-built)
        if let Some((llm_config, provider)) = pre_llm {
            self.set_state(PipelineState::Polishing);
            let llm_start = std::time::Instant::now();

            // on_chunk only drives the UI transcript display; actual output happens
            // in batch after the full response arrives (see output_text below).
            let app_handle = self.app_handle.clone();
            let on_chunk: llm::ChunkCallback = Box::new(move |chunk: &str| {
                let _ = app_handle.emit("llm:chunk", chunk);
            });

            let req = PolishRequest {
                raw_text: raw_text.clone(),
                app_type: app_ctx.app_type,
                dictionary: dictionary_words,
                translate_enabled: self.translate_session.swap(false, Ordering::Relaxed),
                target_lang: config.target_lang.clone(),
                selected_text,
                style_examples: {
                    let app_type_str = format!("{:?}", app_ctx.app_type);
                    self.app_handle
                        .state::<storage::HistoryStore>()
                        .recent_polished_by_app_type(&app_type_str, 5)
                        .await
                },
            };

            match provider.polish(&llm_config, &req, Some(&on_chunk)).await {
                Ok(response) => {
                    final_text = strip_emoji(response.polished_text.trim());
                    llm_elapsed = llm_start.elapsed();

                    if final_text.is_empty() {
                        tracing::info!(
                            "LLM returned empty text (filler-only input), skipping output"
                        );
                        self.set_state(PipelineState::Idle);
                        return Ok(());
                    }

                    if let Err(e) = self
                        .output_text(&final_text, &app_ctx.app_name, &config)
                        .await
                    {
                        tracing::error!("Output failed: {}", e);
                        let _ = self
                            .app_handle
                            .emit("pipeline:error", format!("Output failed: {e}"));
                    }
                }
                Err(e) => {
                    tracing::error!("LLM polish failed: {}, outputting raw text", e);
                    final_text = raw_text.clone();
                    llm_elapsed = llm_start.elapsed();

                    let _ = self
                        .app_handle
                        .emit("pipeline:error", format!("LLM polish failed: {e}"));
                    if let Err(e) = self
                        .output_text(&final_text, &app_ctx.app_name, &config)
                        .await
                    {
                        tracing::error!("Output failed: {}", e);
                        let _ = self
                            .app_handle
                            .emit("pipeline:error", format!("Output failed: {e}"));
                    }
                }
            }

            tracing::info!(
                "[Pipeline Timing] LLM polish: {}ms",
                llm_elapsed.as_millis()
            );
        } else {
            llm_elapsed = std::time::Duration::ZERO;
            final_text = raw_text.clone();
            if let Err(e) = self
                .output_text(&final_text, &app_ctx.app_name, &config)
                .await
            {
                tracing::error!("Output failed: {}", e);
                let _ = self
                    .app_handle
                    .emit("pipeline:error", format!("Output failed: {e}"));
            }
        }

        let total_elapsed = stop_start.elapsed();

        // Compute recording duration
        let duration_ms = self
            .recording_start
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .map(|start| start.elapsed().as_millis() as i64);

        tracing::info!(
            "[Pipeline Timing] Total stop(): {}ms (STT: {}ms, LLM: {}ms, Output+Save: {}ms)",
            total_elapsed.as_millis(),
            stt_elapsed.as_millis(),
            llm_elapsed.as_millis(),
            total_elapsed.as_millis() - stt_elapsed.as_millis() - llm_elapsed.as_millis(),
        );

        // Emit timing to frontend
        let _ = self.app_handle.emit(
            "pipeline:timing",
            serde_json::json!({
                "stt_ms": stt_elapsed.as_millis() as u64,
                "llm_ms": llm_elapsed.as_millis() as u64,
                "total_ms": total_elapsed.as_millis() as u64,
                "recording_ms": duration_ms,
            }),
        );

        // Save to history
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let entry = storage::HistoryEntry {
            id: 0, // auto-increment
            created_at: now,
            app_name: app_ctx.app_name,
            app_type: format!("{:?}", app_ctx.app_type),
            raw_text,
            polished_text: final_text,
            language: None,
            duration_ms,
        };
        if let Err(e) = self
            .app_handle
            .state::<storage::HistoryStore>()
            .add(entry)
            .await
        {
            tracing::error!("Failed to save history: {}", e);
        }

        self.set_state(PipelineState::Idle);
        Ok(())
    }

    async fn output_text(
        &self,
        text: &str,
        app_name: &str,
        config: &storage::AppConfig,
    ) -> Result<()> {
        self.set_state(PipelineState::Outputting);

        #[cfg(target_os = "macos")]
        if config.output_mode == "keyboard" && !is_accessibility_trusted() {
            let text_copy = text.to_string();
            let _ = tokio::task::spawn_blocking(move || {
                arboard::Clipboard::new()
                    .and_then(|mut cb| cb.set_text(text_copy))
                    .ok()
            })
            .await;
            let _ = self.app_handle.emit("pipeline:target_app", app_name);
            let _ = self
                .app_handle
                .emit("pipeline:copy_ready", text.to_string());
            // Raise capsule NOW while state is still Outputting — the idle guard in
            // raise_capsule_window blocks calls made after set_state(Idle).
            crate::raise_capsule_window(&self.app_handle);
            return Ok(());
        }

        // Use the input-focus state captured at recording-start time (before the Capsule window
        // was raised to the front). Checking it here would query the wrong frontmost app.
        let input_focused = self.preloaded_input_focused.load(Ordering::SeqCst);
        if !input_focused {
            let text_copy = text.to_string();
            let _ = tokio::task::spawn_blocking(move || {
                arboard::Clipboard::new()
                    .and_then(|mut cb| cb.set_text(text_copy))
                    .ok()
            })
            .await;
            let _ = self.app_handle.emit("pipeline:target_app", app_name);
            let _ = self
                .app_handle
                .emit("pipeline:copy_ready", text.to_string());
            // Raise capsule NOW while state is still Outputting — the idle guard in
            // raise_capsule_window blocks calls made after set_state(Idle).
            crate::raise_capsule_window(&self.app_handle);
            return Ok(());
        }

        let mode = if config.output_mode == "keyboard" {
            OutputMode::Keyboard
        } else {
            OutputMode::Clipboard
        };

        let output = output::create_output(mode);
        output.type_text(text).await?;

        let _ = self.app_handle.emit("pipeline:target_app", app_name);

        Ok(())
    }

    /// Translate the last transcribed text using LLM and output the result.
    /// Called when the user presses the translate hotkey after recording.
    pub async fn translate_last(&self) -> Result<()> {
        // Only run when idle
        if self.state.load(Ordering::SeqCst) != PipelineState::Idle.as_u8() {
            return Ok(());
        }

        let raw_text = self
            .last_raw_text
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        if raw_text.is_empty() {
            let _ = self
                .app_handle
                .emit("pipeline:error", "No recent transcription to translate.");
            return Ok(());
        }

        let config = self.load_config().await;

        let llm_api_key = if config.llm_provider == "cloud" {
            self.app_handle
                .state::<SessionTokenStore>()
                .0
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        } else {
            config.llm_api_key.clone()
        };

        if llm_api_key.is_empty() && config.llm_provider != "cloud" {
            let _ = self.app_handle.emit(
                "pipeline:error",
                "LLM API key is not configured. Please set it in Settings → AI Polish.",
            );
            return Ok(());
        }

        let llm_config = LlmConfig {
            api_key: llm_api_key,
            model: config.llm_model.clone(),
            base_url: config.llm_base_url.clone(),
            max_tokens: 4096,
            temperature: 0.3,
        };

        let provider = llm::create_provider(&config.llm_provider, Some(self.shared_client.clone()));
        let app_ctx = app_detector::detect_current_app();

        self.set_state(PipelineState::Polishing);

        let app_handle = self.app_handle.clone();
        let on_chunk: llm::ChunkCallback = Box::new(move |chunk: &str| {
            let _ = app_handle.emit("llm:chunk", chunk);
        });

        let req = PolishRequest {
            raw_text: raw_text.clone(),
            app_type: app_ctx.app_type,
            dictionary: self
                .app_handle
                .state::<storage::DictionaryStore>()
                .words()
                .await,
            translate_enabled: true,
            target_lang: config.target_lang.clone(),
            selected_text: None,
            style_examples: {
                let app_type_str = format!("{:?}", app_ctx.app_type);
                self.app_handle
                    .state::<storage::HistoryStore>()
                    .recent_polished_by_app_type(&app_type_str, 5)
                    .await
            },
        };

        match provider.polish(&llm_config, &req, Some(&on_chunk)).await {
            Ok(response) => {
                let final_text = strip_emoji(response.polished_text.trim());
                if final_text.is_empty() {
                    self.set_state(PipelineState::Idle);
                    return Ok(());
                }
                if let Err(e) = self
                    .output_text(&final_text, &app_ctx.app_name, &config)
                    .await
                {
                    tracing::error!("translate_last output failed: {}", e);
                    let _ = self
                        .app_handle
                        .emit("pipeline:error", format!("Output failed: {e}"));
                }
            }
            Err(e) => {
                tracing::error!("translate_last LLM failed: {}", e);
                let _ = self
                    .app_handle
                    .emit("pipeline:error", format!("Translation failed: {e}"));
            }
        }

        self.set_state(PipelineState::Idle);
        Ok(())
    }

    /// P1-2: Pre-warm HTTP connection pool by issuing a HEAD request to the STT endpoint.
    /// Call once after app startup to avoid cold-start TLS handshake on first recording.
    pub async fn pre_warm(&self) {
        let config = self.load_config().await;

        // Pre-warm STT endpoint
        let stt_endpoint = match config.stt_provider.as_str() {
            "cloud" => {
                let base = crate::api_base_url();
                format!("{}/api/proxy/stt", base)
            }
            "glm-asr" => "https://open.bigmodel.cn/api/paas/v4/audio/transcriptions".to_string(),
            "openai-whisper" => "https://api.openai.com/v1/audio/transcriptions".to_string(),
            "groq-whisper" => "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            "siliconflow" => "https://api.siliconflow.cn/v1/audio/transcriptions".to_string(),
            "deepgram" => "https://api.deepgram.com/v1/listen".to_string(),
            "assemblyai" => "https://api.assemblyai.com/v2/transcript".to_string(),
            _ => {
                tracing::debug!(
                    "Unknown STT provider '{}', skipping pre-warm",
                    config.stt_provider
                );
                return;
            }
        };
        tracing::debug!("Pre-warming HTTP connection to {}", stt_endpoint);
        let _ = self
            .shared_client
            .head(&stt_endpoint)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;
        tracing::debug!("STT connection pre-warm complete");

        // Pre-warm LLM endpoint if polish is enabled
        if config.polish_enabled {
            let llm_url = if config.llm_provider == "cloud" {
                let base = crate::api_base_url();
                format!("{}/api/proxy/llm", base)
            } else {
                config.llm_base_url.clone()
            };
            tracing::debug!("Pre-warming LLM connection to {}", llm_url);
            let _ = self
                .shared_client
                .head(&llm_url)
                .timeout(std::time::Duration::from_secs(5))
                .send()
                .await;
            tracing::debug!("LLM connection pre-warm complete");
        }
    }
}
