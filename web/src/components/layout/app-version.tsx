import Typography from '@mui/material/Typography';
import { useEffect, useState, type ReactElement } from 'react';
import { getVersion } from '../../api-client/version';

const UNKNOWN_VERSION = '…';
const ERROR_VERSION = '?';

export default function AppVersion(): ReactElement {
  const [version, setVersion] = useState(UNKNOWN_VERSION);

  useEffect(() => {
    const fetchVersion = async (): Promise<void> => {
      try {
        const { version: ver } = await getVersion();
        setVersion(ver);
      } catch {
        setVersion(ERROR_VERSION);
      }
    };

    fetchVersion().catch(() => {
      setVersion(ERROR_VERSION);
    });
  }, []);

  return (
    <Typography
      variant="caption"
      component="p"
      className="app-version"
      aria-label="App version"
      sx={{ mt: 1, opacity: 0.6 }}
    >
      via Twitch Relay · v{version}
    </Typography>
  );
}
