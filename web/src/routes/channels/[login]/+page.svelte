<script lang="ts">
  import { onMount } from 'svelte';

  import {
    getChannels,
    getRecordingRules,
    upsertRecordingRule,
    type RecordingRule
  } from '$lib/api';

  let { data } = $props<{ data: { login: string } }>();

  const QUALITY_OPTIONS = ['best', 'source', '1080p60', '1080p', '720p60', '720p', '480p', '360p', '160p'];

  const channelLogin = $derived(data.login.trim().toLowerCase());
  let channelExists = $state(true);
  let channelDisplayName = $state('');

  let isLoading = $state(true);
  let isSaving = $state(false);
  let errorMessage = $state<string | null>(null);
  let successMessage = $state<string | null>(null);

  let enabled = $state(false);
  let quality = $state('best');
  let stopWhenOffline = $state(true);
  let maxDurationMinutesInput = $state('');
  let keepLastVideosInput = $state('');

  onMount(async () => {
    await loadPageState();
  });

  async function loadPageState(): Promise<void> {
    isLoading = true;
    errorMessage = null;
    successMessage = null;

    try {
      const [channels, rules] = await Promise.all([getChannels(), getRecordingRules()]);
      const channel = channels.find((entry) => entry.login === channelLogin);
      channelExists = Boolean(channel);
      channelDisplayName = channel?.display_name || channel?.login || channelLogin;

      const rule = rules.find((entry) => entry.channel_login === channelLogin);
      applyRule(rule || null);
    } catch (err) {
      errorMessage = readMessage(err, 'failed to load channel settings');
    } finally {
      isLoading = false;
    }
  }

  function applyRule(rule: RecordingRule | null): void {
    if (!rule) {
      enabled = false;
      quality = 'best';
      stopWhenOffline = true;
      maxDurationMinutesInput = '';
      keepLastVideosInput = '';
      return;
    }

    enabled = rule.enabled;
    quality = rule.quality || 'best';
    stopWhenOffline = rule.stop_when_offline;
    maxDurationMinutesInput = rule.max_duration_minutes == null ? '' : String(rule.max_duration_minutes);
    keepLastVideosInput = rule.keep_last_videos == null ? '' : String(rule.keep_last_videos);
  }

  function parseOptionalPositiveInt(value: string, label: string): number | null {
    const trimmed = value.trim();
    if (!trimmed) {
      return null;
    }

    if (!/^\d+$/.test(trimmed)) {
      throw new Error(`${label} must be a whole number`);
    }

    const parsed = Number(trimmed);
    if (!Number.isSafeInteger(parsed) || parsed < 1) {
      throw new Error(`${label} must be at least 1`);
    }

    return parsed;
  }

  async function saveSettings(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    isSaving = true;
    errorMessage = null;
    successMessage = null;

    try {
      const maxDurationMinutes = parseOptionalPositiveInt(maxDurationMinutesInput, 'Max duration minutes');
      const keepLastVideos = parseOptionalPositiveInt(keepLastVideosInput, 'Keep last videos');

      const saved = await upsertRecordingRule({
        channel_login: channelLogin,
        enabled,
        quality,
        stop_when_offline: stopWhenOffline,
        max_duration_minutes: maxDurationMinutes,
        keep_last_videos: keepLastVideos
      });

      applyRule(saved);
      successMessage = 'Saved';
    } catch (err) {
      errorMessage = readMessage(err, 'failed to save settings');
    } finally {
      isSaving = false;
    }
  }

  function goBack(): void {
    window.location.assign('/');
  }

  function readMessage(error: unknown, fallback: string): string {
    if (error instanceof Error && error.message.trim().length > 0) {
      return error.message;
    }
    return fallback;
  }
</script>

<svelte:head>
  <title>Channel Setup - Twitch Relay</title>
</svelte:head>

<main class="shell">
  <section class="panel">
    <header class="header">
      <div>
        <p class="eyebrow">Channel Settings</p>
        <h1>{channelDisplayName}</h1>
        <p class="subtle">Configure recording behavior for <strong>{channelLogin}</strong></p>
      </div>
      <button type="button" class="ghost" onclick={goBack}>Back to channels</button>
    </header>

    {#if errorMessage}
      <p class="error" role="alert">{errorMessage}</p>
    {/if}
    {#if successMessage}
      <p class="success" role="status">{successMessage}</p>
    {/if}

    {#if isLoading}
      <p class="muted">Loading settings...</p>
    {:else if !channelExists}
      <p class="muted">This channel is not in your list. Add it on the front page first.</p>
    {:else}
      <form class="settings-form" onsubmit={saveSettings}>
        <label class="toggle-row">
          <input type="checkbox" bind:checked={enabled} />
          <span>Enable auto-record</span>
        </label>

        <label>
          Quality
          <select bind:value={quality}>
            {#each QUALITY_OPTIONS as option (option)}
              <option value={option}>{option}</option>
            {/each}
          </select>
        </label>

        <label class="toggle-row">
          <input type="checkbox" bind:checked={stopWhenOffline} />
          <span>Stop when channel goes offline</span>
        </label>

        <label>
          Max duration minutes
          <input
            type="number"
            min="1"
            step="1"
            bind:value={maxDurationMinutesInput}
            placeholder="Leave empty for no limit"
            inputmode="numeric"
          />
        </label>

        <label>
          Keep last videos
          <input
            type="number"
            min="1"
            step="1"
            bind:value={keepLastVideosInput}
            placeholder="Leave empty for no limit"
            inputmode="numeric"
          />
        </label>
        <p class="hint">Applies to completed recordings only. Older completed files are deleted automatically.</p>

        <div class="actions">
          <button type="submit" disabled={isSaving}>{isSaving ? 'Saving...' : 'Save settings'}</button>
        </div>
      </form>
    {/if}
  </section>
</main>

<style>
  :global(body) {
    --bg: #1e2030;
    --fg: #c8d3f5;
    --muted: #a9b8e8;
    --accent: #82aaff;
    --danger: #ff757f;
    --success: #c3e88d;
    margin: 0;
    min-height: 100vh;
    background: radial-gradient(circle at 20% -10%, #3b4261 0%, #222436 45%, #1e2030 100%);
    color: var(--fg);
    font-family: 'Space Grotesk', 'IBM Plex Sans', 'Noto Sans', sans-serif;
  }

  .shell {
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 2rem 1rem;
  }

  .panel {
    width: min(42rem, 100%);
    background: linear-gradient(160deg, rgba(47, 51, 77, 0.95), rgba(34, 36, 54, 0.95));
    border: 1px solid rgba(68, 74, 115, 0.65);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  h1 {
    margin: 0.18rem 0 0;
    font-size: clamp(1.45rem, 4vw, 1.9rem);
    line-height: 1.1;
    text-transform: lowercase;
  }

  .subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.86rem;
  }

  .subtle strong {
    color: var(--fg);
  }

  .settings-form {
    display: grid;
    gap: 0.8rem;
  }

  label {
    display: grid;
    gap: 0.35rem;
    font-weight: 600;
  }

  .toggle-row {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    font-weight: 500;
  }

  input,
  select {
    border: 1px solid rgba(160, 181, 216, 0.35);
    background: rgba(8, 12, 19, 0.9);
    color: var(--fg);
    border-radius: 0.6rem;
    padding: 0.68rem 0.76rem;
    font: inherit;
  }

  button {
    border: 0;
    border-radius: 0.6rem;
    padding: 0.62rem 0.95rem;
    background: var(--accent);
    color: #1e2030;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  .ghost {
    background: transparent;
    border: 1px solid rgba(162, 182, 217, 0.35);
    color: var(--fg);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.2rem;
  }

  .hint,
  .muted {
    margin: 0;
    color: var(--muted);
    font-size: 0.84rem;
  }

  .error,
  .success {
    margin: 0 0 0.9rem;
    border-radius: 0.6rem;
    padding: 0.65rem 0.72rem;
  }

  .error {
    background: rgba(194, 67, 89, 0.18);
    border: 1px solid rgba(246, 135, 154, 0.45);
    color: color-mix(in srgb, var(--danger) 72%, white);
  }

  .success {
    background: rgba(129, 199, 132, 0.18);
    border: 1px solid rgba(195, 232, 141, 0.45);
    color: color-mix(in srgb, var(--success) 72%, white);
  }

  @media (max-width: 640px) {
    .panel {
      padding: 1rem;
    }

    .header {
      flex-direction: column;
      align-items: flex-start;
    }

    .actions {
      justify-content: flex-start;
    }
  }
</style>
