<script lang="ts">
  import type { ConfirmRemoveDialogProps } from './types';

  let {
    channelLogin,
    isRemoving,
    onConfirm,
    onCancel
  }: ConfirmRemoveDialogProps = $props();

  // Local state to handle exit animation
  let isExiting = $state(false);

  // When channelLogin becomes truthy, reset isExiting
  $effect(() => {
    if (channelLogin) {
      isExiting = false;
    }
  });

  function handleCancel(): void {
    isExiting = true;
    // Wait for animation to complete before actually closing
    setTimeout(() => {
      onCancel();
    }, 180);
  }

  function handleConfirm(): void {
    // For confirm, we animate out after the action completes
    // Or immediately if already removing
    if (isRemoving) {
      onConfirm();
    } else {
      // Trigger confirm immediately
      onConfirm();
      // Note: the parent will set channelLogin to null when done
      // We could animate here but it adds complexity - keeping it simple
    }
  }

  function handleOverlayClick(): void {
    if (!isRemoving) {
      handleCancel();
    }
  }
</script>

{#if channelLogin}
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
      <p class="modal-text">Remove <strong>{channelLogin}</strong> from the channel list?</p>
      <div class="modal-actions">
        <button type="button" class="ui-ghost-btn" onclick={handleCancel} disabled={isRemoving}>
          Cancel
        </button>
        <button type="button" class="danger" onclick={handleConfirm} disabled={isRemoving}>
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

  /* .ghost button styles now provided by app.css via .ui-ghost-btn */

  .danger {
    background: color-mix(in srgb, var(--danger) 92%, #1e2030);
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
