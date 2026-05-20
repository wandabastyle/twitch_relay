<script lang="ts">
  import ArrowLeftRight from 'lucide-svelte/icons/arrow-left-right';
  import type { Snippet } from 'svelte';

  interface Props {
    eyebrow: string;
    title: string;
    /** Simple text subtitle */
    subtitleText?: string;
    /** Snippet subtitle (for complex content) */
    subtitleSnippet?: Snippet;
    onToggle: () => void;
    toggleLabel: string;
    /** Additional content in the header (e.g. action buttons) */
    children?: Snippet;
  }

  const { children, eyebrow, onToggle, subtitleSnippet, subtitleText, title, toggleLabel }: Props = $props();
</script>

<header class="relay-header">
  <div class="relay-header-title">
    <p class="relay-header-eyebrow">{eyebrow}</p>
    <button
      type="button"
      class="relay-header-button"
      onclick={onToggle}
      aria-label={toggleLabel}
      title={toggleLabel}
    >
      <h1>{title}</h1>
      <span class="relay-header-toggle-icon" aria-hidden="true">
        <ArrowLeftRight size={14} />
      </span>
    </button>
    {#if subtitleText || subtitleSnippet}
      <p class="relay-header-subtitle">
        {#if subtitleText}
          {subtitleText}
        {:else}
          {@render subtitleSnippet?.()}
        {/if}
      </p>
    {/if}
  </div>
  {@render children?.()}
</header>

<style>
  .relay-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    position: relative;
  }

  .relay-header-title {
    min-width: 0;
  }

  .relay-header-eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  .relay-header-subtitle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.86rem;
  }

  .relay-header-subtitle :global(strong) {
    color: var(--fg);
    font-weight: 700;
  }

  h1 {
    margin: 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  .relay-header-button,
  .relay-header-button:hover,
  .relay-header-button:focus,
  .relay-header-button:active {
    text-decoration: none;
  }

  .relay-header-button {
    appearance: none;
    background: transparent;
    border: 0;
    padding: 0;
    margin: 0.2rem 0 0;
    font: inherit;
    font-weight: inherit;
    cursor: pointer;
    text-align: left;
    color: inherit;
    display: inline-flex;
    align-items: baseline;
    gap: 0.4rem;
  }

  .relay-header-button:hover {
    color: var(--accent);
  }

  .relay-header-toggle-icon {
    display: inline-flex;
    align-items: center;
    opacity: 0.45;
    transition:
      opacity 0.15s ease,
      transform 0.15s ease;
    color: var(--muted);
  }

  .relay-header-button:hover .relay-header-toggle-icon {
    opacity: 0.9;
    color: var(--accent);
    transform: rotate(180deg);
  }

  @media (max-width: 600px) {
    .relay-header {
      flex-direction: column;
      align-items: flex-start;
    }
  }
</style>
