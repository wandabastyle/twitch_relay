<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  import {
    getChannels,
    getRecordingRules,
    upsertRecordingRule,
    type RecordingRule
  } from '$lib/api';
  import LoadedFade from '$lib/components/LoadedFade.svelte';

  const SUCCESS_DISMISS_MS = 3500;

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

  // Auto-dismiss success message timer
  let successDismissTimer = $state<ReturnType<typeof setTimeout> | null>(null);

  function scheduleSuccessDismiss(): void {
    if (successDismissTimer) {
      clearTimeout(successDismissTimer);
    }
    successDismissTimer = setTimeout(() => {
      successMessage = null;
    }, SUCCESS_DISMISS_MS);
  }

  onDestroy(() => {
    if (successDismissTimer) {
      clearTimeout(successDismissTimer);
    }
  });

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

  function parseOptionalPositiveInt(
    value: string | number | null | undefined,
    label: string
  ): number | null {
    const normalized =
      value == null ? '' : typeof value === 'number' ? String(value) : value;
    const trimmed = normalized.trim();
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
      scheduleSuccessDismiss();
    } catch (err) {
      errorMessage = readMessage(err, 'failed to save settings');
    } finally {
      isSaving = false;
    }
  }

  function goBack(): void {
    window.location.assign('/twitch');
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

<section class="ui-page-panel">
  <header class="ui-page-header">
    <div>
      <p class="ui-page-eyebrow">Channel Settings</p>
      <h1 class="ui-page-title">{channelDisplayName}</h1>
      <p class="ui-page-subtle">Configure recording behavior for <strong>{channelLogin}</strong></p>
    </div>
    <button type="button" class="ui-nav-chip" onclick={goBack}>Back to channels</button>
  </header>

  {#if errorMessage}
    <p class="ui-error" role="alert">{errorMessage}</p>
  {/if}
  {#if successMessage}
    <p class="ui-alert-success" role="status">{successMessage}</p>
  {/if}

  {#if !isLoading && !channelExists}
    <p class="ui-muted">This channel is not in your list. Add it on the front page first.</p>
  {:else if !isLoading}
    <LoadedFade loaded={true}>
      <form class="ui-form" onsubmit={saveSettings}>
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

        <div class="ui-action-row">
          <button class="ui-button-primary" type="submit" disabled={isSaving}>{isSaving ? 'Saving...' : 'Save settings'}</button>
        </div>
      </form>
    </LoadedFade>
  {/if}
</section>

<style>
  /* Local form styles - kept for channel-specific layout */
  .toggle-row {
    display: inline-flex;
    align-items: center;
    gap: 0.55rem;
    font-weight: 500;
  }

  /* Form field and input styles - kept for specific layout needs */
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

  /* Local override to style button in header (overrides generic button selector) */
  .ui-page-header :global(.ui-nav-chip) {
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
    color: var(--fg);
    padding: 0.4rem 0.8rem;
    font-size: 0.85rem;
    min-height: 2rem;
  }

  .ui-page-header :global(.ui-nav-chip:hover) {
    border-color: var(--accent-border);
    background: var(--accent-soft);
  }

  /* Hint text - channel settings specific */
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.84rem;
  }

  @media (max-width: 640px) {
    .ui-page-panel {
      padding: 1rem;
    }

    .ui-page-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .ui-action-row {
      justify-content: flex-start;
    }
  }
</style>
