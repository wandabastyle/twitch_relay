<script lang="ts">
  import { ConfirmDialog } from '$lib/components/ui';

  interface Props {
    pendingMerge: { channelLogin: string; action: "finalize" | "merge"; filenames: string[] } | null;
    isProcessing: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { pendingMerge, isProcessing, onConfirm, onCancel }: Props = $props();

  const isOpen = $derived(pendingMerge !== null);
</script>

<ConfirmDialog
  {isOpen}
  isBusy={isProcessing}
  {onConfirm}
  {onCancel}
  confirmText={isProcessing ? 'Processing...' : (pendingMerge?.action === "finalize" ? "Finalize" : "Merge")}
>
  <p>
    {pendingMerge?.action === "finalize" ? "Finalize" : "Merge"}
    <strong>{pendingMerge?.filenames.length}</strong>
    incomplete recording(s) for
    <strong>{pendingMerge?.channelLogin}</strong>?
  </p>
  <p class="subtle">This action cannot be undone.</p>
</ConfirmDialog>
