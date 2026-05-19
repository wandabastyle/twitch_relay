<script lang="ts">
  import type { RecordingFileEntry } from '$lib/api-client/types';
  import { recordingDeleteKey } from '$lib/home/recordings';
  import Star from 'lucide-svelte/icons/star';
  import Play from 'lucide-svelte/icons/play';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import Wrench from 'lucide-svelte/icons/wrench';

  interface Props {
    file: RecordingFileEntry;
    deletingRecordingKey: string | null;
    pinningRecordingKey: string | null;
    repairingRecordingKey: string | null;
    onToggleRecordingPin: (file: RecordingFileEntry) => void;
    onOpenRecordingPlayer: (file: RecordingFileEntry) => void;
    onRepairRecording: (file: RecordingFileEntry) => void;
    onRequestDeleteRecordingFile: (bucket: 'completed' | 'incomplete', file: RecordingFileEntry) => void;
  }

  let {
    file,
    deletingRecordingKey,
    pinningRecordingKey,
    repairingRecordingKey,
    onToggleRecordingPin,
    onOpenRecordingPlayer,
    onRepairRecording,
    onRequestDeleteRecordingFile
  }: Props = $props();

  const deleteKey = $derived(recordingDeleteKey('completed', file));
</script>

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

<style>
  .recording-item-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    margin-top: 0.1rem;
  }

  .recording-pin-btn,
  .recording-play-btn,
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

  .recording-pin-btn:hover:not(:disabled),
  .recording-play-btn:hover:not(:disabled) {
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
  .recording-delete-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .recording-pin-btn.pinned {
    color: var(--accent);
  }

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
    .recording-item-actions {
      justify-content: flex-start;
      margin-top: 0.35rem;
    }
  }
</style>
