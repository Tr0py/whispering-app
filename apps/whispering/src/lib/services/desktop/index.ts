import { AutostartServiceLive } from './autostart';
import { CommandServiceLive } from './command';
import { FfmpegServiceLive } from './ffmpeg';
import { FnKeyListenerLive } from './fn-key-listener';
import { FsServiceLive } from './fs';
import { GlobalShortcutManagerLive } from './global-shortcut-manager';
import { PermissionsServiceLive } from './permissions';
import { CpalRecorderServiceLive } from './recorder/cpal';
import { FfmpegRecorderServiceLive } from './recorder/ffmpeg';
import { TrayIconServiceLive } from './tray';

/**
 * Desktop-only services.
 * These services are only available in the Tauri desktop app.
 */
export const desktopServices = {
	autostart: AutostartServiceLive,
	command: CommandServiceLive,
	ffmpeg: FfmpegServiceLive,
	fs: FsServiceLive,
	tray: TrayIconServiceLive,
	globalShortcutManager: GlobalShortcutManagerLive,
	fnKeyListener: FnKeyListenerLive,
	permissions: PermissionsServiceLive,
	cpalRecorder: CpalRecorderServiceLive,
	ffmpegRecorder: FfmpegRecorderServiceLive,
} as const;
