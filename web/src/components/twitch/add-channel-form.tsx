import { Box, Button, TextField } from '@mui/material';
import type { ReactElement } from 'react';

interface AddChannelFormProps {
  isAdding: boolean;
  newChannelLogin: string;
  onCancel: () => void;
  onSubmit: (event: React.SyntheticEvent<HTMLFormElement>) => void;
  onUpdateValue: (value: string) => void;
}

export const AddChannelForm = ({
  isAdding,
  newChannelLogin,
  onCancel,
  onSubmit,
  onUpdateValue,
}: AddChannelFormProps): ReactElement => (
  <Box component="form" onSubmit={onSubmit} sx={{ display: 'flex', gap: 2 }}>
    <TextField
      autoComplete="off"
      onChange={(event) => {
        onUpdateValue(event.currentTarget.value);
      }}
      placeholder="channel_login"
      size="small"
      spellCheck="false"
      value={newChannelLogin}
      variant="outlined"
    />
    <Button disabled={isAdding} type="submit" variant="contained">
      {isAdding ? 'Adding...' : 'Add'}
    </Button>
    <Button onClick={onCancel} variant="outlined">
      Cancel
    </Button>
  </Box>
);
