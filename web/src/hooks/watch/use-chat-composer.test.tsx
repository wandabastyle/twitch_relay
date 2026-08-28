import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useChatComposer, type UseChatComposerReturn } from './use-chat-composer';

describe('useChatComposer', () => {
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

  it('retains composer content when sending fails', async () => {
    const onSubmit = vi
      .fn<(text: string) => Promise<void>>()
      .mockRejectedValue(new Error('offline'));
    const composerRef: { current: UseChatComposerReturn | undefined } = { current: undefined };

    const TestComponent = (): null => {
      composerRef.current = useChatComposer({ availableEmotes: [], onSubmit });
      return null;
    };

    act(() => {
      root?.render(<TestComponent />);
    });
    const initialComposer = composerRef.current;
    if (initialComposer === undefined) {
      throw new Error('Composer did not render');
    }
    act(() => {
      initialComposer.setComposerText('please retry');
    });
    const composer = composerRef.current;
    if (composer === undefined) {
      throw new Error('Composer did not update');
    }
    await act(async () => {
      await composer.submit();
    });

    expect(onSubmit).toHaveBeenCalledWith('please retry');
    expect(composerRef.current?.text).toBe('please retry');
  });
});
