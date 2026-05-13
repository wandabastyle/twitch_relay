<script lang="ts">
  interface Props {
    pendingMerge: { channelLogin: string; action: "finalize" | "merge"; filenames: string[] } | null;
    isProcessing: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { pendingMerge, isProcessing, onConfirm, onCancel }: Props = $props();

  let isExiting = $state(false);

  $effect(() => {
    if (pendingMerge) {
      isExiting = false;
    }
  });

  function handleCancel(): void {
    isExiting = true;
    setTimeout(() => {
      onCancel();
    }, 180);
  }

  function handleConfirm(): void {
    onConfirm();
  }

  function handleOverlayClick(): void {
    if (!isProcessing) {
      handleCancel();
    }
  }
</script>

{#if pendingMerge}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="modal-overlay"
    class:entering={!isExiting}
    class:exiting={isExiting}
    onclick={handleOverlayClick}
    role="presentation"
  >
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <div
      class="modal"
      class:entering={!isExiting}
      class:exiting={isExiting}
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
    >
      <p class="modal-text">
        {pendingMerge.action === "finalize" ? "Finalize" : "Merge"}
        <strong>{pendingMerge.filenames.length}</strong>
        incomplete recording(s) for
        <strong>{pendingMerge.channelLogin}</strong>?
      </p>
      <p class="modal-subtext">This action cannot be undone.</p>
      <div class="modal-actions">
        <button type="button" class="ui-ghost-btn" onclick={handleCancel} disabled={isProcessing}>
          Cancel
        </button>
        <button type="button" class="primary" onclick={handleConfirm} disabled={isProcessing}>
          {isProcessing ? 'Processing...' : (pendingMerge.action === "finalize" ? "Finalize" : "Merge")}
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
    opacity: 0;
  }

  .modal-overlay.entering {
    animation: overlayFadeIn 180ms ease-out forwards;
  }

  .modal-overlay.exiting {
    animation: overlayFadeOut 180ms ease-in forwards;
  }

  .modal {
    background: linear-gradient(160deg, rgba(20, 28, 43, 0.98), rgba(13, 18, 28, 0.98));
    border: 1px solid rgba(164, 182, 216, 0.3);
    border-radius: 1rem;
    padding: 1.5rem;
    max-width: 20rem;
    width: 90%;
    opacity: 0;
    transform: scale(0.96);
  }

  .modal.entering {
    animation: modalEnter 180ms ease-out forwards;
  }

  .modal.exiting {
    animation: modalExit 180ms ease-in forwards;
  }

  @keyframes overlayFadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes overlayFadeOut {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }

  @keyframes modalEnter {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @keyframes modalExit {
    from {
      opacity: 1;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(0.96);
    }
  }

  .modal-text {
    margin: 0 0 0.5rem;
    color: var(--fg);
    line-height: 1.5;
  }

  .modal-text strong {
    color: var(--accent);
  }

  .modal-subtext {
    margin: 0 0 1.25rem;
    color: var(--muted);
    font-size: 0.9rem;
    line-height: 1.4;
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

  .primary {
    background: var(--accent);
    color: #1e2030;
  }

  @media (prefers-reduced-motion: reduce) {
    .modal-overlay,
    .modal {
      animation: none;
      opacity: 1;
      transform: none;
    }
  }
</style>
