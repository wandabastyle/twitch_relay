import { Paper, Typography, List, ListItem, ListItemText } from '@mui/material';
import type { ReactElement } from 'react';
import type { ActiveRecording } from '../../api-client/types';

interface ActiveRecordingsSectionProps {
  readonly activeList: ActiveRecording[];
  readonly shownActive: ActiveRecording[];
}

const EMPTY_LENGTH = 0;

export const ActiveRecordingsSection = ({
  activeList,
  shownActive,
}: ActiveRecordingsSectionProps): ReactElement => (
  <Paper className="recordings-section" sx={{ marginBottom: 2, padding: 2 }}>
    <Typography gutterBottom variant="h6">
      Active ({activeList.length})
    </Typography>
    {activeList.length === EMPTY_LENGTH ? (
      <Typography className="ui-muted section-empty" color="text.secondary" variant="body2">
        No active recordings right now.
      </Typography>
    ) : (
      <List className="recordings-list" dense>
        {shownActive.map((recording) => (
          <ListItem disableGutters key={recording.channel_login}>
            <ListItemText
              primary={recording.channel_login}
              secondary={`${recording.mode} · ${recording.quality}`}
            />
          </ListItem>
        ))}
      </List>
    )}
  </Paper>
);
