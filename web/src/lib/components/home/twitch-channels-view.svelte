<script lang="ts">  import type { ChannelEntry, TwitchChannelsViewProps } from './types';
  import AddChannelForm from './add-channel-form.svelte';
  import ChannelCard from './channel-card.svelte';
  import EmptyState from '../ui/empty-state.svelte';

  // Destructure props to make liveOnly bindable with $bindable()
  let {
    liveOnly = $bindable(false),
    ...props
  }: Omit<TwitchChannelsViewProps, 'liveOnly'> & { liveOnly: boolean } = $props();

  const visibleChannels = (): ChannelEntry[] => {
    if (!liveOnly) {
      return props.channels;
    }
    // If liveOnly is enabled but we haven't loaded live status yet, show all cached channels
    // This prevents false "No channels are live" message
    if (!props.isLiveStatusLoaded) {
      return props.channels;
    }
    return props.channels.filter((channel) => Boolean(props.liveStatus[channel.login]?.live));
  };
</script>

<div class="channels-header">
  <div class="channels-title-row">
    <span class="channels-label">Channels</span>
    <label class="live-only-switch" aria-label="Show only live channels">
      <span class="switch-text">Live only</span>
      <input
        class="switch-input"
        type="checkbox"
        bind:checked={liveOnly}
        onchange={() => props.onLiveOnlyChange(liveOnly)}
      />
      <span class="switch-track" aria-hidden="true">
        <span class="switch-knob"></span>
      </span>
    </label>
  </div>
  <div class="channels-actions">
    <button type="button" class="ui-nav-chip" onclick={props.onOpenRecordings}>
      Recordings overview
    </button>
    {#if !props.showAddForm}
      <button type="button" class="add-btn" onclick={props.onShowAddForm}>
        + Add channel
      </button>
    {/if}
  </div>
</div>

{#if props.liveStatusError}
  <p class="live-status-warning">{props.liveStatusError}</p>
{/if}

{#if props.showAddForm}
  <AddChannelForm
    newChannelLogin={props.newChannelLogin}
    isAdding={props.isAddingChannel}
    onSubmit={props.onSubmitAddChannel}
    onCancel={props.onCancelAddForm}
    onUpdateValue={props.onUpdateNewChannelLogin}
  />
{/if}

<div class="channels">
  {#if visibleChannels().length === 0}
    {#if liveOnly && props.isLiveStatusLoaded}
      <EmptyState
        title="No channels are live"
        description="Toggle off 'Live only' to see all configured channels."
        variant="channels"
      />
    {:else}
      <EmptyState
        title="No channels configured"
        description="Add Twitch channels to see their live status here."
        variant="channels"
      />
    {/if}
  {:else}
    {#each visibleChannels() as channel (channel.login)}
      <ChannelCard
        {channel}
        status={props.liveStatus[channel.login]}
        recordingRule={props.recordingRules[channel.login]}
        activeRecording={props.activeRecordings[channel.login]}
        isWatching={props.watchingChannel === channel.login}
        onOpenSetup={() => props.onOpenChannelSetup(channel.login)}
        onStartWatching={() => props.onStartWatching(channel.login)}
        onToggleAutoRecord={() => props.onToggleAutoRecord(channel.login)}
        onToggleManualRecording={() => props.onToggleManualRecording(channel.login)}
        onRemove={() => props.onPromptRemoveChannel(channel.login)}
      />
    {/each}
  {/if}
</div>

<style>
  .channels-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.6rem;
    margin-bottom: 0.75rem;
  }

  .channels-actions {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .channels-title-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .channels-label {
    font-weight: 600;
    color: var(--fg);
  }

  .live-only-switch {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    color: var(--muted);
    font-size: 0.82rem;
    cursor: pointer;
    user-select: none;
    line-height: 1;
  }

  .switch-text {
    color: var(--muted);
    letter-spacing: 0.01em;
  }

  .switch-input {
    position: absolute;
    opacity: 0;
    width: 1px;
    height: 1px;
    pointer-events: none;
  }

  .switch-track {
    width: 2.6rem;
    height: 1.45rem;
    border-radius: 999px;
    background: rgba(149, 170, 206, 0.3);
    border: 1px solid color-mix(in srgb, var(--border) 75%, transparent);
    display: inline-flex;
    align-items: center;
    padding: 0.11rem;
    transition: background-color 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease;
  }

  .switch-knob {
    width: 1.12rem;
    height: 1.12rem;
    border-radius: 50%;
    background: var(--fg);
    box-shadow: 0 1px 5px rgba(0, 0, 0, 0.28);
    transform: translateX(0);
    transition: transform 0.18s ease;
  }

  .switch-input:checked + .switch-track {
    background: color-mix(in srgb, var(--accent) 80%, var(--accent-2));
    border-color: color-mix(in srgb, var(--accent) 68%, white);
  }

  .switch-input:checked + .switch-track .switch-knob {
    transform: translateX(1.12rem);
  }

  .switch-input:focus-visible + .switch-track {
    box-shadow: 0 0 0 3px var(--focus-ring);
  }

  .switch-input:disabled + .switch-track {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .live-only-switch input {
    margin: 0;
  }

  .live-status-warning {
    margin: 0 0 0.65rem;
    color: var(--warn);
    font-size: 0.8rem;
  }

  .add-btn {
    background: transparent;
    border: 1px dashed color-mix(in srgb, var(--border) 78%, transparent);
    border-radius: 0.6rem;
    color: var(--muted);
    padding: 0.4rem 0.8rem;
    font: inherit;
    font-size: 0.85rem;
    font-weight: 600;
    line-height: 1;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 2rem;
  }

  /* .nav-chip-btn styles now provided by app.css via .ui-nav-chip */

  .add-btn:hover {
    border-color: var(--accent-border);
    background: var(--accent-soft);
    color: var(--fg);
  }

  .channels {
    display: grid;
    gap: 0.75rem;
  }

  .channels {
    display: grid;
    gap: 0.75rem;
  }

  @media (max-width: 600px) {
    .channels-title-row {
      flex-wrap: wrap;
    }

    .channels-header {
      flex-direction: column;
      align-items: flex-start;
    }

    .channels-actions {
      width: 100%;
      justify-content: flex-start;
    }
  }
</style>
