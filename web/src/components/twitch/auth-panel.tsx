import { Box, Button, Paper, TextField, Typography } from '@mui/material';
import type { ReactElement } from 'react';

interface AuthPanelProps {
  accessCode: string;
  isBusy: boolean;
  loginMode: 'code' | 'qr';
  onSubmitLogin: (event: React.SyntheticEvent<HTMLFormElement>) => void;
  onSwitchToCode: () => void;
  onSwitchToQr: () => void;
  onUpdateAccessCode: (value: string) => void;
  qrDataUrl: string | undefined;
}

export const AuthPanel = ({
  accessCode,
  isBusy,
  loginMode,
  onSubmitLogin,
  onSwitchToCode,
  onSwitchToQr,
  onUpdateAccessCode,
  qrDataUrl,
}: AuthPanelProps): ReactElement => {
  if (loginMode === 'code') {
    return (
      <Box
        component="form"
        onSubmit={onSubmitLogin}
        sx={{ display: 'flex', flexDirection: 'column', gap: 2, maxWidth: 400 }}
      >
        <TextField
          autoComplete="current-password"
          id="access-code"
          label="Access code"
          onChange={(event) => {
            onUpdateAccessCode(event.currentTarget.value);
          }}
          placeholder="Enter shared access code"
          type="password"
          value={accessCode}
          variant="outlined"
        />
        <Button disabled={isBusy} type="submit" variant="contained">
          {isBusy ? 'Signing in...' : 'Sign in'}
        </Button>
        <Button onClick={onSwitchToQr} variant="outlined">
          Sign in with QR code
        </Button>
      </Box>
    );
  }

  return (
    <Box sx={{ alignItems: 'center', display: 'flex', flexDirection: 'column', gap: 2 }}>
      <Paper
        sx={{
          alignItems: 'center',
          display: 'flex',
          justifyContent: 'center',
          minHeight: 200,
          minWidth: 200,
          padding: 2,
        }}
      >
        {qrDataUrl !== undefined && qrDataUrl !== '' ? (
          <img alt="QR Code for login" src={qrDataUrl} style={{ maxWidth: '100%' }} />
        ) : (
          <Typography color="text.secondary">Generating QR code...</Typography>
        )}
      </Paper>
      <Typography align="center">
        Scan with your phone
        <br />
        <Typography component="span" color="text.secondary" variant="caption">
          expires in 5 minutes
        </Typography>
      </Typography>
      <Button onClick={onSwitchToCode} variant="outlined">
        Sign in with access code
      </Button>
    </Box>
  );
};
