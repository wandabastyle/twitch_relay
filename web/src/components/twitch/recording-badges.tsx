import { Box, Chip } from '@mui/material';
import type { ReactElement } from 'react';
import type { RecordingFileEntry } from '../../api-client/types';

interface RecordingBadgesProps {
  readonly file: RecordingFileEntry;
}

export const RecordingBadges = ({ file }: RecordingBadgesProps): ReactElement => (
  <Box className="recording-badges" sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
    {file.processing_state === 'processing' && (
      <Chip label="Processing" size="small" color="primary" variant="filled" />
    )}
    {file.pinned && <Chip label="Pinned" size="small" color="secondary" variant="filled" />}
    {!file.has_hls && <Chip label="Needs repair" size="small" color="error" variant="outlined" />}
  </Box>
);
