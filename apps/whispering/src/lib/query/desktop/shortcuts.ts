import { Err, type Result } from 'wellcrafted/result';
import { type Command, commandCallbacks } from '$lib/commands';
import { IS_MACOS } from '$lib/constants/platform';
import { defineMutation } from '$lib/query/client';
import { desktopServices } from '$lib/services/desktop';
import type { FnKeyListenerError } from '$lib/services/desktop/fn-key-listener';
import {
	type Accelerator,
	FN_ACCELERATOR,
	type ShortcutError,
} from '$lib/services/desktop/global-shortcut-manager';

type RegisterError = ShortcutError | FnKeyListenerError;
type UnregisterError = ShortcutError | FnKeyListenerError;

/**
 * Global shortcuts - desktop-only, require Tauri.
 * These use system-level global shortcuts that work even when the app is not focused.
 *
 * The bare `'Fn'` accelerator is routed through the NSEvent-based Fn key
 * listener instead of `tauri-plugin-global-shortcut`, because the underlying
 * Carbon `RegisterEventHotKey` API cannot capture the Fn key (issue #1153).
 */
export const globalShortcuts = {
	registerCommand: defineMutation({
		mutationKey: ['shortcuts', 'registerCommandGlobally'] as const,
		mutationFn: async ({
			command,
			// Parameter renamed to indicate it may contain legacy "CommandOrControl" syntax
			// Legacy format: "CommandOrControl+Shift+R" → Modern format: "Command+Shift+R" (macOS) or "Control+Shift+R" (Windows/Linux)
			accelerator: legacyAcceleratorString,
		}: {
			command: Command;
			accelerator: Accelerator;
		}): Promise<Result<void, RegisterError>> => {
			// Convert legacy "CommandOrControl" syntax to platform-specific modifiers for backwards compatibility
			// This ensures users with old settings don't need to manually update their shortcuts
			const accelerator = legacyAcceleratorString.replace(
				'CommandOrControl',
				IS_MACOS ? 'Command' : 'Control',
			) as Accelerator;

			if (accelerator === FN_ACCELERATOR) {
				return desktopServices.fnKeyListener.register({
					callback: commandCallbacks[command.id],
					on: command.on,
				});
			}

			return desktopServices.globalShortcutManager.register({
				accelerator,
				callback: commandCallbacks[command.id],
				on: command.on,
			});
		},
	}),

	unregisterCommand: defineMutation({
		mutationKey: ['shortcuts', 'unregisterCommandGlobally'] as const,
		mutationFn: async ({
			accelerator,
		}: {
			accelerator: Accelerator;
		}): Promise<Result<void, UnregisterError>> => {
			if (accelerator === FN_ACCELERATOR) {
				return desktopServices.fnKeyListener.unregister();
			}
			return await desktopServices.globalShortcutManager.unregister(
				accelerator,
			);
		},
	}),

	unregisterAll: defineMutation({
		mutationKey: ['shortcuts', 'unregisterAllGlobalShortcuts'] as const,
		mutationFn: async (): Promise<Result<void, UnregisterError>> => {
			const { error: fnError } =
				await desktopServices.fnKeyListener.unregister();
			if (fnError) return Err(fnError);
			return desktopServices.globalShortcutManager.unregisterAll();
		},
	}),
};
