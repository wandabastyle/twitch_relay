<script lang="ts">
  import ChevronDown from 'lucide-svelte/icons/chevron-down';

  interface Props {
    channelOptions: string[];
    recordingsChannelFilter: string;
    onUpdateFilter: (value: string) => void;
  }

  let { channelOptions, recordingsChannelFilter, onUpdateFilter }: Props = $props();
</script>

<div class="recordings-filter-row">
  <label class="recordings-filter-label" for="recordings-filter">Filter by channel</label>
  <div class="select-wrapper">
    <select
      id="recordings-filter"
      class="recordings-filter-select"
      value={recordingsChannelFilter}
      onchange={(e) => onUpdateFilter(e.currentTarget.value)}
    >
      <option value="all">All channels</option>
      {#each channelOptions as channelLogin (channelLogin)}
        <option value={channelLogin}>{channelLogin}</option>
      {/each}
    </select>
    <span class="select-chevron" aria-hidden="true">
      <ChevronDown size={14} />
    </span>
  </div>
  <p class="recordings-filter-hint">All channels shows latest 3 per section.</p>
</div>

<style>
  .recordings-filter-row {
    display: grid;
    gap: 0.35rem;
    margin-top: -0.1rem;
  }

  .recordings-filter-label {
    color: var(--muted);
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .select-wrapper {
    position: relative;
    width: min(22rem, 100%);
  }

  .select-chevron {
    position: absolute;
    right: 0.75rem;
    top: 50%;
    transform: translateY(-50%);
    color: var(--muted);
    pointer-events: none;
  }

  .recordings-filter-select {
    width: 100%;
    border: 1px solid color-mix(in srgb, var(--border) 65%, transparent);
    background: color-mix(in srgb, var(--bg-soft) 70%, var(--surface));
    color: var(--fg);
    border-radius: 0.6rem;
    padding: 0.6rem 2.5rem 0.6rem 0.7rem;
    font: inherit;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    transition: border-color 0.15s ease, background-color 0.15s ease;
  }

  .recordings-filter-select:hover {
    border-color: var(--accent-border);
  }

  .recordings-filter-select:focus {
    outline: none;
    border-color: var(--accent-border);
    box-shadow: 0 0 0 2px var(--focus-ring);
  }

  .recordings-filter-select:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .recordings-filter-hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.78rem;
  }
</style>
