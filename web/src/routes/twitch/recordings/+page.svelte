<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { RecordingsOverview } from '$lib/components/home/recordings';
  import TwitchPanel from '$lib/components/twitch/TwitchPanel.svelte';
  import { ConfirmDialog } from '$lib/components/ui';
  import TwitchRelayHeader from '$lib/components/twitch/TwitchRelayHeader.svelte';
  import ErrorState from '$lib/components/ui/ErrorState.svelte';
  import SkeletonRecordingList from '$lib/components/ui/SkeletonRecordingList.svelte';
  import { createRecordingsController } from '$lib/home/recordingsController.svelte';
  import type { RecordingFileEntry } from '$lib/api-client/types';

  let recordingsChannelFilter = $state<string>('all');
  let loadError = $state<string | null>(null);
  let isLoadingRecordings = $state(true);

  const recordingsController = createRecordingsController({
    setError: (msg) => {
      loadError = msg;
      console.error(msg);
    }
  });

  async function loadRecordings(): Promise<void> {
    isLoadingRecordings = true;
    loadError = null;
    try {
      await recordingsController.loadRecordingState();
    } catch (e) {
      loadError = e instanceof Error ? e.message : 'Failed to load recordings';
    } finally {
      isLoadingRecordings = false;
    }
  }

  onMount(loadRecordings);

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

<TwitchPanel>
  <TwitchRelayHeader
    subtitle="Recording activity and files"
    onToggleMode={() => goto('/youtube')}
  />

  {#if isLoadingRecordings}
    <SkeletonRecordingList sections={3} itemsPerSection={3} />
  {:else if loadError}
    <ErrorState
      message={loadError}
      onRetry={loadRecordings}
      isRetrying={isLoadingRecordings}
    />
  {:else}
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
      pendingDelete={recordingsController.pendingDelete}
      pendingMerge={recordingsController.pendingMerge}
      onBackToChannels={backToChannels}
      {onUpdateFilter}
      onOpenRecordingPlayer={openRecordingPlayer}
      onRequestDeleteRecordingFile={recordingsController.requestDeleteRecordingFile}
      onConfirmDeleteRecordingFile={recordingsController.confirmDeleteRecordingFile}
      onCancelDeleteRecordingFile={recordingsController.cancelDeleteRecordingFile}
      onToggleRecordingPin={recordingsController.toggleRecordingPin}
      onRepairRecording={recordingsController.repairRecording}
      onToggleIncompleteMergeSelection={recordingsController.toggleIncompleteMergeSelection}
      onRequestProcessIncompleteFiles={recordingsController.requestProcessIncompleteFiles}
      onConfirmProcessIncompleteFiles={recordingsController.confirmProcessIncompleteFiles}
      onCancelProcessIncompleteFiles={recordingsController.cancelProcessIncompleteFiles}
    />
  {/if}
</TwitchPanel>

<ConfirmDialog
  isOpen={recordingsController.pendingDelete !== null}
  isBusy={recordingsController.deletingRecordingKey !== null}
  onConfirm={recordingsController.confirmDeleteRecordingFile}
  onCancel={recordingsController.cancelDeleteRecordingFile}
  confirmText={recordingsController.deletingRecordingKey !== null ? 'Deleting...' : 'Delete'}
  confirmVariant="danger"
>
  <p>
    Delete <strong class="danger-text">{recordingsController.pendingDelete?.file.filename}</strong>?
  </p>
  <p class="subtle">This action cannot be undone.</p>
</ConfirmDialog>

<ConfirmDialog
  isOpen={recordingsController.pendingMerge !== null}
  isBusy={recordingsController.mergingRecordingKey !== null}
  onConfirm={recordingsController.confirmProcessIncompleteFiles}
  onCancel={recordingsController.cancelProcessIncompleteFiles}
  confirmText={recordingsController.mergingRecordingKey !== null
    ? 'Processing...'
    : (recordingsController.pendingMerge?.action === "finalize" ? "Finalize" : "Merge")}
>
  <p>
    {recordingsController.pendingMerge?.action === "finalize" ? "Finalize" : "Merge"}
    <strong>{recordingsController.pendingMerge?.filenames.length}</strong>
    incomplete recording(s) for
    <strong>{recordingsController.pendingMerge?.channelLogin}</strong>?
  </p>
  <p class="subtle">This action cannot be undone.</p>
</ConfirmDialog>
