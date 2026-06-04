import { Box, IconButton, Tooltip, CircularProgress } from '@mui/material';
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
    <Box className="recording-item-actions" sx={{ alignItems: 'center', display: 'flex', gap: 1 }}>
      <Tooltip title={file.pinned ? 'Unpin recording' : 'Pin recording'}>
        <IconButton
          type="button"
          aria-label={file.pinned ? 'Unpin recording' : 'Pin recording'}
          aria-pressed={file.pinned}
          aria-busy={pinningRecordingKey === deleteKey}
          disabled={pinningRecordingKey === deleteKey || file.processing_state === 'processing'}
          onClick={() => {
            onToggleRecordingPin(file);
          }}
        >
          <Star size={16} fill={file.pinned ? 'currentColor' : 'none'} />
        </IconButton>
      </Tooltip>
      <Tooltip title="Play recording">
        <span>
          {/* Span wrapper to allow disabled Tooltip behavior */}
          <IconButton
            type="button"
            aria-label="Play recording"
            disabled={file.processing_state === 'processing' || !file.has_hls}
            onClick={() => {
              onOpenRecordingPlayer(file);
            }}
          >
            <Play size={14} />
          </IconButton>
        </span>
      </Tooltip>
      {(file.processing_state === 'processing' || !file.has_hls) && (
        <Tooltip title="Repair playback assets">
          <span>
            <IconButton
              type="button"
              aria-label="Repair playback assets"
              aria-busy={repairingRecordingKey === deleteKey}
              disabled={repairingRecordingKey === deleteKey}
              onClick={() => {
                onRepairRecording(file);
              }}
            >
              {repairingRecordingKey === deleteKey ? (
                <CircularProgress size={14} thickness={5} />
              ) : (
                <Wrench size={14} />
              )}
            </IconButton>
          </span>
        </Tooltip>
      )}
      <Tooltip title="Delete recording">
        <span>
          <IconButton
            type="button"
            aria-label="Delete recording"
            aria-busy={deletingRecordingKey === deleteKey}
            disabled={deletingRecordingKey === deleteKey || file.processing_state === 'processing'}
            onClick={() => {
              onRequestDeleteRecordingFile('completed', file);
            }}
          >
            {deletingRecordingKey === deleteKey ? (
              <CircularProgress size={14} thickness={5} />
            ) : (
              <Trash2 size={14} />
            )}
          </IconButton>
        </span>
      </Tooltip>
    </Box>
  );
};
