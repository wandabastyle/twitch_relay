<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    isOpen: boolean;
    isBusy: boolean;
    onConfirm: () => void;
    onCancel: () => void;
    confirmText: string;
    confirmVariant?: 'primary' | 'danger';
    cancelText?: string;
    children: Snippet;
    initialFocus?: 'confirm' | 'cancel';
  }

  let {
    isOpen,
    isBusy,
    onConfirm,
    onCancel,
    confirmText,
    confirmVariant = 'primary',
    cancelText = 'Cancel',
    children,
    initialFocus = 'cancel'
  }: Props = $props();

  let isExiting = $state(false);
  let modalElement = $state<HTMLDivElement | null>(null);
  let confirmButton = $state<HTMLButtonElement | null>(null);
  let cancelButton = $state<HTMLButtonElement | null>(null);
  let lastFocusedElement = $state<HTMLElement | null>(null);

  $effect(() => {
    if (isOpen) {
      isExiting = false;
      // Store the element that had focus before opening
      lastFocusedElement = document.activeElement as HTMLElement;
    }
  });

  $effect(() => {
    if (isOpen && !isExiting && modalElement) {
      // Focus the initial element after the modal is rendered
      setTimeout(() => {
        if (initialFocus === 'confirm' && confirmButton) {
          confirmButton.focus({ preventScroll: true });
        } else if (cancelButton) {
          cancelButton.focus({ preventScroll: true });
        }
      }, 0);
    }
  });

  function handleCancel(): void {
    if (isBusy) return;
    isExiting = true;
    setTimeout(() => {
      onCancel();
      // Restore focus after modal closes
      if (lastFocusedElement && typeof lastFocusedElement.focus === 'function') {
        lastFocusedElement.focus({ preventScroll: true });
      }
    }, 180);
  }

  function handleConfirm(): void {
    if (isBusy) return;
    onConfirm();
    // Restore focus after modal closes
    setTimeout(() => {
      if (lastFocusedElement && typeof lastFocusedElement.focus === 'function') {
        lastFocusedElement.focus({ preventScroll: true });
      }
    }, 180);
  }

  function handleOverlayClick(): void {
    if (!isBusy) {
      handleCancel();
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && !isBusy) {
      event.preventDefault();
      handleCancel();
      return;
    }

    // Focus trapping: Tab cycles through focusable elements in modal
    if (event.key === 'Tab' && modalElement) {
      const focusableElements = modalElement.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      const firstElement = focusableElements[0] as HTMLElement;
      const lastElement = focusableElements[focusableElements.length - 1] as HTMLElement;

      if (event.shiftKey) {
        // Shift+Tab: if on first element, wrap to last
        if (document.activeElement === firstElement) {
          event.preventDefault();
          lastElement.focus({ preventScroll: true });
        }
      } else {
        // Tab: if on last element, wrap to first
        if (document.activeElement === lastElement) {
          event.preventDefault();
          firstElement.focus({ preventScroll: true });
        }
      }
    }
  }
</script>

{#if isOpen}
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
      bind:this={modalElement}
      class="modal"
      class:entering={!isExiting}
      class:exiting={isExiting}
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}
      role="dialog"
      aria-modal="true"
    >
      <div class="modal-content">
        {@render children()}
      </div>
      <div class="modal-actions">
        <button
          bind:this={cancelButton}
          type="button"
          class="ui-ghost-btn"
          onclick={handleCancel}
          disabled={isBusy}
        >
          {cancelText}
        </button>
        <button
          bind:this={confirmButton}
          type="button"
          class={confirmVariant}
          onclick={handleConfirm}
          disabled={isBusy}
        >
          {isBusy ? 'Processing...' : confirmText}
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
    max-width: 22rem;
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
    from { opacity: 0; }
    to { opacity: 1; }
  }

  @keyframes overlayFadeOut {
    from { opacity: 1; }
    to { opacity: 0; }
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

  .modal-content {
    margin-bottom: 1.25rem;
    min-width: 0;
  }

  .modal-content :global(p) {
    margin: 0 0 0.5rem;
    color: var(--fg);
    line-height: 1.5;
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .modal-content :global(p:last-child) {
    margin-bottom: 0;
  }

  .modal-content :global(.subtle) {
    color: var(--muted);
    font-size: 0.9rem;
  }

  .modal-content :global(strong) {
    color: var(--accent);
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .modal-content :global(.danger-text) {
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
    transition: opacity 0.15s ease;
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .primary {
    background: var(--accent);
    color: #1e2030;
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
