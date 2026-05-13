<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import ArrowLeftRight from 'lucide-svelte/icons/arrow-left-right';
  import RecordingsOverview from '$lib/components/home/RecordingsOverview.svelte';
  import { createRecordingsController } from '$lib/home/recordingsController.svelte';
  import type { RecordingFileEntry } from '$lib/api-client/types';

  let recordingsChannelFilter = $state<string>('all');

  const recordingsController = createRecordingsController({
    setError: (msg) => console.error(msg)
  });

  onMount(async () => {
    await recordingsController.loadRecordingState();
  });

  function backToChannels(): void {
    goto('/twitch');
  }

  function openRecordingPlayer(file: RecordingFileEntry): void {
    const query = new URLSearchParams({
      channel_login: file.channel_login,
      filename: file.filename
    });
    goto(`/twitch/recordings/play?${query.toString()}`);
  }

  function onUpdateFilter(value: string): void {
    recordingsChannelFilter = value;
  }
</script>

<svelte:head>
  <title>Recordings - Twitch Relay</title>
</svelte:head>

<section class="twitch-panel">
  <header class="panel-header">
    <div class="panel-title">
      <p class="eyebrow">Private Deck</p>
      <button
        type="button"
        class="relay-title-button"
        onclick={() => goto('/youtube')}
        aria-label="Switch to YouTube Relay"
        title="Switch to YouTube Relay"
      >
        <h1>Twitch Relay</h1>
        <span class="toggle-icon" aria-hidden="true">
          <ArrowLeftRight size={14} />
        </span>
      </button>
      <p class="header-subtle">Recording activity and files</p>
    </div>
  </header>

  <RecordingsOverview
    activeRecordings={recordingsController.activeRecordings}
    completedRecordings={recordingsController.completedRecordings}
    incompleteRecordings={recordingsController.incompleteRecordings}
    {recordingsChannelFilter}
    deletingRecordingKey={recordingsController.deletingRecordingKey}
    pinningRecordingKey={recordingsController.pinningRecordingKey}
    repairingRecordingKey={recordingsController.repairingRecordingKey}
    mergingRecordingKey={recordingsController.mergingRecordingKey}
    selectedIncompleteFilenames={recordingsController.selectedIncompleteFilenames}
    pendingJob={recordingsController.pendingJob}
    {backToChannels}
    {onUpdateFilter}
    {openRecordingPlayer}
    onRemoveRecordingFile={recordingsController.removeRecordingFile}
    onToggleRecordingPin={recordingsController.toggleRecordingPin}
    onRepairRecording={recordingsController.repairRecording}
    onToggleIncompleteMergeSelection={recordingsController.toggleIncompleteMergeSelection}
    onProcessIncompleteFiles={recordingsController.processSelectedIncompleteFiles}
  />
</section>

<style>
  .twitch-panel {
    width: min(46rem, 100%);
    background: linear-gradient(160deg, color-mix(in srgb, var(--surface) 95%, transparent), color-mix(in srgb, var(--bg-soft) 95%, transparent));
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    border-radius: 1rem;
    padding: 1.2rem;
    box-shadow: 0 1rem 2.5rem rgba(3, 8, 16, 0.45);
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    position: relative;
  }

  .panel-title {
    min-width: 0;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  .relay-title-button,
  .relay-title-button:hover,
  .relay-title-button:focus,
  .relay-title-button:active {
    text-decoration: none;
  }

  .relay-title-button {
    appearance: none;
    background: transparent;
    border: 0;
    padding: 0;
    margin: 0.2rem 0 0;
    font: inherit;
    font-weight: inherit;
    cursor: pointer;
    text-align: left;
    color: inherit;
    display: inline-flex;
    align-items: baseline;
    gap: 0.4rem;
  }

  .relay-title-button h1 {
    margin: 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  .toggle-icon {
    display: inline-flex;
    align-items: center;
    opacity: 0.45;
    transition: opacity 0.15s ease, transform 0.15s ease;
    color: var(--muted);
  }

  .relay-title-button:hover .toggle-icon {
    opacity: 0.9;
    color: var(--accent);
    transform: rotate(180deg);
  }

  .relay-title-button:hover {
    color: var(--accent);
  }

  .header-subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.86rem;
  }
</style>
