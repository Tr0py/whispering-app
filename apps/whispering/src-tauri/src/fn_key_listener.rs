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
//! The monitor lives behind `tauri::State<FnKeyListener>` and is installed /
//! torn down on demand via `start_fn_key_listener` / `stop_fn_key_listener`.

use tokio::sync::Mutex;

#[cfg(target_os = "macos")]
mod platform {
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags};
    use tauri::{AppHandle, Emitter, Runtime};

    pub struct MonitorHandle {
        token: Retained<AnyObject>,
    }

    // SAFETY: The token is opaque and only ever handed back to AppKit's
    // `+[NSEvent removeMonitor:]`, which is safe to call from any thread.
    // We never dereference it from Rust.
    unsafe impl Send for MonitorHandle {}

    pub fn install<R: Runtime>(app: AppHandle<R>) -> Result<MonitorHandle, String> {
        let last_state = Arc::new(AtomicBool::new(false));
        let app_for_block = app.clone();
        let last_for_block = last_state.clone();

        let block = RcBlock::new(move |event: NonNull<NSEvent>| {
            // SAFETY: AppKit passes a non-null, valid NSEvent for the duration of
            // the block invocation.
            let event_ref = unsafe { event.as_ref() };
            let flags = event_ref.modifierFlags();
            let now_down = flags.contains(NSEventModifierFlags::Function);
            let was_down = last_for_block.swap(now_down, Ordering::SeqCst);
            if now_down == was_down {
                return;
            }
            let name = if now_down {
                "fn-key-pressed"
            } else {
                "fn-key-released"
            };
            let _ = app_for_block.emit(name, ());
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

    pub fn uninstall(handle: MonitorHandle) {
        // SAFETY: `token` is a valid event monitor handle returned by
        // `addGlobalMonitorForEventsMatchingMask:handler:`. Calling
        // `removeMonitor:` with such a handle is the documented teardown.
        unsafe { NSEvent::removeMonitor(&handle.token) };
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use tauri::{AppHandle, Runtime};

    pub struct MonitorHandle;

    pub fn install<R: Runtime>(_app: AppHandle<R>) -> Result<MonitorHandle, String> {
        Err("Fn key listener is only supported on macOS".to_string())
    }

    pub fn uninstall(_handle: MonitorHandle) {}
}

pub use platform::MonitorHandle;

#[derive(Default)]
pub struct FnKeyListener {
    monitor: Mutex<Option<MonitorHandle>>,
}

pub mod commands {
    use tauri::{AppHandle, Runtime, State};

    use super::{platform, FnKeyListener};

    #[tauri::command]
    pub async fn start_fn_key_listener<R: Runtime>(
        app: AppHandle<R>,
        state: State<'_, FnKeyListener>,
    ) -> Result<(), String> {
        if !cfg!(target_os = "macos") {
            return Err("Fn key listener is only supported on macOS".to_string());
        }

        let mut guard = state.monitor.lock().await;
        if guard.is_some() {
            return Ok(());
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let app_for_main = app.clone();
        app.run_on_main_thread(move || {
            let _ = tx.send(platform::install(app_for_main));
        })
        .map_err(|e| format!("Failed to dispatch to main thread: {e}"))?;

        let install_result = rx
            .await
            .map_err(|_| "Main thread dropped install task".to_string())?;
        let handle = install_result?;
        *guard = Some(handle);
        Ok(())
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

        let (tx, rx) = tokio::sync::oneshot::channel();
        app.run_on_main_thread(move || {
            platform::uninstall(handle);
            let _ = tx.send(());
        })
        .map_err(|e| format!("Failed to dispatch to main thread: {e}"))?;
        rx.await
            .map_err(|_| "Main thread dropped uninstall task".to_string())?;
        Ok(())
    }

    #[tauri::command]
    pub fn is_fn_key_listener_supported() -> bool {
        cfg!(target_os = "macos")
    }
}
