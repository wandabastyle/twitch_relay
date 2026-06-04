import { ListItem, ListItemText, Box } from '@mui/material';
import type { ReactElement } from 'react';
import type { RecordingFileEntry } from '../../api-client/types';
import { RecordingActions } from './recording-actions';
import { RecordingBadges } from './recording-badges';

interface CompletedRecordingRowProps {
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

export const CompletedRecordingRow = ({
  file,
  deletingRecordingKey,
  pinningRecordingKey,
  repairingRecordingKey,
  onToggleRecordingPin,
  onOpenRecordingPlayer,
  onRepairRecording,
  onRequestDeleteRecordingFile,
}: CompletedRecordingRowProps): ReactElement => (
  <ListItem
    secondaryAction={
      <RecordingActions
        file={file}
        deletingRecordingKey={deletingRecordingKey}
        pinningRecordingKey={pinningRecordingKey}
        repairingRecordingKey={repairingRecordingKey}
        onToggleRecordingPin={onToggleRecordingPin}
        onOpenRecordingPlayer={onOpenRecordingPlayer}
        onRepairRecording={onRepairRecording}
        onRequestDeleteRecordingFile={onRequestDeleteRecordingFile}
      />
    }
    sx={{ alignItems: 'flex-start' }}
  >
    <ListItemText
      primary={
        <Box sx={{ alignItems: 'center', display: 'flex', justifyContent: 'space-between' }}>
          <span title={file.filename}>{file.filename}</span>
          <RecordingBadges file={file} />
        </Box>
      }
      secondary={<span title={file.path_display}>{file.path_display}</span>}
    />
  </ListItem>
);
