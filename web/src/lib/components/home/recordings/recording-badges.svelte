<script lang="ts">
  import type { RecordingFileEntry } from '$lib/api-client/types';

  interface Props {
    file: RecordingFileEntry;
  }

  const { file }: Props = $props();
</script>

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

<style>
  .recording-badges {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

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

  @media (max-width: 600px) {
    .recording-badges {
      width: 100%;
    }
  }
</style>
