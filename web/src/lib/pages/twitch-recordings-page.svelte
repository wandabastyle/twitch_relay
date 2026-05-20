<script lang="ts">
  import { onMount } from 'svelte';

  import type { RecordingFileEntry } from '$lib/api-client';
  import { createRecordingsController } from '$lib/home/recordings-controller.svelte';
  import { navigate } from '$lib/router/router.svelte';
  import { ConfirmDialog, ErrorState } from '$lib/components/ui';
  import { RecordingsOverview } from '$lib/components/home/recordings';
  import SkeletonRecordingList from '$lib/components/ui/skeleton-recording-list.svelte';
  import TwitchPanel from '$lib/components/twitch/twitch-panel.svelte';
  import TwitchRelayHeader from '$lib/components/twitch/twitch-relay-header.svelte';

  const DEFAULT_FILTER = 'all';
  const FAILED_TO_LOAD = 'Failed to load recordings';
  const INITIAL_SKELETON_SECTIONS = 3;
  const INITIAL_SKELETON_ITEMS = 3;

  let recordingsChannelFilter = $state<string>(DEFAULT_FILTER);
  let loadError = $state<string>();
  let isLoadingRecordings = $state(true);

  const recordingsController = createRecordingsController({
    setError: (msg: string | null) => {
      loadError = msg ?? undefined;
    },
  });

  const loadRecordings = async (): Promise<void> => {
    isLoadingRecordings = true;
    loadError = undefined;
    try {
      await recordingsController.loadRecordingState();
    } catch (error_) {
      const errorMessage = error_ instanceof Error ? error_.message : FAILED_TO_LOAD;
      loadError = errorMessage;
    } finally {
      isLoadingRecordings = false;
    }
  };

  onMount(loadRecordings);

  const backToChannels = (): void => {
    navigate('/twitch');
  };

  const openRecordingPlayer = (file: RecordingFileEntry): void => {
    const query = new URLSearchParams({
      channel_login: file.channel_login,
      filename: file.filename
    });
    navigate(`/twitch/recordings/play?${query.toString()}`);
  };

  const onUpdateFilter = (value: string): void => {
    recordingsChannelFilter = value;
  };
</script>

<TwitchPanel>
  <TwitchRelayHeader
    subtitle="Recording activity and files"
    onToggleMode={() => navigate('/youtube')}
  />

  {#if isLoadingRecordings}
    <SkeletonRecordingList sections={INITIAL_SKELETON_SECTIONS} itemsPerSection={INITIAL_SKELETON_ITEMS} />
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
  isOpen={recordingsController.pendingDelete !== undefined}
  isBusy={recordingsController.deletingRecordingKey !== undefined}
  onConfirm={recordingsController.confirmDeleteRecordingFile}
  onCancel={recordingsController.cancelDeleteRecordingFile}
    confirmText={
      recordingsController.deletingRecordingKey !== undefined ? 'Deleting...' : 'Delete'
    }
  confirmVariant="danger"
>
  <p>
    Delete <strong class="danger-text">{recordingsController.pendingDelete?.file.filename}</strong>?
  </p>
  <p class="subtle">This action cannot be undone.</p>
</ConfirmDialog>

<ConfirmDialog
  isOpen={recordingsController.pendingMerge !== undefined}
  isBusy={recordingsController.mergingRecordingKey !== undefined}
  onConfirm={recordingsController.confirmProcessIncompleteFiles}
  onCancel={recordingsController.cancelProcessIncompleteFiles}
  confirmText={
      recordingsController.mergingRecordingKey !== undefined
        ? 'Processing...'
        : (recordingsController.pendingMerge?.action === 'finalize' ? 'Finalize' : 'Merge')
    }
>
  <p>
    {recordingsController.pendingMerge?.action === 'finalize' ? 'Finalize' : 'Merge'}
    <strong>{recordingsController.pendingMerge?.filenames.length}</strong>
    incomplete recording(s) for
    <strong>{recordingsController.pendingMerge?.channelLogin}</strong>?
  </p>
  <p class="subtle">This action cannot be undone.</p>
</ConfirmDialog>
