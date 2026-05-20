import './lib/styles/app.css';
import { mount } from 'svelte';

import App from './app.svelte';

const appElement = globalThis.document.querySelector('#app');

if (!appElement) {
  throw new Error('Failed to find #app element');
}

const app = mount(App, {
  target: appElement,
});

export default app;
