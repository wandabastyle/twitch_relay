<script lang="ts">
	import { onMount } from 'svelte';
	import favicon from '$lib/assets/favicon.svg';
	import { relayMode } from '$lib/stores';

	let { children } = $props();

	onMount(() => {
		relayMode.init();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}

<style>
	/* Global theme variables */
	:root {
		/* Tokyo Night Moon theme (Twitch) - default */
		--bg: #1e2030;
		--bg-soft: #222436;
		--surface: #2f334d;
		--surface-2: #3b4261;
		--fg: #c8d3f5;
		--muted: #a9b8e8;
		--accent: #82aaff;
		--accent-hover: #a8c5ff;
		--accent-soft: rgba(130, 170, 255, 0.16);
		--accent-border: rgba(130, 170, 255, 0.38);
		--focus-ring: rgba(130, 170, 255, 0.5);
		--accent-2: #c099ff;
		--success: #c3e88d;
		--warn: #ffc777;
		--danger: #ff757f;
		--border: #444a73;
		--ring: rgba(130, 170, 255, 0.45);
	}

	:global(body[data-theme="youtube"]) {
		/* YouTube-inspired red theme */
		--bg: #2a171d;
		--bg-soft: #342029;
		--surface: #462a35;
		--surface-2: #5a3342;
		--border: #7b3f52;
		--accent: #ff0033;
		--accent-hover: #cc0029;
		--accent-soft: rgba(255, 0, 51, 0.16);
		--accent-border: rgba(255, 0, 51, 0.35);
		--focus-ring: rgba(255, 0, 51, 0.5);
		--success: #4caf50;
		--danger: #ff5252;
		--warn: #ffb74d;
		--ring: rgba(255, 0, 51, 0.35);
	}

	:global(body) {
		margin: 0;
		min-height: 100vh;
		background: radial-gradient(circle at 20% -10%, color-mix(in srgb, var(--surface-2) 88%, black) 0%, var(--bg-soft) 45%, var(--bg) 100%);
		color: var(--fg);
		font-family: 'Space Grotesk', 'IBM Plex Sans', 'Noto Sans', sans-serif;
	}
</style>
