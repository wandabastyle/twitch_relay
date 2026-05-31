import { useEffect, useRef, useState, useCallback, type ReactElement, type ReactNode } from 'react';

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

const FIRST_INDEX = 0;
const FOCUS_TIMEOUT_MS = 0;
const LAST_INDEX_OFFSET = 1;
const TRANSITION_DURATION_MS = 180;
const ZERO_LENGTH = 0;

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
  const [isExiting, setIsExiting] = useState(false);
  const modalElement = useRef<HTMLDivElement>(null);
  const confirmButton = useRef<HTMLButtonElement>(null);
  const cancelButton = useRef<HTMLButtonElement>(null);
  const lastFocusedElement = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (isOpen) {
      setIsExiting(false);
      if (document.activeElement instanceof HTMLElement) {
        lastFocusedElement.current = document.activeElement;
      }
    }
  }, [isOpen]);

  useEffect(() => {
    if (isOpen && !isExiting) {
      const timer = setTimeout(() => {
        if (initialFocus === 'confirm' && confirmButton.current) {
          confirmButton.current.focus({ preventScroll: true });
        } else if (cancelButton.current) {
          cancelButton.current.focus({ preventScroll: true });
        }
      }, FOCUS_TIMEOUT_MS);
      return (): void => {
        clearTimeout(timer);
      };
    }
  }, [isOpen, isExiting, initialFocus]);

  const restoreFocus = useCallback(() => {
    if (lastFocusedElement.current && typeof lastFocusedElement.current.focus === 'function') {
      lastFocusedElement.current.focus({ preventScroll: true });
    }
  }, []);

  const handleCancel = useCallback(() => {
    if (isBusy) {
      return;
    }
    setIsExiting(true);
    setTimeout(() => {
      onCancel();
      restoreFocus();
    }, TRANSITION_DURATION_MS);
  }, [isBusy, onCancel, restoreFocus]);

  const handleConfirm = useCallback(() => {
    if (isBusy) {
      return;
    }
    onConfirm();
  }, [isBusy, onConfirm]);

  const handleOverlayClick = useCallback(() => {
    if (!isBusy) {
      handleCancel();
    }
  }, [isBusy, handleCancel]);

  const handleKeydown = useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key === 'Escape' && !isBusy) {
        event.preventDefault();
        handleCancel();
        return;
      }

      if (event.key === 'Tab' && modalElement.current) {
        const focusableElements = modalElement.current.querySelectorAll(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
        );
        if (focusableElements.length === ZERO_LENGTH) {
          return;
        }

        const firstElement = focusableElements[FIRST_INDEX];
        const lastElement = focusableElements[focusableElements.length - LAST_INDEX_OFFSET];

        if (!(firstElement instanceof HTMLElement) || !(lastElement instanceof HTMLElement)) {
          return;
        }

        if (event.shiftKey && document.activeElement === firstElement) {
          event.preventDefault();
          lastElement.focus({ preventScroll: true });
        } else if (!event.shiftKey && document.activeElement === lastElement) {
          event.preventDefault();
          firstElement.focus({ preventScroll: true });
        }
      }
    },
    [isBusy, handleCancel],
  );

  if (!isOpen) {
    return null;
  }

  return (
    <div
      className={`modal-overlay ${isExiting ? 'exiting' : 'entering'}`}
      onClick={handleOverlayClick}
      role="presentation"
    >
      <div
        ref={modalElement}
        className={`modal ${isExiting ? 'exiting' : 'entering'}`}
        onClick={(clickEvent) => {
          clickEvent.stopPropagation();
        }}
        onKeyDown={handleKeydown}
        role="dialog"
        aria-modal="true"
      >
        <div className="modal-content">{children}</div>
        <div className="modal-actions">
          <button
            ref={cancelButton}
            type="button"
            className="ui-ghost-btn"
            onClick={handleCancel}
            disabled={isBusy}
          >
            {cancelText}
          </button>
          <button
            ref={confirmButton}
            type="button"
            className={confirmVariant}
            onClick={handleConfirm}
            disabled={isBusy}
          >
            {isBusy ? 'Processing...' : confirmText}
          </button>
        </div>
      </div>
    </div>
  );
}
