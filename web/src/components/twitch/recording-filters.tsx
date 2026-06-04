import { Box, FormControl, InputLabel, Select, MenuItem, Typography } from '@mui/material';
import type { ReactElement } from 'react';

interface RecordingFiltersProps {
  readonly channelOptions: string[];
  readonly recordingsChannelFilter: string;
  readonly onUpdateFilter: (value: string) => void;
}

export const RecordingFilters = ({
  channelOptions,
  recordingsChannelFilter,
  onUpdateFilter,
}: RecordingFiltersProps): ReactElement => (
  <Box sx={{ alignItems: 'center', display: 'flex', marginBottom: 2 }}>
    <FormControl variant="outlined" size="small" sx={{ minWidth: 200, mr: 2 }}>
      <InputLabel id="recordings-filter-label">Filter by channel</InputLabel>
      <Select
        labelId="recordings-filter-label"
        id="recordings-filter"
        value={recordingsChannelFilter}
        label="Filter by channel"
        onChange={(event) => {
          onUpdateFilter(event.target.value);
        }}
      >
        <MenuItem value="all">All channels</MenuItem>
        {channelOptions.map((channelLogin) => (
          <MenuItem key={channelLogin} value={channelLogin}>
            {channelLogin}
          </MenuItem>
        ))}
      </Select>
    </FormControl>
    <Typography variant="caption" color="text.secondary">
      All channels shows latest 3 per section.
    </Typography>
  </Box>
);
