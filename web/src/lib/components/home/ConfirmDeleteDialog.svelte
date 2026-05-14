<script lang="ts">
  import { ConfirmDialog } from '$lib/components/ui';
  import type { RecordingFileEntry } from '$lib/api-client/types';

  interface Props {
    pendingDelete: { bucket: "completed" | "incomplete"; file: RecordingFileEntry } | null;
    isDeleting: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { pendingDelete, isDeleting, onConfirm, onCancel }: Props = $props();

  const isOpen = $derived(pendingDelete !== null);
</script>

<ConfirmDialog
  {isOpen}
  isBusy={isDeleting}
  {onConfirm}
  {onCancel}
  confirmText={isDeleting ? 'Deleting...' : 'Delete'}
  confirmVariant="danger"
>
  <p>
    Delete <strong class="danger-text">{pendingDelete?.file.filename}</strong>?
  </p>
  <p class="subtle">This action cannot be undone.</p>
</ConfirmDialog>
