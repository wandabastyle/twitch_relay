<script lang="ts">
  import RelayHeader from '$lib/components/shared/relay-header.svelte';

  interface Props {
    onToggleMode: () => void;
    subtitle: string;
  }

  const { subtitle, onToggleMode }: Props = $props();
</script>

<RelayHeader
  eyebrow="Private Deck"
  title="Twitch Relay"
  subtitleText={subtitle}
  onToggle={onToggleMode}
  toggleLabel="Switch to YouTube Relay"
/>
