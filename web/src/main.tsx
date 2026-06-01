import './lib/styles/app.css';

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import App from './app';

const appElement = document.querySelector('#app');

if (!appElement) {
  throw new Error('Failed to find #app element');
}

const root = createRoot(appElement);

root.render(
  <StrictMode>
    <App />
  </StrictMode>,
);
