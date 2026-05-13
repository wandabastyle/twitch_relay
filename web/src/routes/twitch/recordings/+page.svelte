<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import RecordingsOverview from '$lib/components/home/RecordingsOverview.svelte';
  import ConfirmDeleteDialog from '$lib/components/home/ConfirmDeleteDialog.svelte';
  import ConfirmMergeDialog from '$lib/components/home/ConfirmMergeDialog.svelte';
  import TwitchPanel from '$lib/components/twitch/TwitchPanel.svelte';
  import TwitchRelayHeader from '$lib/components/twitch/TwitchRelayHeader.svelte';
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

<TwitchPanel>
  <TwitchRelayHeader
    subtitle="Recording activity and files"
    onToggleMode={() => goto('/youtube')}
  />

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
</TwitchPanel>

<ConfirmDeleteDialog
  pendingDelete={recordingsController.pendingDelete}
  isDeleting={recordingsController.deletingRecordingKey !== null}
  onConfirm={recordingsController.confirmDeleteRecordingFile}
  onCancel={recordingsController.cancelDeleteRecordingFile}
/>

<ConfirmMergeDialog
  pendingMerge={recordingsController.pendingMerge}
  isProcessing={recordingsController.mergingRecordingKey !== null}
  onConfirm={recordingsController.confirmProcessIncompleteFiles}
  onCancel={recordingsController.cancelProcessIncompleteFiles}
/>
