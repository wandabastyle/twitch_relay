<script lang="ts">
  import AlarmClock from 'lucide-svelte/icons/alarm-clock';
  import Circle from 'lucide-svelte/icons/circle';
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
  </div>

  <div class="channel-main">
    <div class="channel-main-top">
      <button type="button" class="channel-name" onclick={onOpenSetup}>
        {status?.display_name || channel.display_name || channel.login}
      </button>
    </div>
    <p class="channel-meta">
      {channel.source === 'manual' ? 'Manual' : channel.source === 'followed' ? 'Followed' : 'Manual + Followed'}
    </p>
    <div class="channel-main-bottom">
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

  <div class="channel-side">
    <div class="channel-side-top">
      {#if status?.live}
        <span class="live-badge">
          <span class="live-dot"></span>
          LIVE
        </span>
      {/if}
      <button
        type="button"
        class="watch-btn"
        onclick={onStartWatching}
        disabled={isWatching}
      >
        {isWatching ? 'Opening...' : 'Watch'}
      </button>
    </div>

    <div class="channel-actions">
      <div class="channel-controls">
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
  </div>
</article>

<style>
  .channel-card {
    display: grid;
    grid-template-columns: 74px minmax(0, 1fr) auto;
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

  .channel-card > * {
    min-width: 0;
  }

  .channel-avatar-wrap {
    height: 100%;
    min-height: 74px;
    display: flex;
    align-items: center;
  }

  .channel-avatar {
    width: 74px;
    height: 74px;
    border-radius: 50%;
    /* object-fit: cover, display: block from .ui-avatar; background override */
    background: color-mix(in srgb, var(--surface-2) 70%, transparent);
  }

  /* .channel-avatar.fallback styles now provided by .ui-avatar-fallback */

  .channel-main {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    min-width: 0;
    overflow: hidden;
    min-height: 74px;
  }

  .channel-main-top {
    display: flex;
    align-items: center;
    min-height: 1.6rem;
    min-width: 0;
    overflow: hidden;
  }

  .channel-main-bottom {
    min-height: 2.15rem;
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
    min-width: 0;
    flex: 1;
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

  .live-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    background: color-mix(in srgb, var(--success) 86%, transparent);
    color: #1e2030;
    font-size: 0.74rem;
    line-height: 1;
    font-weight: 700;
    height: 2rem;
    padding: 0 0.72rem;
    border-radius: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .live-dot {
    width: 6px;
    height: 6px;
    background: #1e2030;
    border-radius: 50%;
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
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

  .channel-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    justify-self: end;
    flex-shrink: 0;
  }

  .channel-side {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: flex-end;
    min-height: 74px;
    gap: 0.35rem;
  }

  .channel-side-top {
    min-height: 2rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .watch-btn {
    height: 2rem;
    border: 0;
    border-radius: 0.55rem;
    min-width: 4.7rem;
    padding: 0 0.8rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.9rem;
    font-weight: 700;
    letter-spacing: 0.01em;
    background: var(--accent);
    color: #1e2030;
    cursor: pointer;
  }

  .channel-controls {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
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

  .icon-btn:focus-visible,
  .watch-btn:focus-visible {
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
    .channel-controls {
      --ctrl-h: 2.15rem;
      --ctrl-r: 0.5rem;
      gap: 0.35rem;
    }

    .channel-avatar-wrap {
      grid-row: span 2;
      min-height: 96px;
    }

    .channel-avatar {
      width: 64px;
      height: 64px;
    }

    .channel-main {
      min-height: 0;
    }

    .channel-side {
      grid-column: 2;
      align-items: stretch;
      min-height: 0;
    }

    .channel-side-top {
      justify-content: flex-start;
    }

    .live-badge,
    .watch-btn {
      height: 1.9rem;
    }

    .live-badge {
      padding: 0 0.62rem;
      font-size: 0.7rem;
    }

    .channel-actions {
      width: 100%;
      gap: 0.45rem;
    }

    .channel-actions button:not(.remove-btn) {
      flex: 1;
    }

    .channel-controls {
      --ctrl-h: 2.15rem;
      --ctrl-r: 0.56rem;
      gap: 0.4rem;
    }

    .watch-btn {
      min-width: 4.4rem;
      font-size: 0.84rem;
      padding: 0 0.65rem;
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
