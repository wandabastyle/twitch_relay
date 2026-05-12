<script lang="ts">
  import AlarmClock from 'lucide-svelte/icons/alarm-clock';
  import Circle from 'lucide-svelte/icons/circle';
  import Loader from 'lucide-svelte/icons/loader';
  import Play from 'lucide-svelte/icons/play';
  import X from 'lucide-svelte/icons/x';
  import type { ChannelCardProps } from './types';

  let {
    channel,
    status,
    recordingRule,
    activeRecording,
    isWatching,
    onOpenSetup,
    onStartWatching,
    onToggleAutoRecord,
    onToggleManualRecording,
    onRemove
  }: ChannelCardProps = $props();

  function getRecordingTitle(): string {
    if (activeRecording?.mode === 'manual') {
      return 'Stop manual recording';
    }
    if (activeRecording?.mode === 'auto') {
      return 'Stop auto recording';
    }
    return 'Start recording now';
  }

  function getRecordingLabel(): string {
    if (activeRecording?.mode === 'manual') {
      return 'Stop manual recording';
    }
    if (activeRecording?.mode === 'auto') {
      return 'Stop auto recording';
    }
    return 'Start recording now';
  }

  function getRecordingClass(): string {
    if (activeRecording?.mode === 'manual') return 'active-manual';
    if (activeRecording?.mode === 'auto') return 'active-auto';
    return '';
  }
</script>

<article class="channel-card" class:live={status?.live}>
  <div class="channel-avatar-wrap">
    {#if channel.image_url}
      <img class="ui-avatar channel-avatar" src={channel.image_url} alt={channel.login} />
    {:else}
      <div class="ui-avatar ui-avatar-fallback channel-avatar fallback" aria-hidden="true">{channel.login.slice(0, 1)}</div>
    {/if}
    {#if status?.live}
      <span class="avatar-status-dot" aria-hidden="true"></span>
    {/if}
  </div>

  <div class="channel-content">
    <div class="channel-content-header">
      <div class="channel-name-area">
        <button type="button" class="channel-name" onclick={onOpenSetup}>
          {status?.display_name || channel.display_name || channel.login}
        </button>
        <p class="channel-meta">
          {channel.source === 'manual' ? 'Manual' : channel.source === 'followed' ? 'Followed' : 'Manual + Followed'}
        </p>
      </div>

      <div class="channel-controls">
        {#if status?.live}
          <button
            type="button"
            class={`icon-btn play-btn ${isWatching ? 'watching' : ''}`}
            onclick={onStartWatching}
            disabled={isWatching}
            title={isWatching ? 'Opening...' : 'Watch'}
            aria-label={isWatching ? 'Opening stream...' : 'Watch stream'}
          >
            {#if isWatching}
              <Loader size={18} class="spinning" />
            {:else}
              <Play size={18} />
            {/if}
          </button>
        {/if}
        <button
          type="button"
          class={`icon-btn clock-btn ${recordingRule?.enabled ? 'enabled' : ''}`}
          title={recordingRule?.enabled ? 'Disable auto-record' : 'Enable auto-record'}
          aria-label={recordingRule?.enabled ? 'Disable auto-record' : 'Enable auto-record'}
          onclick={onToggleAutoRecord}
        >
          <AlarmClock size={18} />
        </button>
        <button
          type="button"
          class={`icon-btn record-btn ${getRecordingClass()}`}
          title={getRecordingTitle()}
          aria-label={getRecordingLabel()}
          onclick={onToggleManualRecording}
        >
          <Circle size={16} fill="currentColor" />
        </button>
        {#if channel.removable}
          <button
            type="button"
            class="icon-btn remove-btn"
            onclick={onRemove}
            title="Remove channel"
            aria-label="Remove channel"
          >
            <X size={18} />
          </button>
        {/if}
      </div>
    </div>

    <div class="channel-content-body">
      {#if status?.live && status.title}
        <p class="channel-title" title={status.title}>{status.title}</p>
      {/if}
      <p class="channel-subtitle">
        {#if status?.live && status.game}
          Playing: {status.game}
        {:else if status?.live && status.viewer_count}
          {status.viewer_count.toLocaleString()} viewers
        {:else}
          Offline
        {/if}
      </p>
    </div>
  </div>
</article>

<style>
  .channel-card {
    display: grid;
    grid-template-columns: 74px minmax(0, 1fr);
    align-items: stretch;
    gap: 0.75rem;
    height: 5.5rem;
    border: 1px solid color-mix(in srgb, var(--border) 58%, transparent);
    background: color-mix(in srgb, var(--bg-soft) 62%, #0a101b);
    border-radius: 0.75rem;
    padding: 0.8rem;
    transition: border-color 0.2s ease, transform 0.15s ease;
  }

  .channel-card:hover {
    transform: translateY(-1px);
  }

  .channel-card.live {
    border-left: 3px solid var(--success);
  }

  .channel-avatar-wrap {
    height: 100%;
    min-height: 74px;
    display: flex;
    align-items: center;
    position: relative;
  }

  .channel-avatar {
    width: 74px;
    height: 74px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--surface-2) 70%, transparent);
  }

  .avatar-status-dot {
    position: absolute;
    bottom: 4px;
    right: 4px;
    width: 14px;
    height: 14px;
    background: var(--success);
    border-radius: 50%;
    border: 2px solid var(--bg-soft);
    animation: pulse 1.5s ease-in-out infinite;
  }

  .channel-content {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    min-width: 0;
    overflow: hidden;
    min-height: 74px;
  }

  .channel-content-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 0.5rem;
    min-width: 0;
  }

  .channel-name-area {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .channel-name {
    margin: 0;
    padding: 0;
    border: 0;
    background: transparent;
    font-size: 0.9rem;
    font-weight: 600;
    text-transform: lowercase;
    color: var(--fg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: left;
    cursor: pointer;
  }

  .channel-name:hover {
    text-decoration: underline;
  }

  .channel-meta {
    margin: 0.2rem 0 0;
    color: var(--muted);
    font-size: 0.74rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }

  .channel-content-body {
    min-height: 2.15rem;
  }

  .channel-title {
    display: block;
    width: 100%;
    margin: 0.2rem 0 0;
    color: color-mix(in srgb, var(--fg) 85%, var(--muted));
    font-size: 0.82rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .channel-subtitle {
    margin: 0.15rem 0 0;
    color: var(--muted);
    font-size: 0.87rem;
  }

  .channel-controls {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    flex-shrink: 0;
    --ctrl-h: 2.35rem;
    --ctrl-r: 0.5rem;
    --ctrl-border: rgba(160, 181, 216, 0.32);
    --ctrl-bg: rgba(14, 22, 36, 0.92);
    --ctrl-fg: var(--fg);
  }

  .icon-btn {
    width: var(--ctrl-h);
    height: var(--ctrl-h);
    border: 1px solid var(--ctrl-border);
    border-radius: var(--ctrl-r);
    font-size: 0.9rem;
    line-height: 1;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--ctrl-bg);
    color: var(--ctrl-fg);
    cursor: pointer;
    transition:
      border-color 0.15s ease,
      background-color 0.15s ease,
      width 0.2s ease,
      opacity 0.2s ease,
      margin 0.2s ease;
  }

  .play-btn {
    background: var(--accent);
    color: #1e2030;
    border-color: transparent;
  }

  .play-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--accent) 85%, white);
  }

  .play-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .play-btn :global(.spinning) {
    animation: spin 0.8s linear infinite;
  }

  .clock-btn.enabled {
    background: color-mix(in srgb, var(--accent) 46%, var(--ctrl-bg));
    border-color: color-mix(in srgb, var(--accent) 68%, white);
    color: #eaf2ff;
  }

  .record-btn {
    color: color-mix(in srgb, var(--danger) 65%, var(--muted));
    background: color-mix(in srgb, var(--danger) 8%, var(--ctrl-bg));
    border-color: color-mix(in srgb, var(--danger) 25%, var(--ctrl-border));
  }

  .record-btn.active-auto {
    background: color-mix(in srgb, #f3b35f 74%, #1e2030);
    border-color: color-mix(in srgb, #f3b35f 76%, #fff);
    color: #fff;
  }

  .record-btn.active-manual {
    background: color-mix(in srgb, var(--danger) 74%, #1e2030);
    border-color: color-mix(in srgb, var(--danger) 75%, #fff);
    color: #fff;
  }

  .icon-btn:hover {
    border-color: var(--accent-border);
    background: color-mix(in srgb, var(--ctrl-bg) 82%, #101b30);
  }

  .play-btn:hover {
    border-color: transparent;
  }

  .icon-btn:focus-visible {
    outline: none;
    box-shadow: 0 0 0 3px var(--focus-ring);
  }

  .remove-btn {
    width: 0;
    opacity: 0;
    margin-left: 0;
    pointer-events: none;
    overflow: hidden;
    border-color: color-mix(in srgb, var(--border) 60%, transparent);
    color: var(--muted);
  }

  .channel-card:hover .remove-btn,
  .channel-controls:focus-within .remove-btn,
  .remove-btn:focus-visible {
    width: var(--ctrl-h);
    opacity: 1;
    margin-left: 0;
    pointer-events: auto;
    overflow: visible;
  }

  .remove-btn:hover {
    border-color: color-mix(in srgb, var(--danger) 60%, transparent);
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 10%, var(--ctrl-bg));
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  @media (hover: none) {
    .remove-btn {
      width: var(--ctrl-h);
      opacity: 1;
      margin-left: 0;
      pointer-events: auto;
      overflow: visible;
    }
  }

  @media (max-width: 600px) {
    .channel-card {
      grid-template-columns: 64px minmax(0, 1fr);
      align-items: stretch;
    }

    .channel-avatar-wrap {
      grid-row: span 2;
      min-height: 96px;
    }

    .channel-avatar {
      width: 64px;
      height: 64px;
    }

    .channel-content {
      min-height: 0;
      grid-column: 2;
    }

    .channel-content-header {
      flex-wrap: wrap;
    }

    .channel-controls {
      --ctrl-h: 2.15rem;
      --ctrl-r: 0.5rem;
      gap: 0.35rem;
    }

    .channel-controls button:not(.remove-btn) {
      flex: 1;
    }

    .remove-btn {
      width: var(--ctrl-h);
      opacity: 1;
      margin-left: 0;
      pointer-events: auto;
      overflow: visible;
    }
  }
</style>
