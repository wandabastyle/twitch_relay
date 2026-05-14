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
  import Star from 'lucide-svelte/icons/star';
  import Play from 'lucide-svelte/icons/play';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import Wrench from 'lucide-svelte/icons/wrench';
  import Square from 'lucide-svelte/icons/square';
  import CheckSquare from 'lucide-svelte/icons/check-square';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';

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
  onRequestDeleteRecordingFile,
  onToggleRecordingPin,
  onRepairRecording,
  onToggleIncompleteMergeSelection,
  onRequestProcessIncompleteFiles
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
    <div class="select-wrapper">
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
      <span class="select-chevron" aria-hidden="true">
        <ChevronDown size={14} />
      </span>
    </div>
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
        <p class="ui-muted section-empty">No active recordings right now.</p>
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
        <p class="ui-muted section-empty">No completed files yet.</p>
      {:else}
        <ul class="recordings-list">
          {#each shownCompleted as file (file.path_display)}
            {@const deleteKey = recordingDeleteKey('completed', file)}
            <li class="recordings-item-with-action">
              <div class="recording-entry">
                <div class="recording-title-row">
                  <span class="entry-main" title={file.filename}>{file.filename}</span>
                  <div class="recording-badges">
                    {#if file.processing_state === 'processing'}
                      <span class="badge badge-processing">Processing</span>
                    {/if}
                    {#if file.pinned}
                      <span class="badge badge-pinned">Pinned</span>
                    {/if}
                    {#if !file.has_hls}
                      <span class="badge badge-repair">Needs repair</span>
                    {/if}
                  </div>
                </div>
                <span class="entry-meta" title={file.path_display}>{file.path_display}</span>
              </div>
              <div class="recording-item-actions">
                <button
                  type="button"
                  class="recording-pin-btn"
                  class:pinned={file.pinned}
                  onclick={() => onToggleRecordingPin(file)}
                  title={file.pinned ? 'Unpin recording' : 'Pin recording'}
                  aria-label={file.pinned ? 'Unpin recording' : 'Pin recording'}
                  aria-pressed={file.pinned}
                  aria-busy={pinningRecordingKey === deleteKey}
                  disabled={pinningRecordingKey === deleteKey || file.processing_state === 'processing'}
                >
                  <Star size={16} fill={file.pinned ? 'currentColor' : 'none'} />
                </button>
                <button
                  type="button"
                  class="recording-play-btn"
                  onclick={() => onOpenRecordingPlayer(file)}
                  title="Play recording"
                  aria-label="Play recording"
                  disabled={file.processing_state === 'processing'}
                >
                  <Play size={14} />
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
                    {#if repairingRecordingKey === deleteKey}
                      <span class="repair-spinner"></span>
                    {:else}
                      <Wrench size={14} />
                    {/if}
                  </button>
                {/if}
                <button
                  type="button"
                  class="recording-delete-btn"
                  onclick={() => onRequestDeleteRecordingFile('completed', file)}
                  title="Delete recording"
                  aria-label="Delete recording"
                  aria-busy={deletingRecordingKey === deleteKey}
                  disabled={deletingRecordingKey === deleteKey || file.processing_state === 'processing'}
                >
                  {#if deletingRecordingKey === deleteKey}
                    <span class="delete-spinner"></span>
                  {:else}
                    <Trash2 size={14} />
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
               onclick={() => onRequestProcessIncompleteFiles(recordingsChannelFilter)}
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
           <p class="ui-muted section-empty">No incomplete files.</p>
         {:else}
         <ul class="recordings-list">
           {#each shownIncomplete as file (file.path_display)}
             {@const deleteKey = recordingDeleteKey('incomplete', file)}
                <li class="recordings-item-with-action">
                  <div class="recording-entry-incomplete">
                    <div class="recording-title-row">
                      <span class="entry-main" title={file.filename}>{file.filename}</span>
                    </div>
                    <span class="entry-meta" title={file.path_display}>{file.path_display}</span>
                  </div>
                  <div class="recording-item-actions">
                    {#if recordingsChannelFilter !== "all"}
                      <button
                        type="button"
                        class="recording-select-btn"
                        class:selected={selectedIncompleteFilenames.has(file.filename)}
                        onclick={() => onToggleIncompleteMergeSelection(file.filename)}
                        disabled={mergingRecordingKey === recordingsChannelFilter}
                        title={selectedIncompleteFilenames.has(file.filename) ? 'Deselect' : 'Select'}
                        aria-label={selectedIncompleteFilenames.has(file.filename) ? 'Deselect file' : 'Select file'}
                        aria-pressed={selectedIncompleteFilenames.has(file.filename)}
                      >
                        {#if selectedIncompleteFilenames.has(file.filename)}
                          <CheckSquare size={14} />
                        {:else}
                          <Square size={14} />
                        {/if}
                      </button>
                    {/if}
                    <button
                      type="button"
                      class="recording-delete-btn"
                      onclick={() => onRequestDeleteRecordingFile('incomplete', file)}
                      title="Delete recording"
                      aria-label="Delete recording"
                      aria-busy={deletingRecordingKey === deleteKey}
                      disabled={deletingRecordingKey === deleteKey}
                    >
                     {#if deletingRecordingKey === deleteKey}
                       <span class="delete-spinner"></span>
                     {:else}
                       <Trash2 size={14} />
                     {/if}
                   </button>
                  </div>
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

  .select-wrapper {
    position: relative;
    width: min(22rem, 100%);
  }

  .select-chevron {
    position: absolute;
    right: 0.75rem;
    top: 50%;
    transform: translateY(-50%);
    color: var(--muted);
    pointer-events: none;
  }

  .recordings-filter-select {
    width: 100%;
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    background: color-mix(in srgb, var(--bg-soft) 70%, var(--surface));
    color: var(--fg);
    border-radius: 0.6rem;
    padding: 0.6rem 2.5rem 0.6rem 0.7rem;
    font: inherit;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    transition: border-color 0.15s ease, background-color 0.15s ease;
  }

  .recordings-filter-select:hover {
    border-color: var(--accent-border);
  }

  .recordings-filter-select:focus {
    outline: none;
    border-color: var(--accent-border);
    box-shadow: 0 0 0 2px var(--focus-ring);
  }

  .recordings-filter-select:disabled {
    opacity: 0.55;
    cursor: not-allowed;
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

     .recording-entry {
      display: grid;
      gap: 0.2rem;
      min-width: 0;
    }

    .recording-title-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      flex-wrap: wrap;
      min-width: 0;
    }

    .recording-title-row .entry-main {
      min-width: 0;
      flex: 1;
    }

    .recording-badges {
      display: flex;
      gap: 0.35rem;
      flex-wrap: wrap;
    }

    .recording-entry-incomplete {
      display: grid;
      gap: 0.2rem;
      min-width: 0;
    }

    /* Status badges */
    .badge {
      display: inline-flex;
      align-items: center;
      padding: 0.15rem 0.45rem;
      border-radius: 0.25rem;
      font-size: 0.7rem;
      font-weight: 600;
      line-height: 1.2;
      text-transform: uppercase;
      letter-spacing: 0.02em;
      white-space: nowrap;
    }

    .badge-processing {
      background: color-mix(in srgb, var(--warn) 20%, transparent);
      color: var(--warn);
      border: 1px solid color-mix(in srgb, var(--warn) 40%, transparent);
    }

    .badge-pinned {
      background: color-mix(in srgb, var(--accent) 20%, transparent);
      color: var(--accent);
      border: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
    }

    .badge-repair {
      background: color-mix(in srgb, var(--danger) 15%, transparent);
      color: var(--danger);
      border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    }

    /* Section empty state - compact */
    .section-empty {
      margin: 0.5rem 0;
      padding: 0.5rem 0;
    }

    /* Recordings list */
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
      align-items: flex-start;
      gap: 0.5rem;
    }

    /* Entry text styles */
    .entry-main {
      font-size: 0.88rem;
      color: var(--fg);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      display: block;
    }

    .entry-meta {
      font-size: 0.8rem;
      color: var(--muted);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      display: block;
    }

    /* Recording action buttons */
    .recording-item-actions {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      margin-top: 0.1rem;
    }

    .recording-pin-btn,
    .recording-play-btn,
    .recording-delete-btn,
    .recording-select-btn {
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
      cursor: pointer;
      transition: border-color 0.15s ease, background-color 0.15s ease, color 0.15s ease;
    }

    .recording-pin-btn:hover:not(:disabled),
    .recording-play-btn:hover:not(:disabled),
    .recording-select-btn:hover:not(:disabled) {
      border-color: color-mix(in srgb, var(--accent) 68%, white);
      background: color-mix(in srgb, var(--accent) 34%, #1b2436);
      color: var(--fg);
    }

    .recording-delete-btn:hover:not(:disabled) {
      border-color: color-mix(in srgb, var(--danger) 68%, white);
      background: rgba(35, 14, 22, 0.9);
      color: var(--danger);
    }

    .recording-pin-btn:disabled,
    .recording-play-btn:disabled,
    .recording-delete-btn:disabled,
    .recording-select-btn:disabled {
      opacity: 0.55;
      cursor: not-allowed;
    }

    .recording-pin-btn.pinned,
    .recording-select-btn.selected {
      color: var(--accent);
    }

    /* Spinner for delete/repair buttons */
    .delete-spinner,
    .repair-spinner {
      width: 0.9rem;
      height: 0.9rem;
      border: 2px solid rgba(255, 255, 255, 0.2);
      border-top: 2px solid var(--fg);
      border-radius: 50%;
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

    @media (max-width: 600px) {
      .recordings-header {
        align-items: flex-start;
      }

      .recording-title-row {
        flex-direction: column;
        align-items: flex-start;
        gap: 0.35rem;
      }

      .recording-badges {
        width: 100%;
      }

      .recordings-item-with-action {
        grid-template-columns: 1fr;
        gap: 0.35rem;
      }

      .recording-item-actions {
        justify-content: flex-start;
        margin-top: 0.35rem;
      }
    }
  </style>
