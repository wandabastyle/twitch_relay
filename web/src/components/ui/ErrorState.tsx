import { AlertCircle, RefreshCw } from 'lucide-react';
import type { ReactElement } from 'react';

interface ErrorStateProps {
  isRetrying?: boolean;
  message: string;
  onRetry?: () => void;
  retryLabel?: string;
}

export const ErrorState = ({
  isRetrying = false,
  message,
  onRetry,
  retryLabel = 'Try again',
}: ErrorStateProps): ReactElement => {
  return (
    <div className="error-state" role="alert">
      <div className="error-icon">
        <AlertCircle size={20} />
      </div>
      <div className="error-content">
        <p className="error-message">{message}</p>
        {onRetry && (
          <button
            type="button"
            className="retry-btn"
            onClick={onRetry}
            disabled={isRetrying}
            aria-busy={isRetrying}
          >
            {isRetrying ? <span className="retry-spinner" /> : <RefreshCw size={14} />}
            <span>{isRetrying ? 'Retrying...' : retryLabel}</span>
          </button>
        )}
      </div>
    </div>
  );
}
