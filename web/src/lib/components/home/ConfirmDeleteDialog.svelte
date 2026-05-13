<script lang="ts">
  import type { RecordingFileEntry } from '$lib/api-client/types';

  interface Props {
    pendingDelete: { bucket: "completed" | "incomplete"; file: RecordingFileEntry } | null;
    isDeleting: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { pendingDelete, isDeleting, onConfirm, onCancel }: Props = $props();

  let isExiting = $state(false);

  $effect(() => {
    if (pendingDelete) {
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
    if (!isDeleting) {
      handleCancel();
    }
  }
</script>

{#if pendingDelete}
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
        Delete <strong>{pendingDelete.file.filename}</strong>?
      </p>
      <p class="modal-subtext">This action cannot be undone.</p>
      <div class="modal-actions">
        <button type="button" class="ui-ghost-btn" onclick={handleCancel} disabled={isDeleting}>
          Cancel
        </button>
        <button type="button" class="danger" onclick={handleConfirm} disabled={isDeleting}>
          {isDeleting ? 'Deleting...' : 'Delete'}
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
    color: var(--danger);
    word-break: break-word;
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

  .danger {
    background: color-mix(in srgb, var(--danger) 92%, #1e2030);
    color: white;
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
