<script lang="ts">
  import { ConfirmDialog } from '$lib/components/ui';

  interface Props {
    channelLogin: string | null;
    isRemoving: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { channelLogin, isRemoving, onConfirm, onCancel }: Props = $props();

  const isOpen = $derived(channelLogin !== null);
</script>

<ConfirmDialog
  {isOpen}
  isBusy={isRemoving}
  {onConfirm}
  {onCancel}
  confirmText={isRemoving ? 'Removing...' : 'Remove'}
  confirmVariant="danger"
>
  <p>
    Remove <strong class="danger-text">{channelLogin}</strong> from the channel list?
  </p>
</ConfirmDialog>
