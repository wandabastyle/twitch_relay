<script lang="ts">
  import ArrowLeftRight from 'lucide-svelte/icons/arrow-left-right';

  interface Props {
    subtitle: string;
    onToggleMode: () => void;
  }

  let { subtitle, onToggleMode }: Props = $props();
</script>

<header class="panel-header">
  <div class="panel-title">
    <p class="eyebrow">Private Deck</p>
    <button
      type="button"
      class="relay-title-button"
      onclick={onToggleMode}
      aria-label="Switch to YouTube Relay"
      title="Switch to YouTube Relay"
    >
      <h1>Twitch Relay</h1>
      <span class="toggle-icon" aria-hidden="true">
        <ArrowLeftRight size={14} />
      </span>
    </button>
    <p class="header-subtle">{subtitle}</p>
  </div>
</header>

<style>
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
    position: relative;
  }

  .panel-title {
    min-width: 0;
  }

  .eyebrow {
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    font-size: 0.68rem;
    color: var(--muted);
  }

  .header-subtle {
    margin: 0.35rem 0 0;
    color: var(--muted);
    font-size: 0.86rem;
  }

  h1 {
    margin: 0;
    font-size: clamp(1.5rem, 4vw, 2rem);
    line-height: 1.1;
  }

  .relay-title-button,
  .relay-title-button:hover,
  .relay-title-button:focus,
  .relay-title-button:active {
    text-decoration: none;
  }

  .relay-title-button {
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

  .toggle-icon {
    display: inline-flex;
    align-items: center;
    opacity: 0.45;
    transition:
      opacity 0.15s ease,
      transform 0.15s ease;
    color: var(--muted);
  }

  .relay-title-button:hover .toggle-icon {
    opacity: 0.9;
    color: var(--accent);
    transform: rotate(180deg);
  }

  .relay-title-button:hover {
    color: var(--accent);
  }
</style>
