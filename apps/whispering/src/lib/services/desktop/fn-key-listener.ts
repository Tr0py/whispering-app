import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
	defineErrors,
	extractErrorMessage,
	type InferErrors,
} from 'wellcrafted/error';
import { Err, Ok, type Result, tryAsync } from 'wellcrafted/result';
import type { ShortcutEventState } from '$lib/commands';
import { IS_MACOS } from '$lib/constants/platform';

/**
 * Service for the Fn key global listener.
 *
 * The standard global shortcut path (tauri-plugin-global-shortcut, which wraps
 * the Carbon RegisterEventHotKey API on macOS) cannot capture the Fn key
 * because Fn is a hardware modifier the OS does not deliver to that API.
 * This service runs a parallel NSEvent flag-change monitor in the Rust backend
 * and bridges its press / release events into the same callback contract that
 * the global shortcut manager uses.
 *
 * Only one subscription is supported at a time. Calling `register` twice
 * replaces the previous binding, matching the OS hotkey ownership model.
 */

const FnKeyListenerError = defineErrors({
	Unsupported: () => ({
		message: 'Fn key listener is only available on macOS.',
	}),
	StartFailed: ({ cause }: { cause: unknown }) => ({
		message: `Failed to start Fn key listener: ${extractErrorMessage(cause)}`,
		cause,
	}),
	StopFailed: ({ cause }: { cause: unknown }) => ({
		message: `Failed to stop Fn key listener: ${extractErrorMessage(cause)}`,
		cause,
	}),
	ListenFailed: ({ cause }: { cause: unknown }) => ({
		message: `Failed to subscribe to Fn key events: ${extractErrorMessage(cause)}`,
		cause,
	}),
});

export type FnKeyListenerError = InferErrors<typeof FnKeyListenerError>;

type Subscription = {
	callback: (state: ShortcutEventState) => void;
	on: ShortcutEventState[];
};

let activeSubscription: Subscription | null = null;
let unlistenPressed: UnlistenFn | null = null;
let unlistenReleased: UnlistenFn | null = null;
let watchdogTimer: ReturnType<typeof setTimeout> | null = null;

// Force a synthetic release if no real release arrives. Covers the rare case
// where Accessibility is revoked mid-hold and the backend stops firing events.
const WATCHDOG_MS = 5 * 60 * 1000;

function clearWatchdog() {
	if (watchdogTimer) {
		clearTimeout(watchdogTimer);
		watchdogTimer = null;
	}
}

function dispatch(state: ShortcutEventState) {
	const sub = activeSubscription;
	if (!sub) return;
	if (state === 'Pressed') {
		clearWatchdog();
		watchdogTimer = setTimeout(() => dispatch('Released'), WATCHDOG_MS);
	} else {
		clearWatchdog();
	}
	if (sub.on.includes(state)) sub.callback(state);
}

async function ensureBridgeStarted(): Promise<
	Result<void, FnKeyListenerError>
> {
	if (unlistenPressed && unlistenReleased) return Ok(undefined);

	const { data: pressed, error: pressedError } = await tryAsync({
		try: () => listen('fn-key-pressed', () => dispatch('Pressed')),
		catch: (error) => FnKeyListenerError.ListenFailed({ cause: error }),
	});
	if (pressedError) return Err(pressedError);

	const { data: released, error: releasedError } = await tryAsync({
		try: () => listen('fn-key-released', () => dispatch('Released')),
		catch: (error) => FnKeyListenerError.ListenFailed({ cause: error }),
	});
	if (releasedError) {
		await pressed();
		return Err(releasedError);
	}

	unlistenPressed = pressed;
	unlistenReleased = released;

	const { error: startError } = await tryAsync({
		try: () => invoke<void>('start_fn_key_listener'),
		catch: (error) => FnKeyListenerError.StartFailed({ cause: error }),
	});
	if (startError) {
		await pressed();
		await released();
		unlistenPressed = null;
		unlistenReleased = null;
		return Err(startError);
	}
	return Ok(undefined);
}

async function tearDownBridge(): Promise<Result<void, FnKeyListenerError>> {
	clearWatchdog();
	if (unlistenPressed) {
		await unlistenPressed();
		unlistenPressed = null;
	}
	if (unlistenReleased) {
		await unlistenReleased();
		unlistenReleased = null;
	}
	const { error } = await tryAsync({
		try: () => invoke<void>('stop_fn_key_listener'),
		catch: (error) => FnKeyListenerError.StopFailed({ cause: error }),
	});
	if (error) return Err(error);
	return Ok(undefined);
}

export const FnKeyListenerLive = {
	isSupported(): boolean {
		return IS_MACOS;
	},

	async register({
		callback,
		on,
	}: {
		callback: (state: ShortcutEventState) => void;
		on: ShortcutEventState[];
	}): Promise<Result<void, FnKeyListenerError>> {
		if (!IS_MACOS) return FnKeyListenerError.Unsupported();
		const { error } = await ensureBridgeStarted();
		if (error) return Err(error);
		activeSubscription = { callback, on };
		return Ok(undefined);
	},

	async unregister(): Promise<Result<void, FnKeyListenerError>> {
		if (!IS_MACOS) return Ok(undefined);
		activeSubscription = null;
		return tearDownBridge();
	},
};

export type FnKeyListener = typeof FnKeyListenerLive;
