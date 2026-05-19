<script lang="ts">
  import type { RecordingFileEntry } from '$lib/api-client/types';
  import { recordingDeleteKey } from '$lib/home/recordings';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import Square from 'lucide-svelte/icons/square';
  import CheckSquare from 'lucide-svelte/icons/check-square';

  interface Props {
    file: RecordingFileEntry;
    deletingRecordingKey: string | null;
    mergingRecordingKey: string | null;
    selectedIncompleteFilenames: Set<string>;
    recordingsChannelFilter: string;
    onToggleIncompleteMergeSelection: (filename: string) => void;
    onRequestDeleteRecordingFile: (bucket: 'completed' | 'incomplete', file: RecordingFileEntry) => void;
  }

  let {
    file,
    deletingRecordingKey,
    mergingRecordingKey,
    selectedIncompleteFilenames,
    recordingsChannelFilter,
    onToggleIncompleteMergeSelection,
    onRequestDeleteRecordingFile
  }: Props = $props();

  const deleteKey = $derived(recordingDeleteKey('incomplete', file));
  const isSelected = $derived(selectedIncompleteFilenames.has(file.filename));
</script>

<li class="recordings-item-with-action">
  <div class="recording-entry-incomplete">
    <div class="recording-title-row">
      <span class="entry-main" title={file.filename}>{file.filename}</span>
    </div>
    <span class="entry-meta" title={file.path_display}>{file.path_display}</span>
  </div>
  <div class="recording-item-actions">
    {#if recordingsChannelFilter !== 'all'}
      <button
        type="button"
        class="recording-select-btn"
        class:selected={isSelected}
        onclick={() => onToggleIncompleteMergeSelection(file.filename)}
        disabled={mergingRecordingKey === recordingsChannelFilter}
        title={isSelected ? 'Deselect' : 'Select'}
        aria-label={isSelected ? 'Deselect file' : 'Select file'}
        aria-pressed={isSelected}
      >
        {#if isSelected}
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

<style>
  .recording-entry-incomplete {
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

  .recording-item-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    margin-top: 0.1rem;
  }

  .recording-select-btn,
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
    cursor: pointer;
    transition: border-color 0.15s ease, background-color 0.15s ease, color 0.15s ease;
  }

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

  .recording-select-btn:disabled,
  .recording-delete-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .recording-select-btn.selected {
    color: var(--accent);
  }

  .delete-spinner {
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
    .recording-title-row {
      flex-direction: column;
      align-items: flex-start;
      gap: 0.35rem;
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
