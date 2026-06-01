import { useEffect, type ReactElement } from 'react';
import { navigate } from '../router';

export const IndexRedirect = (): ReactElement => {
  useEffect(() => {
    // Redirect to Twitch as default
    navigate('/twitch');
  }, []);

  // This page redirects to /twitch - no UI needed
  return <></>;
};
