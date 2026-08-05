import { Loader2, Radio, Square } from 'lucide-react';
import { useCallback, useState, type ReactElement } from 'react';
import type { ActiveRecording } from '../../api-client/types';

interface RecordingButtonProps {
  channelLogin: string;
  recording: ActiveRecording | undefined;
  onStart: () => Promise<void>;
  onStop: () => Promise<void>;
}

const EMPTY_MESSAGE_LENGTH = 0;

const readMessage = (error: unknown, fallback: string): string => {
  if (error instanceof Error && error.message.trim().length > EMPTY_MESSAGE_LENGTH) {
    return error.message;
  }
  return fallback;
};

export const RecordingButton = ({
  channelLogin,
  recording,
  onStart,
  onStop,
}: RecordingButtonProps): ReactElement => {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | undefined>();

  const isRecording = recording !== undefined;
  const mode = recording?.mode;
  const recordingClass = ((): string => {
    if (mode === 'auto') {
      return 'active-auto';
    }
    if (mode === 'manual') {
      return 'active-manual';
    }
    return '';
  })();

  const handleClick = useCallback((): void => {
    setLoading(true);
    setError(undefined);

    const action = isRecording ? onStop : onStart;

    void (async (): Promise<void> => {
      try {
        await action();
      } catch (actionError) {
        setError(readMessage(actionError, `Recording action failed for ${channelLogin}`));
      } finally {
        setLoading(false);
      }
    })();
  }, [isRecording, onStart, onStop, channelLogin]);

  const label = isRecording ? 'Stop' : 'Record';
  const icon = isRecording ? <Square size={14} fill="currentColor" /> : <Radio size={14} />;

  return (
    <div className="recording-button-wrap">
      <button
        type="button"
        className={`recording-button ${isRecording ? 'recording' : ''} ${recordingClass} ${loading ? 'loading' : ''}`}
        onClick={handleClick}
        disabled={loading}
        aria-busy={loading}
        title={isRecording ? `Stop ${mode ?? ''} recording`.trim() : 'Start recording'}
        aria-label={isRecording ? `Stop ${mode ?? ''} recording`.trim() : 'Start recording'}
      >
        {loading ? <Loader2 size={14} className="spinning" /> : icon}
        {label}
      </button>
      {isRecording && <span className="recording-pulse" aria-hidden="true" />}
      {error !== undefined && error !== '' && <span className="recording-error">{error}</span>}
    </div>
  );
};
