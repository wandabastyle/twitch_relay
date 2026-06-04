import { Box, Button, CircularProgress, Typography } from '@mui/material';
import type { ReactElement } from 'react';

interface IncompleteSectionHeaderProps {
  readonly incompleteCount: number;
  readonly recordingsChannelFilter: string;
  readonly shownIncompleteLength: number;
  readonly selectedCount: number;
  readonly mergingRecordingKey: string | undefined;
  readonly onRequestProcessIncompleteFiles: (channelLogin: string) => void;
}

const EMPTY_LENGTH = 0;
const MINIMUM_SELECTION = 1;
const SINGLE_SELECTION = 1;

const getButtonLabel = (isMerging: boolean, selectedCount: number): string => {
  if (isMerging) {
    return selectedCount === SINGLE_SELECTION ? 'Finalizing...' : 'Merging...';
  }
  return selectedCount === SINGLE_SELECTION
    ? 'Finalize selected'
    : `Merge selected (${selectedCount})`;
};

export const IncompleteSectionHeader = ({
  incompleteCount,
  recordingsChannelFilter,
  shownIncompleteLength,
  selectedCount,
  mergingRecordingKey,
  onRequestProcessIncompleteFiles,
}: IncompleteSectionHeaderProps): ReactElement => {
  const isMerging = mergingRecordingKey === recordingsChannelFilter;
  const buttonLabel = getButtonLabel(isMerging, selectedCount);

  return (
    <Box sx={{ alignItems: 'center', display: 'flex', justifyContent: 'space-between', mb: 1 }}>
      <Typography variant="h6">Incomplete ({incompleteCount})</Typography>
      {recordingsChannelFilter !== 'all' && shownIncompleteLength > EMPTY_LENGTH && (
        <Button
          variant="contained"
          onClick={() => {
            onRequestProcessIncompleteFiles(recordingsChannelFilter);
          }}
          disabled={selectedCount < MINIMUM_SELECTION || isMerging}
          startIcon={isMerging ? <CircularProgress size={14} /> : null}
        >
          {buttonLabel}
        </Button>
      )}
    </Box>
  );
};
