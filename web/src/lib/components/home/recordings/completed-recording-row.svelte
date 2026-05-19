<script lang="ts">
  import type { RecordingFileEntry } from '$lib/api-client/types';
  import RecordingActions from './recording-actions.svelte';
  import RecordingBadges from './recording-badges.svelte';

  interface Props {
    file: RecordingFileEntry;
    deletingRecordingKey: string | undefined;
    pinningRecordingKey: string | undefined;
    repairingRecordingKey: string | undefined;
    onToggleRecordingPin: (file: RecordingFileEntry) => void;
    onOpenRecordingPlayer: (file: RecordingFileEntry) => void;
    onRepairRecording: (file: RecordingFileEntry) => void;
    onRequestDeleteRecordingFile: (bucket: 'completed' | 'incomplete', file: RecordingFileEntry) => void;
  }

  const {
    file,
    deletingRecordingKey,
    pinningRecordingKey,
    repairingRecordingKey,
    onToggleRecordingPin,
    onOpenRecordingPlayer,
    onRepairRecording,
    onRequestDeleteRecordingFile
  }: Props = $props();
</script>

<li class="recordings-item-with-action">
  <div class="recording-entry">
    <div class="recording-title-row">
      <span class="entry-main" title={file.filename}>{file.filename}</span>
      <RecordingBadges {file} />
    </div>
    <span class="entry-meta" title={file.path_display}>{file.path_display}</span>
  </div>
  <RecordingActions
    {file}
    {deletingRecordingKey}
    {pinningRecordingKey}
    {repairingRecordingKey}
    {onToggleRecordingPin}
    {onOpenRecordingPlayer}
    {onRepairRecording}
    {onRequestDeleteRecordingFile}
  />
</li>

<style>
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

  .recordings-item-with-action {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: flex-start;
    gap: 0.5rem;
  }

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

  @media (max-width: 600px) {
    .recording-title-row {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.35rem;
    }

    .recordings-item-with-action {
      grid-template-columns: 1fr;
      gap: 0.35rem;
    }
  }
</style>
