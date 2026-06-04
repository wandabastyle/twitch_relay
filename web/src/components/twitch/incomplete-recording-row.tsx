import { ListItem, ListItemText, IconButton, CircularProgress, Box } from '@mui/material';
import { CheckSquare, Square, Trash2 } from 'lucide-react';
import type { ReactElement } from 'react';
import { recordingDeleteKey } from '../../api-client/recordings-helpers';
import type { RecordingFileEntry } from '../../api-client/types';

interface IncompleteRecordingRowProps {
  readonly file: RecordingFileEntry;
  readonly deletingRecordingKey: string | undefined;
  readonly mergingRecordingKey: string | undefined;
  readonly selectedIncompleteFilenames: ReadonlySet<string>;
  readonly recordingsChannelFilter: string;
  readonly onToggleIncompleteMergeSelection: (filename: string) => void;
  readonly onRequestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ) => void;
}

export const IncompleteRecordingRow = ({
  file,
  deletingRecordingKey,
  mergingRecordingKey,
  selectedIncompleteFilenames,
  recordingsChannelFilter,
  onToggleIncompleteMergeSelection,
  onRequestDeleteRecordingFile,
}: IncompleteRecordingRowProps): ReactElement => {
  const deleteKey = recordingDeleteKey('incomplete', file);
  const isSelected = selectedIncompleteFilenames.has(file.filename);

  return (
    <ListItem
      secondaryAction={
        <Box sx={{ alignItems: 'center', display: 'flex', gap: 1 }}>
          {recordingsChannelFilter !== 'all' && (
            <IconButton
              aria-label={isSelected ? 'Deselect file' : 'Select file'}
              aria-pressed={isSelected}
              disabled={mergingRecordingKey === recordingsChannelFilter}
              edge="start"
              onClick={() => {
                onToggleIncompleteMergeSelection(file.filename);
              }}
              title={isSelected ? 'Deselect' : 'Select'}
            >
              {isSelected ? <CheckSquare size={14} /> : <Square size={14} />}
            </IconButton>
          )}
          <IconButton
            aria-busy={deletingRecordingKey === deleteKey}
            aria-label="Delete recording"
            disabled={deletingRecordingKey === deleteKey}
            edge="end"
            onClick={() => {
              onRequestDeleteRecordingFile('incomplete', file);
            }}
            title="Delete recording"
          >
            {deletingRecordingKey === deleteKey ? (
              <CircularProgress size={14} />
            ) : (
              <Trash2 size={14} />
            )}
          </IconButton>
        </Box>
      }
      sx={{ alignItems: 'flex-start' }}
    >
      <ListItemText
        primary={
          <Box sx={{ alignItems: 'center', display: 'flex', justifyContent: 'space-between' }}>
            <span title={file.filename}>{file.filename}</span>
          </Box>
        }
        secondary={<span title={file.path_display}>{file.path_display}</span>}
      />
    </ListItem>
  );
};
