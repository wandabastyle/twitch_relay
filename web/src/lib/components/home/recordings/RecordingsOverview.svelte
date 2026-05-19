<script lang="ts">
  import type {
    RecordingsOverviewProps,
    ActiveRecording,
    RecordingFileEntry
  } from '../types';
  import {
    recordingChannelOptions,
    filterRecordingsByChannel,
    shownRecordingEntries
  } from '$lib/home/recordings';
  import LoadedFade from '$lib/components/LoadedFade.svelte';
  import RecordingsFilter from './RecordingsFilter.svelte';
  import ActiveRecordingsSection from './ActiveRecordingsSection.svelte';
  import CompletedRecordingRow from './CompletedRecordingRow.svelte';
  import IncompleteRecordingRow from './IncompleteRecordingRow.svelte';

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

  const selectedCount = $derived(
    recordingsChannelFilter !== 'all' && shownIncomplete.length > 0
      ? Array.from(selectedIncompleteFilenames).filter(
          filename => shownIncomplete.some(file => file.filename === filename)
        ).length
      : 0
  );
</script>

<div class="recordings-view">
  <div class="recordings-header">
    <div>
      <span class="ui-section-title">Recordings overview</span>
      <p class="recordings-subtle">Recent recording activity and files</p>
    </div>
    <button type="button" class="ui-nav-chip" onclick={onBackToChannels}>Back to channels</button>
  </div>

  <RecordingsFilter {channelOptions} {recordingsChannelFilter} {onUpdateFilter} />

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

      <ActiveRecordingsSection {activeList} {shownActive} />

      <section class="recordings-section">
        <h2>Completed ({completedList.length})</h2>
        {#if completedList.length === 0}
          <p class="ui-muted section-empty">No completed files yet.</p>
        {:else}
          <ul class="recordings-list">
            {#each shownCompleted as file (file.path_display)}
              <CompletedRecordingRow
                {file}
                {deletingRecordingKey}
                {pinningRecordingKey}
                {repairingRecordingKey}
                {onToggleRecordingPin}
                {onOpenRecordingPlayer}
                {onRepairRecording}
                {onRequestDeleteRecordingFile}
              />
            {/each}
          </ul>
        {/if}
      </section>

      <section class="recordings-section">
        <div class="incomplete-section-header">
          <h2>Incomplete ({incompleteList.length})</h2>
          {#if recordingsChannelFilter !== 'all' && shownIncomplete.length > 0}
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
                {selectedCount === 1 ? 'Finalize selected' : `Merge selected (${selectedCount})`}
              {/if}
            </button>
          {/if}
        </div>
        {#if incompleteList.length === 0}
          <p class="ui-muted section-empty">No incomplete files.</p>
        {:else}
          <ul class="recordings-list">
            {#each shownIncomplete as file (file.path_display)}
              <IncompleteRecordingRow
                {file}
                {deletingRecordingKey}
                {mergingRecordingKey}
                {selectedIncompleteFilenames}
                {recordingsChannelFilter}
                {onToggleIncompleteMergeSelection}
                {onRequestDeleteRecordingFile}
              />
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

  .recordings-subtle {
    margin: 0.3rem 0 0;
    color: var(--muted);
    font-size: 0.84rem;
  }

  .recordings-grid {
    display: grid;
    gap: 0.75rem;
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

  .section-empty {
    margin: 0.5rem 0;
    padding: 0.5rem 0;
  }

  .recordings-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.45rem;
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
  }
</style>
