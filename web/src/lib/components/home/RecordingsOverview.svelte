<script lang="ts">
    import type {
    RecordingsOverviewProps,
    ActiveRecording,
    RecordingFileEntry
  } from './types';
  import {
    latestThree,
    recordingDeleteKey,
    recordingChannelOptions,
    filterRecordingsByChannel,
    shownRecordingEntries
  } from '$lib/home/recordings';
  import LoadedFade from '$lib/components/LoadedFade.svelte';

let {
  activeRecordings,
  completedRecordings,
  incompleteRecordings,
  recordingsChannelFilter,
  deletingRecordingKey,
  pinningRecordingKey,
  repairingRecordingKey,
  mergingRecordingKey,
  selectedIncompleteFilenames,
  pendingJob,
  onBackToChannels,
  onUpdateFilter,
  onOpenRecordingPlayer,
  onRemoveRecordingFile,
  onToggleRecordingPin,
  onRepairRecording,
  onToggleIncompleteMergeSelection,
  onProcessIncompleteFiles
}: RecordingsOverviewProps = $props();

  const channelOptions = $derived(recordingChannelOptions(
    completedRecordings,
    incompleteRecordings,
    activeRecordings
  ));
  const activeList = $derived(filterRecordingsByChannel(Object.values(activeRecordings), recordingsChannelFilter));
  const completedList = $derived(filterRecordingsByChannel(completedRecordings, recordingsChannelFilter));
  const incompleteList = $derived(filterRecordingsByChannel(incompleteRecordings, recordingsChannelFilter));
  const shownActive = $derived(shownRecordingEntries(Object.values(activeRecordings), recordingsChannelFilter));
  const shownCompleted = $derived(shownRecordingEntries(completedRecordings, recordingsChannelFilter));
  const shownIncomplete = $derived(shownRecordingEntries(incompleteRecordings, recordingsChannelFilter));
</script>

<div class="recordings-view">
    <div class="recordings-header">
    <div>
      <span class="ui-section-title">Recordings overview</span>
      <p class="recordings-subtle">Recent recording activity and files</p>
    </div>
    <button type="button" class="ui-nav-chip" onclick={onBackToChannels}>Back to channels</button>
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
      {#each channelOptions as channelLogin (channelLogin)}
        <option value={channelLogin}>{channelLogin}</option>
      {/each}
    </select>
    <p class="recordings-filter-hint">All channels shows latest 3 per section.</p>
  </div>

  <LoadedFade loaded={true}>
    <div class="recordings-grid">
      {#if pendingJob}
        <section class="recordings-section">
          <h2>Pending {pendingJob.kind}</h2>
          <p class="ui-muted">
            {pendingJob.channelLogin}: {pendingJob.status} ({pendingJob.sourceCount} files) ->
            {pendingJob.expectedFilename}
          </p>
        </section>
      {/if}

    <section class="recordings-section">
      <h2>Active ({activeList.length})</h2>
      {#if activeList.length === 0}
        <p class="ui-muted">No active recordings right now.</p>
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
        <p class="ui-muted">No completed files yet.</p>
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
                  disabled={pinningRecordingKey === deleteKey || file.processing_state === 'processing'}
                >
                  {file.pinned ? '★' : '☆'}
                </button>
                <button
                  type="button"
                  class="recording-play-btn"
                  onclick={() => onOpenRecordingPlayer(file)}
                  title="Play recording"
                  aria-label="Play recording"
                  disabled={file.processing_state === 'processing'}
                >
                  Play
                </button>
                {#if file.processing_state === 'processing' || !file.has_hls}
                  <button
                    type="button"
                    class="recording-play-btn"
                    onclick={() => onRepairRecording(file)}
                    title="Repair playback assets"
                    aria-label="Repair playback assets"
                    aria-busy={repairingRecordingKey === deleteKey}
                    disabled={repairingRecordingKey === deleteKey}
                  >
                    {repairingRecordingKey === deleteKey ? 'Repairing...' : 'Repair'}
                  </button>
                {/if}
                <button
                  type="button"
                  class="recording-delete-btn"
                  onclick={() => onRemoveRecordingFile('completed', file)}
                  title="Delete recording"
                  aria-label="Delete recording"
                  aria-busy={deletingRecordingKey === deleteKey}
                  disabled={deletingRecordingKey === deleteKey || file.processing_state === 'processing'}
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
       <div class="incomplete-section-header">
         <h2>Incomplete ({incompleteList.length})</h2>
         {#if recordingsChannelFilter !== "all" && shownIncomplete.length > 0}
           {@const selectedCount = Array.from(selectedIncompleteFilenames).filter(
             filename => shownIncomplete.some(file => file.filename === filename)
           ).length}
            <button
              type="button"
              class="merge-btn"
              onclick={() => onProcessIncompleteFiles(recordingsChannelFilter)}
              disabled={selectedCount < 1 || mergingRecordingKey === recordingsChannelFilter}
            >
              {#if mergingRecordingKey === recordingsChannelFilter}
                <span class="merge-btn-spinner"></span>
                {selectedCount === 1 ? 'Finalizing...' : 'Merging...'}
              {:else}
                {selectedCount === 1
                  ? 'Finalize selected'
                  : `Merge selected (${selectedCount})`}
              {/if}
            </button>
          {/if}
       </div>
       {#if incompleteList.length === 0}
         <p class="ui-muted">No incomplete files.</p>
       {:else}
         <ul class="recordings-list">
           {#each shownIncomplete as file (file.path_display)}
             {@const deleteKey = recordingDeleteKey('incomplete', file)}
             <li class="recordings-item-with-action">
               <div>
                 {#if recordingsChannelFilter !== "all"}
                   <input
                     type="checkbox"
                     class="merge-checkbox"
                     checked={selectedIncompleteFilenames.has(file.filename)}
                     onchange={() => onToggleIncompleteMergeSelection(file.filename)}
                     disabled={mergingRecordingKey === recordingsChannelFilter}
                   />
                 {/if}
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
  </LoadedFade>
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

  /* .channels-label, .nav-chip-btn, .nav-chip-btn:hover, .muted styles now provided by app.css via .ui-section-title, .ui-nav-chip, .ui-muted */

  .recordings-subtle {
    margin: 0.3rem 0 0;
    color: var(--muted);
    font-size: 0.84rem;
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

   .incomplete-section-header {
     display: flex;
     justify-content: space-between;
     align-items: center;
     margin-bottom: 0.55rem;
   }

   .merge-btn {
     height: 1.8rem;
     border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
     border-radius: 0.55rem;
     background: color-mix(in srgb, var(--bg-soft) 70%, #0e1624);
     color: var(--fg);
     padding: 0 0.62rem;
     font-size: 0.78rem;
     font-weight: 600;
     display: inline-flex;
     align-items: center;
     gap: 0.4rem;
   }

   .merge-btn:hover:not(:disabled) {
     border-color: color-mix(in srgb, var(--accent) 68%, white);
     background: color-mix(in srgb, var(--accent) 34%, #1b2436);
   }

   .merge-btn:disabled {
     opacity: 0.55;
     cursor: not-allowed;
   }

   .merge-btn-spinner {
     width: 0.8rem;
     height: 0.8rem;
     border: 2px solid rgba(255, 255, 255, 0.2);
     border-top: 2px solid rgba(255, 255, 255, 0.8);
     border-radius: 50%;
     animation: spin 0.8s linear infinite;
   }

   .merge-checkbox {
     width: 1rem;
     height: 1rem;
     margin-right: 0.5rem;
     vertical-align: middle;
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

  /* .muted style now provided by app.css via .ui-muted */

  @media (max-width: 600px) {
    .recordings-header {
      align-items: flex-start;
    }
  }
</style>
