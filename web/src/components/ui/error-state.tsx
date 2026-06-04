import { Alert, Button, CircularProgress } from '@mui/material';
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
}: ErrorStateProps): ReactElement => (
  <Alert severity="error" role="alert" icon={<AlertCircle size={20} />}>
    {message}
    {onRetry && (
      <Button
        variant="contained"
        size="small"
        onClick={onRetry}
        disabled={isRetrying}
        aria-busy={isRetrying}
        startIcon={
          isRetrying ? <CircularProgress size={14} thickness={5} /> : <RefreshCw size={14} />
        }
        sx={{ mt: 1 }}
      >
        {isRetrying ? 'Retrying...' : retryLabel}
      </Button>
    )}
  </Alert>
);
