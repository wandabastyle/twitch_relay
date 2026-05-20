<script lang="ts">
  import AlertCircle from 'lucide-svelte/icons/alert-circle';
  import RefreshCw from 'lucide-svelte/icons/refresh-cw';

  interface Props {
    isRetrying?: boolean;
    message: string;
    onRetry?: () => void;
    retryLabel?: string;
  }

  const {
    isRetrying = false,
    message,
    onRetry,
    retryLabel = 'Try again',
  }: Props = $props();
</script>

<div class="error-state" role="alert">
  <div class="error-icon">
    <AlertCircle size={20} />
  </div>
  <div class="error-content">
    <p class="error-message">{message}</p>
    {#if onRetry}
      <button
        type="button"
        class="retry-btn"
        onclick={onRetry}
        disabled={isRetrying}
        aria-busy={isRetrying}
      >
        {#if isRetrying}
          <span class="retry-spinner"></span>
        {:else}
          <RefreshCw size={14} />
        {/if}
        <span>{isRetrying ? 'Retrying...' : retryLabel}</span>
      </button>
    {/if}
  </div>
</div>

<style>
  .error-state {
    display: flex;
    gap: 0.75rem;
    padding: 1rem;
    background: rgba(255, 82, 82, 0.08);
    border: 1px solid color-mix(in srgb, var(--danger) 40%, transparent);
    border-radius: 0.6rem;
    align-items: flex-start;
  }

  .error-icon {
    color: var(--danger);
    flex-shrink: 0;
    margin-top: 0.1rem;
  }

  .error-content {
    display: grid;
    gap: 0.6rem;
    min-width: 0;
    flex: 1;
  }

  .error-message {
    margin: 0;
    color: var(--fg);
    line-height: 1.5;
    overflow-wrap: anywhere;
  }

  .retry-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.45rem 0.8rem;
    background: transparent;
    border: 1px solid color-mix(in srgb, var(--border) 72%, transparent);
    border-radius: 0.5rem;
    color: var(--fg);
    font: inherit;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition: border-color 0.15s ease, background-color 0.15s ease;
    width: fit-content;
  }

  .retry-btn:hover:not(:disabled) {
    border-color: var(--accent-border);
    background: var(--accent-soft);
  }

  .retry-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .retry-spinner {
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

  @media (prefers-reduced-motion: reduce) {
    .retry-spinner {
      animation: none;
    }
  }
</style>
