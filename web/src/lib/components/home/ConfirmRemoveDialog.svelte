<script lang="ts">
  import type { ConfirmRemoveDialogProps } from './types';

  let {
    channelLogin,
    isRemoving,
    onConfirm,
    onCancel
  }: ConfirmRemoveDialogProps = $props();
</script>

{#if channelLogin}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div class="modal-overlay" onclick={onCancel} role="presentation">
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
      <p class="modal-text">Remove <strong>{channelLogin}</strong> from the channel list?</p>
      <div class="modal-actions">
        <button type="button" class="ghost" onclick={onCancel} disabled={isRemoving}>
          Cancel
        </button>
        <button type="button" class="danger" onclick={onConfirm} disabled={isRemoving}>
          {isRemoving ? 'Removing...' : 'Remove'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal {
    background: linear-gradient(160deg, rgba(20, 28, 43, 0.98), rgba(13, 18, 28, 0.98));
    border: 1px solid rgba(164, 182, 216, 0.3);
    border-radius: 1rem;
    padding: 1.5rem;
    max-width: 20rem;
    width: 90%;
  }

  .modal-text {
    margin: 0 0 1.25rem;
    color: var(--fg);
    line-height: 1.5;
  }

  .modal-text strong {
    text-transform: lowercase;
    color: var(--danger);
  }

  .modal-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }

  button {
    border: 0;
    border-radius: 0.6rem;
    padding: 0.62rem 0.95rem;
    background: var(--accent);
    color: #1e2030;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .ghost {
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    color: var(--fg);
  }

  .danger {
    background: color-mix(in srgb, var(--danger) 92%, #1e2030);
  }
</style>
