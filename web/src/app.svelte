<script lang="ts">
	import { initRouter, page } from '$lib/router/router.svelte';
	import { matchRoute } from '$lib/router/routes';
	import TwitchLayout from '$lib/layouts/twitch-layout.svelte';
	import YouTubeLayout from '$lib/layouts/you-tube-layout.svelte';
	import IndexRedirect from '$lib/pages/index-redirect.svelte';
	import QrLoginPage from '$lib/pages/qr-login-page.svelte';
	import TwitchChannelPage from '$lib/pages/twitch-channel-page.svelte';
	import TwitchHomePage from '$lib/pages/twitch-home-page.svelte';
	import TwitchRecordingPlayerPage from '$lib/pages/twitch-recording-player-page.svelte';
	import TwitchRecordingsPage from '$lib/pages/twitch-recordings-page.svelte';
	import WatchPage from '$lib/pages/watch-page.svelte';
	import YouTubeChannelPage from '$lib/pages/you-tube-channel-page.svelte';
	import YouTubeHomePage from '$lib/pages/you-tube-home-page.svelte';
	import YouTubePlaylistPage from '$lib/pages/you-tube-playlist-page.svelte';
	import YouTubePlaylistsPage from '$lib/pages/you-tube-playlists-page.svelte';
	import YouTubeRecentPage from '$lib/pages/you-tube-recent-page.svelte';
	import YouTubeWatchPage from '$lib/pages/you-tube-watch-page.svelte';

	// Initialize router
	initRouter();
	
	// Handle redirects (e.g., / to /twitch)
	const currentPath = $derived(page.url.pathname);
	
	// Redirect from / to /twitch
	$effect(() => {
		if (currentPath === '/') {
			globalThis.window.location.replace('/twitch');
		}
	});
	
	// Current route name
	const routeMatch = $derived(matchRoute(currentPath));
	const currentRoute = $derived(routeMatch && routeMatch.name);
	
	// Determine if we need a layout
	const isTwitchRoute = $derived(
		currentRoute === 'twitch' ||
		currentRoute === 'twitch_channel' ||
		currentRoute === 'twitch_recordings' ||
		currentRoute === 'twitch_recordings_play'
	);
	
	const isYouTubeRoute = $derived(
		currentRoute === 'youtube' ||
		currentRoute === 'youtube_recent' ||
		currentRoute === 'youtube_playlists' ||
		currentRoute === 'youtube_channel' ||
		currentRoute === 'youtube_playlist' ||
		currentRoute === 'youtube_watch'
	);
</script>

<main class="app-root">
	{#if currentRoute === 'index'}
		<IndexRedirect />
	{:else if currentRoute === 'twitch'}
		<TwitchLayout><TwitchHomePage /></TwitchLayout>
	{:else if currentRoute === 'twitch_channel'}
		<TwitchLayout><TwitchChannelPage /></TwitchLayout>
	{:else if currentRoute === 'twitch_recordings'}
		<TwitchLayout><TwitchRecordingsPage /></TwitchLayout>
	{:else if currentRoute === 'twitch_recordings_play'}
		<TwitchLayout><TwitchRecordingPlayerPage /></TwitchLayout>
	{:else if currentRoute === 'watch'}
		<WatchPage ticket={page.params.ticket ?? ''} />
	{:else if currentRoute === 'qr_login'}
		<QrLoginPage token={page.params.token ?? ''} />
	{:else if currentRoute === 'youtube'}
		<YouTubeLayout><YouTubeHomePage /></YouTubeLayout>
	{:else if currentRoute === 'youtube_recent'}
		<YouTubeLayout><YouTubeRecentPage /></YouTubeLayout>
	{:else if currentRoute === 'youtube_playlists'}
		<YouTubeLayout><YouTubePlaylistsPage /></YouTubeLayout>
	{:else if currentRoute === 'youtube_channel'}
		<YouTubeLayout><YouTubeChannelPage channel_id={page.params.channel_id ?? ''} /></YouTubeLayout>
	{:else if currentRoute === 'youtube_playlist'}
		<YouTubeLayout><YouTubePlaylistPage playlist_id={page.params.playlist_id ?? ''} /></YouTubeLayout>
	{:else if currentRoute === 'youtube_watch'}
		<YouTubeLayout><YouTubeWatchPage video_id={page.params.video_id ?? ''} /></YouTubeLayout>
	{:else}
		<div class="not-found">
			<h1>404</h1>
			<p>Page not found</p>
			<a href="/twitch">Go to Twitch Relay</a>
		</div>
	{/if}
</main>

<style>
	.app-root {
		min-height: 100vh;
	}

	.not-found {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		gap: 1rem;

		h1 {
			font-size: 4rem;
			margin: 0;
			color: var(--fg);
		}

		p {
			color: var(--muted);
			margin: 0;
		}

		a {
			color: var(--accent);
			text-decoration: none;
			margin-top: 1rem;

			&:hover {
				text-decoration: underline;
			}
		}
	}
</style>
