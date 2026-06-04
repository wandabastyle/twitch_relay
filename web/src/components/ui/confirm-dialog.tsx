import Button from '@mui/material/Button';
import Dialog from '@mui/material/Dialog';
import DialogActions from '@mui/material/DialogActions';
import DialogContent from '@mui/material/DialogContent';
import { type ReactElement, type ReactNode, useCallback } from 'react';

interface ConfirmDialogProps {
  cancelText?: string;
  children: ReactNode;
  confirmText: string;
  confirmVariant?: 'primary' | 'danger';
  initialFocus?: 'confirm' | 'cancel';
  isBusy: boolean;
  isOpen: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

export const ConfirmDialog = ({
  cancelText = 'Cancel',
  children,
  confirmText,
  confirmVariant = 'primary',
  initialFocus = 'cancel',
  isBusy,
  isOpen,
  onCancel,
  onConfirm,
}: ConfirmDialogProps): ReactElement | null => {
  const handleClose = useCallback(
    (_event: unknown, reason: string) => {
      if (isBusy) {
        return;
      }
      if (reason === 'backdropClick' || reason === 'escapeKeyDown') {
        onCancel();
      }
    },
    [isBusy, onCancel],
  );

  const handleConfirm = useCallback(() => {
    if (isBusy) {
      return;
    }
    onConfirm();
  }, [isBusy, onConfirm]);

  if (!isOpen) {
    return null;
  }

  return (
    <Dialog
      open={isOpen}
      onClose={handleClose}
      // MUI handles focus restore automatically
      // Prevent closing via backdrop/escape when busy (handled in handleClose)
    >
      <DialogContent>{children}</DialogContent>
      <DialogActions>
        <Button
          variant="outlined"
          onClick={onCancel}
          disabled={isBusy}
          autoFocus={initialFocus === 'cancel'}
        >
          {cancelText}
        </Button>
        <Button
          variant="contained"
          color={confirmVariant === 'danger' ? 'error' : 'primary'}
          onClick={handleConfirm}
          disabled={isBusy}
          autoFocus={initialFocus === 'confirm'}
        >
          {isBusy ? 'Processing...' : confirmText}
        </Button>
      </DialogActions>
    </Dialog>
  );
};
