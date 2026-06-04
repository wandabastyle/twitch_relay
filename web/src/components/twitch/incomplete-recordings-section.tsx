import { Paper, List, Typography } from '@mui/material';
import type { ReactElement } from 'react';
import type { RecordingFileEntry } from '../../api-client/types';
import { IncompleteRecordingRow } from './incomplete-recording-row';
import { IncompleteSectionHeader } from './incomplete-section-header';

interface IncompleteRecordingsSectionProps {
  readonly incompleteList: readonly RecordingFileEntry[];
  readonly shownIncomplete: readonly RecordingFileEntry[];
  readonly recordingsChannelFilter: string;
  readonly selectedCount: number;
  readonly mergingRecordingKey: string | undefined;
  readonly deletingRecordingKey: string | undefined;
  readonly selectedIncompleteFilenames: Set<string>;
  readonly onToggleIncompleteMergeSelection: (filename: string) => void;
  readonly onRequestProcessIncompleteFiles: (channelLogin: string) => void;
  readonly onRequestDeleteRecordingFile: (
    bucket: 'completed' | 'incomplete',
    file: RecordingFileEntry,
  ) => void;
}

const EMPTY_LENGTH = 0;

export const IncompleteRecordingsSection = ({
  incompleteList,
  shownIncomplete,
  recordingsChannelFilter,
  selectedCount,
  mergingRecordingKey,
  deletingRecordingKey,
  selectedIncompleteFilenames,
  onToggleIncompleteMergeSelection,
  onRequestProcessIncompleteFiles,
  onRequestDeleteRecordingFile,
}: IncompleteRecordingsSectionProps): ReactElement => (
  <Paper className="recordings-section" sx={{ marginBottom: 2, padding: 2 }}>
    <IncompleteSectionHeader
      incompleteCount={incompleteList.length}
      mergingRecordingKey={mergingRecordingKey}
      onRequestProcessIncompleteFiles={onRequestProcessIncompleteFiles}
      recordingsChannelFilter={recordingsChannelFilter}
      selectedCount={selectedCount}
      shownIncompleteLength={shownIncomplete.length}
    />
    {incompleteList.length === EMPTY_LENGTH ? (
      <Typography className="ui-muted section-empty" color="text.secondary" variant="body2">
        No incomplete files.
      </Typography>
    ) : (
      <List className="recordings-list" dense>
        {shownIncomplete.map((file) => (
          <IncompleteRecordingRow
            deletingRecordingKey={deletingRecordingKey}
            file={file}
            key={file.path_display}
            mergingRecordingKey={mergingRecordingKey}
            recordingsChannelFilter={recordingsChannelFilter}
            selectedIncompleteFilenames={selectedIncompleteFilenames}
            onRequestDeleteRecordingFile={onRequestDeleteRecordingFile}
            onToggleIncompleteMergeSelection={onToggleIncompleteMergeSelection}
          />
        ))}
      </List>
    )}
  </Paper>
);
