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
  import EmptyState from '../ui/EmptyState.svelte';

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
        <EmptyState
          title="No active recordings"
          description="Recordings will appear here when you start capturing a stream."
          variant="recordings"
        />
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
        <div class="section-empty-state">
          <EmptyState
            title="No completed recordings"
            description="Finished recordings appear here after a stream ends."
            variant="recordings"
          />
        </div>
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
                  onclick={() => onRequestDeleteRecordingFile('completed', file)}
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
          <div class="section-empty-state">
            <EmptyState
              title="No incomplete recordings"
              description="Incomplete recordings from interrupted streams will appear here."
              variant="recordings"
            />
          </div>
        {:else}
         <ul class="recordings-list">
           {#each shownIncomplete as file (file.path_display)}
             {@const deleteKey = recordingDeleteKey('incomplete', file)}
              <li class="recordings-item-with-action">
                <div class="recording-entry-incomplete">
                  <div class="incomplete-row">
                    {#if recordingsChannelFilter !== "all"}
                      <input
                        type="checkbox"
                        class="merge-checkbox"
                        checked={selectedIncompleteFilenames.has(file.filename)}
                        onchange={() => onToggleIncompleteMergeSelection(file.filename)}
                        disabled={mergingRecordingKey === recordingsChannelFilter}
                      />
                    {/if}
                    <div class="recording-title-row">
                      <span class="entry-main" title={file.filename}>{file.filename}</span>
                      <span class="badge badge-incomplete">Incomplete</span>
                    </div>
                  </div>
                  <span class="entry-meta" title={file.path_display}>{file.path_display}</span>
                </div>
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

    .incomplete-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
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

    .badge-incomplete {
      background: color-mix(in srgb, var(--muted) 20%, transparent);
      color: var(--muted);
      border: 1px solid color-mix(in srgb, var(--muted) 40%, transparent);
    }

    .section-empty-state {
      padding: 1.5rem 0;
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

      .incomplete-row {
        flex-wrap: wrap;
      }
    }
  </style>
