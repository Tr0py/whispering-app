//! Fn key global listener (macOS only).
//!
//! `tauri-plugin-global-shortcut` wraps Carbon's `RegisterEventHotKey`, which
//! cannot capture the Fn key because Fn is a hardware modifier handled by the
//! keyboard firmware and not delivered to the standard global hotkey API.
//!
//! This module installs an `NSEvent` global monitor for `flagsChanged` events,
//! watches the `NSEventModifierFlagFunction` bit (0x00800000), and emits
//! `fn-key-pressed` / `fn-key-released` Tauri events to the frontend whenever
//! the bit transitions. Requires Accessibility permission.
//!
//! On non-macOS, the Tauri commands stay present (so `generate_handler!` can
//! list them unconditionally) but `start_fn_key_listener` errors out and
//! `stop_fn_key_listener` is a no-op. The frontend gates its calls on
//! `IS_MACOS`, so these paths are not exercised in normal operation.

use tokio::sync::Mutex;

#[cfg(target_os = "macos")]
use std::ptr::NonNull;
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use std::sync::Arc;

#[cfg(target_os = "macos")]
use block2::RcBlock;
#[cfg(target_os = "macos")]
use objc2::rc::Retained;
#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags};
#[cfg(target_os = "macos")]
use tauri::Emitter;

#[cfg(target_os = "macos")]
pub struct MonitorHandle {
    token: Retained<AnyObject>,
}

#[cfg(not(target_os = "macos"))]
pub struct MonitorHandle;

// SAFETY: The token is opaque and only ever handed back to AppKit's
// `+[NSEvent removeMonitor:]`, which is safe to call from any thread.
// We never dereference it from Rust.
#[cfg(target_os = "macos")]
unsafe impl Send for MonitorHandle {}

#[derive(Default)]
pub struct FnKeyListener {
    monitor: Mutex<Option<MonitorHandle>>,
}

#[cfg(target_os = "macos")]
fn install_monitor<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<MonitorHandle, String> {
    let last_state = Arc::new(AtomicBool::new(false));

    let block = RcBlock::new(move |event: NonNull<NSEvent>| {
        // SAFETY: AppKit passes a non-null, valid NSEvent for the duration of
        // the block invocation.
        let event_ref = unsafe { event.as_ref() };
        let flags = event_ref.modifierFlags();
        let now_down = flags.contains(NSEventModifierFlags::Function);
        let was_down = last_state.swap(now_down, Ordering::SeqCst);
        if now_down == was_down {
            return;
        }
        let name = if now_down {
            "fn-key-pressed"
        } else {
            "fn-key-released"
        };
        let _ = app.emit(name, ());
    });

    let token = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(
        NSEventMask::FlagsChanged,
        &block,
    )
    .ok_or_else(|| {
        "Failed to install NSEvent monitor. Grant Accessibility permission and retry."
            .to_string()
    })?;

    Ok(MonitorHandle { token })
}

#[cfg(target_os = "macos")]
fn uninstall_monitor(handle: MonitorHandle) {
    // SAFETY: `token` is a valid event monitor handle returned by
    // `addGlobalMonitorForEventsMatchingMask:handler:`. Calling
    // `removeMonitor:` with such a handle is the documented teardown.
    unsafe { NSEvent::removeMonitor(&handle.token) };
}

pub mod commands {
    use tauri::{AppHandle, Runtime, State};

    use super::FnKeyListener;

    #[tauri::command]
    pub async fn start_fn_key_listener<R: Runtime>(
        app: AppHandle<R>,
        state: State<'_, FnKeyListener>,
    ) -> Result<(), String> {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (app, state);
            return Err("Fn key listener is only supported on macOS".to_string());
        }

        #[cfg(target_os = "macos")]
        {
            let mut guard = state.monitor.lock().await;
            if guard.is_some() {
                return Ok(());
            }

            let (tx, rx) = tokio::sync::oneshot::channel();
            let app_for_main = app.clone();
            app.run_on_main_thread(move || {
                let _ = tx.send(super::install_monitor(app_for_main));
            })
            .map_err(|e| format!("Failed to dispatch to main thread: {e}"))?;

            let handle = rx
                .await
                .map_err(|_| "Main thread dropped install task".to_string())??;
            *guard = Some(handle);
            Ok(())
        }
    }

    #[tauri::command]
    pub async fn stop_fn_key_listener<R: Runtime>(
        app: AppHandle<R>,
        state: State<'_, FnKeyListener>,
    ) -> Result<(), String> {
        let handle = state.monitor.lock().await.take();
        let Some(handle) = handle else {
            return Ok(());
        };

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (app, handle);
            Ok(())
        }

        #[cfg(target_os = "macos")]
        {
            let (tx, rx) = tokio::sync::oneshot::channel();
            app.run_on_main_thread(move || {
                super::uninstall_monitor(handle);
                let _ = tx.send(());
            })
            .map_err(|e| format!("Failed to dispatch to main thread: {e}"))?;
            rx.await
                .map_err(|_| "Main thread dropped uninstall task".to_string())?;
            Ok(())
        }
    }
}
