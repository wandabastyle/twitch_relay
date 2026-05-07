<script lang="ts">
  import type {
    RecordingsOverviewProps,
    ActiveRecording,
    RecordingFileEntry
  } from './types';

  let {
    activeRecordings,
    completedRecordings,
    incompleteRecordings,
    recordingsChannelFilter,
    deletingRecordingKey,
    pinningRecordingKey,
    onBackToChannels,
    onUpdateFilter,
    onOpenRecordingPlayer,
    onRemoveRecordingFile,
    onToggleRecordingPin
  }: RecordingsOverviewProps = $props();

  function latestThree<T>(entries: Array<T>): Array<T> {
    return entries.slice(0, 3);
  }

  function recordingsChannelOptions(): Array<string> {
    const known: Record<string, true> = {};
    for (const item of completedRecordings) {
      known[item.channel_login] = true;
    }
    for (const item of incompleteRecordings) {
      known[item.channel_login] = true;
    }
    for (const item of Object.values(activeRecordings)) {
      known[item.channel_login] = true;
    }
    return Object.keys(known).sort((a, b) => a.localeCompare(b));
  }

  function withFilter<T extends { channel_login: string }>(entries: Array<T>): Array<T> {
    if (recordingsChannelFilter === 'all') {
      return entries;
    }
    return entries.filter((entry) => entry.channel_login === recordingsChannelFilter);
  }

  function shownEntries<T extends { channel_login: string }>(entries: Array<T>): Array<T> {
    const filtered = withFilter(entries);
    return recordingsChannelFilter === 'all' ? latestThree(filtered) : filtered;
  }

  function recordingDeleteKey(bucket: 'completed' | 'incomplete', file: RecordingFileEntry): string {
    return `${bucket}:${file.channel_login}:${file.filename}`;
  }

  const activeList = $derived(withFilter(Object.values(activeRecordings)));
  const completedList = $derived(withFilter(completedRecordings));
  const incompleteList = $derived(withFilter(incompleteRecordings));
  const shownActive = $derived(shownEntries(Object.values(activeRecordings)));
  const shownCompleted = $derived(shownEntries(completedRecordings));
  const shownIncomplete = $derived(shownEntries(incompleteRecordings));
</script>

<div class="recordings-view">
  <div class="recordings-header">
    <div>
      <span class="channels-label">Recordings overview</span>
      <p class="recordings-subtle">Recent recording activity and files</p>
    </div>
    <button type="button" class="nav-chip-btn" onclick={onBackToChannels}>Back to channels</button>
  </div>

  <div class="recordings-filter-row">
    <label class="recordings-filter-label" for="recordings-filter">Filter by channel</label>
    <select
      id="recordings-filter"
      class="recordings-filter-select"
      value={recordingsChannelFilter}
      onchange={(e) => onUpdateFilter(e.currentTarget.value)}
    >
      <option value="all">All channels</option>
      {#each recordingsChannelOptions() as channelLogin (channelLogin)}
        <option value={channelLogin}>{channelLogin}</option>
      {/each}
    </select>
    <p class="recordings-filter-hint">All channels shows latest 3 per section.</p>
  </div>

  <div class="recordings-grid">
    <section class="recordings-section">
      <h2>Active ({activeList.length})</h2>
      {#if activeList.length === 0}
        <p class="muted">No active recordings right now.</p>
      {:else}
        <ul class="recordings-list">
          {#each shownActive as recording (recording.channel_login)}
            <li>
              <span class="entry-main">{recording.channel_login}</span>
              <span class="entry-meta">{recording.mode} · {recording.quality}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="recordings-section">
      <h2>Completed ({completedList.length})</h2>
      {#if completedList.length === 0}
        <p class="muted">No completed files yet.</p>
      {:else}
        <ul class="recordings-list">
          {#each shownCompleted as file (file.path_display)}
            {@const deleteKey = recordingDeleteKey('completed', file)}
            <li class="recordings-item-with-action">
              <div>
                <span class="entry-main" title={file.filename}>{file.filename}</span>
                <span class="entry-meta" title={file.path_display}>{file.path_display}</span>
              </div>
              <div class="recording-item-actions">
                <button
                  type="button"
                  class="recording-pin-btn"
                  onclick={() => onToggleRecordingPin(file)}
                  title={file.pinned ? 'Unpin recording' : 'Pin recording'}
                  aria-label={file.pinned ? 'Unpin recording' : 'Pin recording'}
                  aria-pressed={file.pinned}
                  aria-busy={pinningRecordingKey === deleteKey}
                  disabled={pinningRecordingKey === deleteKey}
                >
                  {file.pinned ? '★' : '☆'}
                </button>
                <button
                  type="button"
                  class="recording-play-btn"
                  onclick={() => onOpenRecordingPlayer(file)}
                  title="Play recording"
                  aria-label="Play recording"
                >
                  Play
                </button>
                <button
                  type="button"
                  class="recording-delete-btn"
                  onclick={() => onRemoveRecordingFile('completed', file)}
                  title="Delete recording"
                  aria-label="Delete recording"
                  aria-busy={deletingRecordingKey === deleteKey}
                  disabled={deletingRecordingKey === deleteKey}
                >
                  {#if deletingRecordingKey === deleteKey}
                    <svg class="recording-delete-spinner" viewBox="0 0 24 24" aria-hidden="true">
                      <circle cx="12" cy="12" r="8" class="spinner-track"></circle>
                      <path d="M12 4a8 8 0 0 1 8 8" class="spinner-head"></path>
                    </svg>
                  {:else}
                    <svg class="recording-delete-icon" viewBox="0 0 24 24" aria-hidden="true">
                      <path d="M9 4h6"></path>
                      <path d="M5 7h14"></path>
                      <path d="M7 7l1 12h8l1-12"></path>
                      <path d="M10 10v6"></path>
                      <path d="M14 10v6"></path>
                    </svg>
                  {/if}
                </button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="recordings-section">
      <h2>Incomplete ({incompleteList.length})</h2>
      {#if incompleteList.length === 0}
        <p class="muted">No incomplete files.</p>
      {:else}
        <ul class="recordings-list">
          {#each shownIncomplete as file (file.path_display)}
            {@const deleteKey = recordingDeleteKey('incomplete', file)}
            <li class="recordings-item-with-action">
              <div>
                <span class="entry-main" title={file.filename}>{file.filename}</span>
                <span class="entry-meta" title={file.path_display}>{file.path_display}</span>
              </div>
              <button
                type="button"
                class="recording-delete-btn"
                onclick={() => onRemoveRecordingFile('incomplete', file)}
                title="Delete recording"
                aria-label="Delete recording"
                aria-busy={deletingRecordingKey === deleteKey}
                disabled={deletingRecordingKey === deleteKey}
              >
                {#if deletingRecordingKey === deleteKey}
                  <svg class="recording-delete-spinner" viewBox="0 0 24 24" aria-hidden="true">
                    <circle cx="12" cy="12" r="8" class="spinner-track"></circle>
                    <path d="M12 4a8 8 0 0 1 8 8" class="spinner-head"></path>
                  </svg>
                {:else}
                  <svg class="recording-delete-icon" viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M9 4h6"></path>
                    <path d="M5 7h14"></path>
                    <path d="M7 7l1 12h8l1-12"></path>
                    <path d="M10 10v6"></path>
                    <path d="M14 10v6"></path>
                  </svg>
                {/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
</div>

<style>
  .recordings-view {
    display: grid;
    gap: 0.85rem;
  }

  .recordings-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.65rem;
    flex-wrap: wrap;
  }

  .channels-label {
    font-weight: 600;
    color: var(--fg);
  }

  .recordings-subtle {
    margin: 0.3rem 0 0;
    color: var(--muted);
    font-size: 0.84rem;
  }

  .nav-chip-btn {
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--border) 78%, transparent);
    border-radius: 0.6rem;
    color: var(--fg);
    padding: 0.4rem 0.8rem;
    font: inherit;
    font-size: 0.85rem;
    font-weight: 600;
    line-height: 1;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 2rem;
  }

  .nav-chip-btn:hover {
    border-color: var(--accent-border);
    background: var(--accent-soft);
  }

  .recordings-grid {
    display: grid;
    gap: 0.75rem;
  }

  .recordings-filter-row {
    display: grid;
    gap: 0.35rem;
    margin-top: -0.1rem;
  }

  .recordings-filter-label {
    color: var(--muted);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .recordings-filter-select {
    width: min(22rem, 100%);
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    background: rgba(8, 12, 19, 0.9);
    color: var(--fg);
    border-radius: 0.6rem;
    padding: 0.6rem 0.7rem;
    font: inherit;
  }

  .recordings-filter-hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.78rem;
  }

  .recordings-section {
    border: 1px solid color-mix(in srgb, var(--border) 58%, transparent);
    background: color-mix(in srgb, var(--bg-soft) 62%, #0a101b);
    border-radius: 0.75rem;
    padding: 0.8rem;
  }

  .recordings-section h2 {
    margin: 0 0 0.55rem;
    font-size: 0.95rem;
    font-weight: 700;
  }

  .recordings-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.45rem;
  }

  .recordings-list li {
    display: grid;
    gap: 0.1rem;
  }

  .recordings-item-with-action {
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 0.5rem;
  }

  .recording-item-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
  }

  .recording-play-btn {
    height: 2rem;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    border-radius: 0.55rem;
    background: color-mix(in srgb, var(--bg-soft) 70%, #0e1624);
    color: var(--fg);
    padding: 0 0.62rem;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .recording-play-btn:hover {
    border-color: color-mix(in srgb, var(--accent) 68%, white);
    background: color-mix(in srgb, var(--accent) 34%, #1b2436);
  }

  .recordings-item-with-action > div {
    min-width: 0;
  }

  .recording-pin-btn {
    width: 2rem;
    height: 2rem;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    border-radius: 0.55rem;
    background: color-mix(in srgb, var(--bg-soft) 70%, #0e1624);
    color: var(--muted);
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.9rem;
    line-height: 1;
    cursor: pointer;
    transition: background-color 120ms ease, border-color 120ms ease, color 120ms ease;
  }

  .recording-pin-btn:hover {
    border-color: color-mix(in srgb, var(--accent) 68%, white);
    background: color-mix(in srgb, var(--accent) 34%, #1b2436);
    color: var(--fg);
  }

  .recording-pin-btn:disabled {
    opacity: 0.55;
    cursor: progress;
  }

  .entry-main {
    font-size: 0.88rem;
    color: var(--fg);
    white-space: normal;
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .entry-meta {
    font-size: 0.8rem;
    color: var(--muted);
    white-space: normal;
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .recording-delete-btn {
    width: 2rem;
    height: 2rem;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    border-radius: 0.55rem;
    background: color-mix(in srgb, var(--bg-soft) 70%, #0e1624);
    color: var(--muted);
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.9rem;
  }

  .recording-delete-btn:hover {
    border-color: color-mix(in srgb, var(--danger) 68%, white);
    color: var(--danger);
    background: rgba(35, 14, 22, 0.9);
  }

  .recording-delete-icon,
  .recording-delete-spinner {
    width: 0.95rem;
    height: 0.95rem;
    stroke: currentColor;
    fill: none;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .spinner-track {
    opacity: 0.28;
  }

  .spinner-head {
    opacity: 0.95;
  }

  .recording-delete-spinner {
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .muted {
    margin: 0;
    color: var(--muted);
  }

  @media (max-width: 600px) {
    .recordings-header {
      align-items: flex-start;
    }
  }
</style>
