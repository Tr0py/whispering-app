<script lang="ts">
	import * as Alert from '@epicenter/ui/alert';
	import { Button } from '@epicenter/ui/button';
	import * as Kbd from '@epicenter/ui/kbd';
	import * as Modal from '@epicenter/ui/modal';
	import AlertTriangle from '@lucide/svelte/icons/alert-triangle';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import HelpCircle from '@lucide/svelte/icons/help-circle';
	import {
		ACCELERATOR_SECTIONS,
		CommandOrAlt,
		CommandOrControl,
		KEYBOARD_EVENT_SUPPORTED_KEY_SECTIONS,
		OPTION_DEAD_KEYS,
	} from '$lib/constants/keyboard';
	import { IS_MACOS } from '$lib/constants/platform';

	let { type }: { type: 'local' | 'global' } = $props();
	let dialogOpen = $state(false);

	const isLocal = $derived(type === 'local');

	/**
	 * Examples for each shortcut type
	 */
	const SHORTCUT_EXAMPLES = {
		local: [
			' ',
			`${CommandOrControl.toLowerCase()}+a`,
			`${CommandOrControl.toLowerCase()}+shift+p`,
			`${CommandOrAlt.toLowerCase()}+s`,
			'f5',
			`control+${CommandOrAlt.toLowerCase()}+delete`,
		],
		global: [
			'Space',
			'Control+A',
			`${CommandOrControl}+Shift+P`,
			`${CommandOrAlt}+S`,
			'F5',
			`Control+${CommandOrAlt}+Delete`,
			...(IS_MACOS ? ['Fn'] : []),
		],
	} as const;
</script>

<Button
	variant="ghost"
	size="icon"
	class="size-6"
	onclick={() => (dialogOpen = true)}
	tooltip="Click for shortcut format guide"
>
	<HelpCircle class="size-4" />
	<span class="sr-only">Shortcut format help</span>
</Button>

<Modal.Root bind:open={dialogOpen}>
	<Modal.Content class="sm:max-w-3xl">
		<Modal.Header>
			<Modal.Title>
				{isLocal ? 'Local' : 'Global'}
				Shortcut Format Guide
			</Modal.Title>
			<Modal.Description>
				Learn how to format keyboard shortcuts for
				{isLocal
					? 'in-app'
					: 'system-wide'}
				use.
			</Modal.Description>
		</Modal.Header>

		<div class="flex flex-col gap-4">
			<!-- Quick format summary -->
			<div class="rounded-lg bg-muted p-4">
				<p class="text-sm">
					Use <code class="font-mono text-xs">modifier+key</code> format or just
					<code class="font-mono text-xs">key</code>
					for single keys.
				</p>
				{#if isLocal}
					<p class="text-sm text-muted-foreground mt-1">
						Any key from your keyboard can be used (lowercase). Below are common
						examples:
					</p>
				{/if}
			</div>

			<!-- Two-column flex layout -->
			<div class="flex flex-col sm:flex-row sm:divide-x">
				<!-- Left column: Modifiers -->
				<div class="sm:pr-4">
					<h4 class="text-sm font-semibold mb-1">Modifiers</h4>
					<p class="text-xs text-muted-foreground mb-2">Hold with other keys</p>
					<div class="flex flex-wrap sm:flex-col gap-1">
						{#each (isLocal ? KEYBOARD_EVENT_SUPPORTED_KEY_SECTIONS[0] : ACCELERATOR_SECTIONS[0]).keys as modifier}
							<Kbd.Root>{modifier}</Kbd.Root>
						{/each}
					</div>
				</div>

				<!-- Right column: All other keys -->
				<div class="flex-1 sm:pl-4">
					<div class="flex flex-col gap-4">
						{#each (isLocal ? KEYBOARD_EVENT_SUPPORTED_KEY_SECTIONS : ACCELERATOR_SECTIONS).slice(1) as section}
							<div>
								<h4 class="text-sm font-semibold mb-1">{section.title}</h4>
								<p class="text-xs text-muted-foreground mb-2">
									{section.description}
								</p>
								<div class="flex flex-wrap gap-1">
									{#each section.keys as key}
										<Kbd.Root>{key}</Kbd.Root>
									{/each}
								</div>
							</div>
						{/each}
					</div>
				</div>
			</div>

			<!-- Examples -->
			<div>
				<h4 class="mb-2 font-medium">Examples</h4>
				<div class="space-y-2 rounded-lg border p-3">
					{#each SHORTCUT_EXAMPLES[isLocal ? 'local' : 'global'] as example}
						<code class="block text-sm">{example}</code>
					{/each}
				</div>
			</div>

			{#if IS_MACOS}
				<Alert.Root variant="warning">
					<AlertTriangle class="size-4" />
					<Alert.Title>macOS Option Key Limitations</Alert.Title>
					<Alert.Description class="space-y-2">
						<p>
							On macOS, certain Option (Alt) key combinations act as "dead keys"
							that don't register properly when recording:
						</p>
						<div class="flex flex-wrap gap-1 my-2">
							{#each OPTION_DEAD_KEYS as key}
								<Kbd.Root>Option + {key.toUpperCase()}</Kbd.Root>
							{/each}
						</div>
						<p class="font-medium">Workarounds:</p>
						<ul class="list-disc list-inside space-y-1 ml-2">
							<li>Record in reverse: Press the letter first, then Option</li>
							<li>Edit manually: Type "alt+e" instead of recording</li>
						</ul>
					</Alert.Description>
				</Alert.Root>
			{/if}

			{#if IS_MACOS && !isLocal}
				<Alert.Root>
					<AlertTriangle class="size-4" />
					<Alert.Title>Using the Fn key on macOS</Alert.Title>
					<Alert.Description class="space-y-2">
						<p>
							<Kbd.Root>Fn</Kbd.Root> is supported as a single-key global shortcut
							(for example push-to-talk). It requires Accessibility permission and
							is routed through a dedicated listener instead of the standard hotkey
							API.
						</p>
						<p>
							Most browsers do not fire keyboard events for the Fn key alone, so
							press <strong>Edit manually</strong> and type
							<code class="font-mono text-xs">fn</code> if recording does not pick
							it up. Fn combined with other keys is not supported; combinations
							like Fn+F5 are claimed by macOS and arrive as F5 instead.
						</p>
					</Alert.Description>
				</Alert.Root>
			{/if}
		</div>

		<Modal.Footer>
			{#if !isLocal}
				<Button
					variant="outline"
					href="https://v2.tauri.app/plugin/global-shortcut/"
					target="_blank"
					rel="noreferrer"
				>
					<ExternalLink class="size-4" />
					View documentation
				</Button>
			{/if}
			<Button onclick={() => (dialogOpen = false)}>Close</Button>
		</Modal.Footer>
	</Modal.Content>
</Modal.Root>
