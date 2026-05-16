<script lang="ts">
  import RelayHeader from '$lib/components/shared/RelayHeader.svelte';

  interface Props {
    subtitle: string;
    onToggleMode: () => void;
  }

  let { subtitle, onToggleMode }: Props = $props();
</script>

<RelayHeader
  eyebrow="Private Deck"
  title="Twitch Relay"
  subtitleText={subtitle}
  onToggle={onToggleMode}
  toggleLabel="Switch to YouTube Relay"
/>
