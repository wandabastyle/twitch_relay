<script lang="ts">
  import type { Snippet } from 'svelte';
  import FolderOpen from 'lucide-svelte/icons/folder-open';
  import Users from 'lucide-svelte/icons/users';
  import Video from 'lucide-svelte/icons/video';
  import Tv from 'lucide-svelte/icons/tv';
  import ListVideo from 'lucide-svelte/icons/list-video';

  interface Props {
    title: string;
    description?: string;
    action?: Snippet;
    variant?: 'default' | 'channels' | 'videos' | 'playlists' | 'recordings';
  }

  let {
    title,
    description,
    action,
    variant = 'default'
  }: Props = $props();

  const icons = {
    default: FolderOpen,
    channels: Users,
    videos: Video,
    playlists: ListVideo,
    recordings: Tv
  };

  const Icon = $derived(icons[variant]);
</script>

<div class="empty-state" role="status">
  <div class="empty-icon">
    <Icon size={32} strokeWidth={1.5} />
  </div>
  <h3 class="empty-title">{title}</h3>
  {#if description}
    <p class="empty-description">{description}</p>
  {/if}
  {#if action}
    <div class="empty-action">
      {@render action()}
    </div>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 2.5rem 1.5rem;
    gap: 0.75rem;
  }

  .empty-icon {
    color: var(--muted);
    opacity: 0.6;
    margin-bottom: 0.25rem;
  }

  .empty-title {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--fg);
  }

  .empty-description {
    margin: 0;
    font-size: 0.9rem;
    color: var(--muted);
    line-height: 1.5;
    max-width: 24rem;
  }

  .empty-action {
    margin-top: 0.5rem;
  }

  @media (max-width: 600px) {
    .empty-state {
      padding: 2rem 1rem;
    }
  }
</style>
