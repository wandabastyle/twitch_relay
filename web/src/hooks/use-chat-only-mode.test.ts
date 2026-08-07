import { act, createElement, Fragment, type ReactNode, useState } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { useChatOnlyMode } from './use-chat-only-mode';

const getRequiredElement = (container: HTMLElement | null, selector: string): HTMLElement => {
  if (container === null) {
    throw new Error('Test container is unavailable');
  }

  const element = container.querySelector<HTMLElement>(selector);
  if (element === null) {
    throw new Error(`Missing element: ${selector}`);
  }
  return element;
};

const TestComponent = ({ ticket }: { ticket: string }): ReactNode => {
  const [isChatCollapsed, setIsChatCollapsed] = useState(false);
  const { chatOnly, toggleChatCollapse, toggleChatOnly } = useChatOnlyMode(
    ticket,
    setIsChatCollapsed,
  );

  return createElement(
    Fragment,
    null,
    createElement('output', { 'data-testid': 'chat-only' }, String(chatOnly)),
    createElement('output', { 'data-testid': 'chat-collapsed' }, String(isChatCollapsed)),
    createElement(
      'button',
      { 'data-testid': 'toggle-chat-only', onClick: toggleChatOnly, type: 'button' },
      'Toggle chat only',
    ),
    createElement(
      'button',
      { 'data-testid': 'toggle-chat-collapse', onClick: toggleChatCollapse, type: 'button' },
      'Toggle chat collapse',
    ),
  );
};

describe('useChatOnlyMode', () => {
  let container: HTMLDivElement | null = null;
  let root: Root | null = null;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);
  });

  afterEach(() => {
    root?.unmount();
    container?.remove();
  });

  it('clears chat-only mode before toggling ordinary chat collapse', () => {
    act(() => {
      root?.render(createElement(TestComponent, { ticket: 'first-ticket' }));
    });

    const chatOnlyButton = getRequiredElement(container, '[data-testid="toggle-chat-only"]');
    const chatCollapseButton = getRequiredElement(
      container,
      '[data-testid="toggle-chat-collapse"]',
    );

    act(() => {
      chatOnlyButton.click();
    });
    expect(getRequiredElement(container, '[data-testid="chat-only"]').textContent).toBe('true');
    expect(getRequiredElement(container, '[data-testid="chat-collapsed"]').textContent).toBe(
      'false',
    );

    act(() => {
      chatCollapseButton.click();
    });
    expect(getRequiredElement(container, '[data-testid="chat-only"]').textContent).toBe('false');
    expect(getRequiredElement(container, '[data-testid="chat-collapsed"]').textContent).toBe(
      'true',
    );
  });

  it('resets chat-only mode when the watch ticket changes', () => {
    act(() => {
      root?.render(createElement(TestComponent, { ticket: 'first-ticket' }));
    });

    act(() => {
      getRequiredElement(container, '[data-testid="toggle-chat-only"]').click();
    });

    act(() => {
      root?.render(createElement(TestComponent, { ticket: 'second-ticket' }));
    });
    expect(getRequiredElement(container, '[data-testid="chat-only"]').textContent).toBe('false');
  });
});
