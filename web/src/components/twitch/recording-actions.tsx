import { Star, Play, Trash2, Wrench } from 'lucide-react';
import type { ReactElement } from 'react';
import { recordingDeleteKey } from '../../api-client/recordings-helpers';
import type { RecordingFileEntry } from '../../api-client/types';

interface RecordingActionsProps {
  readonly file: RecordingFileEntry;
  readonly deletingRecordingKey: string | undefined;
  readonly pinningRecordingKey: string | undefined;
  readonly repairingRecordingKey: string | undefined;
  readonly onToggleRecordingPin: (file: RecordingFileEntry) => void;
  readonly onOpenRecordingPlayer: (file: RecordingFileEntry) => void;
  readonly onRepairRecording: (file: RecordingFileEntry) => void;
  readonly onRequestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ) => void;
}

export const RecordingActions = ({
  file,
  deletingRecordingKey,
  pinningRecordingKey,
  repairingRecordingKey,
  onToggleRecordingPin,
  onOpenRecordingPlayer,
  onRepairRecording,
  onRequestDeleteRecordingFile,
}: RecordingActionsProps): ReactElement => {
  const deleteKey = recordingDeleteKey('completed', file);

  return (
    <div className="recording-item-actions">
      <button
        type="button"
        className={`recording-pin-btn ${file.pinned ? 'pinned' : ''}`}
        onClick={() => {
          onToggleRecordingPin(file);
        }}
        title={file.pinned ? 'Unpin recording' : 'Pin recording'}
        aria-label={file.pinned ? 'Unpin recording' : 'Pin recording'}
        aria-pressed={file.pinned}
        aria-busy={pinningRecordingKey === deleteKey}
        disabled={pinningRecordingKey === deleteKey || file.processing_state === 'processing'}
      >
        <Star size={16} fill={file.pinned ? 'currentColor' : 'none'} />
      </button>
      <button
        type="button"
        className="recording-play-btn"
        onClick={() => {
          onOpenRecordingPlayer(file);
        }}
        title="Play recording"
        aria-label="Play recording"
        disabled={file.processing_state === 'processing' || !file.has_hls}
      >
        <Play size={14} />
      </button>
      {(file.processing_state === 'processing' || !file.has_hls) && (
        <button
          type="button"
          className="recording-play-btn"
          onClick={() => {
            onRepairRecording(file);
          }}
          title="Repair playback assets"
          aria-label="Repair playback assets"
          aria-busy={repairingRecordingKey === deleteKey}
          disabled={repairingRecordingKey === deleteKey}
        >
          {repairingRecordingKey === deleteKey ? (
            <span className="repair-spinner" />
          ) : (
            <Wrench size={14} />
          )}
        </button>
      )}
      <button
        type="button"
        className="recording-delete-btn"
        onClick={() => {
          onRequestDeleteRecordingFile('completed', file);
        }}
        title="Delete recording"
        aria-label="Delete recording"
        aria-busy={deletingRecordingKey === deleteKey}
        disabled={deletingRecordingKey === deleteKey || file.processing_state === 'processing'}
      >
        {deletingRecordingKey === deleteKey ? (
          <span className="delete-spinner" />
        ) : (
          <Trash2 size={14} />
        )}
      </button>
    </div>
  );
};
