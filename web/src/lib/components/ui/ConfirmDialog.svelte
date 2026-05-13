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
  }

  let {
    isOpen,
    isBusy,
    onConfirm,
    onCancel,
    confirmText,
    confirmVariant = 'primary',
    cancelText = 'Cancel',
    children
  }: Props = $props();

  let isExiting = $state(false);

  $effect(() => {
    if (isOpen) {
      isExiting = false;
    }
  });

  function handleCancel(): void {
    if (isBusy) return;
    isExiting = true;
    setTimeout(() => {
      onCancel();
    }, 180);
  }

  function handleConfirm(): void {
    onConfirm();
  }

  function handleOverlayClick(): void {
    if (!isBusy) {
      handleCancel();
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && !isBusy) {
      handleCancel();
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
    onkeydown={handleKeydown}
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
      <div class="modal-content">
        {@render children()}
      </div>
      <div class="modal-actions">
        <button type="button" class="ui-ghost-btn" onclick={handleCancel} disabled={isBusy}>
          {cancelText}
        </button>
        <button
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
