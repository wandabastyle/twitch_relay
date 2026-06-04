import { Paper, Typography, List } from '@mui/material';
import type { ReactElement } from 'react';
import type { RecordingFileEntry } from '../../api-client/types';
import { CompletedRecordingRow } from './completed-recording-row';

interface CompletedRecordingsSectionProps {
  readonly completedList: readonly RecordingFileEntry[];
  readonly shownCompleted: readonly RecordingFileEntry[];
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

const EMPTY_LENGTH = 0;

export const CompletedRecordingsSection = ({
  completedList,
  shownCompleted,
  deletingRecordingKey,
  pinningRecordingKey,
  repairingRecordingKey,
  onToggleRecordingPin,
  onOpenRecordingPlayer,
  onRepairRecording,
  onRequestDeleteRecordingFile,
}: CompletedRecordingsSectionProps): ReactElement => (
  <Paper className="recordings-section" sx={{ marginBottom: 2, padding: 2 }}>
    <Typography gutterBottom variant="h6">
      Completed ({completedList.length})
    </Typography>
    {completedList.length === EMPTY_LENGTH ? (
      <Typography className="ui-muted section-empty" color="text.secondary" variant="body2">
        No completed files yet.
      </Typography>
    ) : (
      <List className="recordings-list" dense>
        {shownCompleted.map((file) => (
          <CompletedRecordingRow
            key={file.path_display}
            file={file}
            deletingRecordingKey={deletingRecordingKey}
            pinningRecordingKey={pinningRecordingKey}
            repairingRecordingKey={repairingRecordingKey}
            onToggleRecordingPin={onToggleRecordingPin}
            onOpenRecordingPlayer={onOpenRecordingPlayer}
            onRepairRecording={onRepairRecording}
            onRequestDeleteRecordingFile={onRequestDeleteRecordingFile}
          />
        ))}
      </List>
    )}
  </Paper>
);
