import { act, type ReactElement } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createWatchTicket } from '../../api-client';
import { navigate } from '../../router';
import { PersistentWatchPlayer } from './persistent-watch-player';

vi.mock('../../api-client', () => ({ createWatchTicket: vi.fn() }));
vi.mock('../../router', () => ({ navigate: vi.fn() }));
vi.mock('../../pages/watch-page', () => ({
  WatchPage: ({
    minimized,
    onWatchSessionReadyChange,
    ticketOverride,
  }: {
    minimized: boolean;
    onWatchSessionReadyChange: (ready: boolean) => void;
    ticketOverride: string;
  }): ReactElement => (
    <>
      <div data-minimized={String(minimized)} data-testid="watch-page">
        {ticketOverride}
      </div>
      <button
        type="button"
        onClick={() => {
          onWatchSessionReadyChange(true);
        }}
      >
        Session ready
      </button>
      <button
        type="button"
        onClick={() => {
          onWatchSessionReadyChange(false);
        }}
      >
        Session failed
      </button>
    </>
  ),
}));

const OLD_TICKET = 'old-ticket';
const NEW_TICKET = 'new-ticket';
const SOURCE_BROADCASTER_ID = '100';
const RAID_DESTINATION = 'streamer_b';
const ONE_CALL = 1;
const NO_SOURCES = 0;
const FIRST_INDEX = 0;
const LAST_INDEX = -1;

class FakeEventSource extends EventTarget {
  public static instances: FakeEventSource[] = [];

  public closed = false;
  public readonly url: string;

  public constructor(url: string | URL) {
    super();
    this.url = String(url);
    FakeEventSource.instances.push(this);
  }

  public close(): void {
    this.closed = true;
  }
}

const raidMessage = (eventId = 'raid-event-1', watchTicket = OLD_TICKET): MessageEvent<string> =>
  new MessageEvent('raid', {
    data: JSON.stringify({
      event_id: eventId,
      from_broadcaster_user_id: SOURCE_BROADCASTER_ID,
      from_broadcaster_user_login: 'streamer_a',
      to_broadcaster_user_id: '200',
      to_broadcaster_user_login: RAID_DESTINATION,
      to_broadcaster_user_name: 'StreamerB',
      viewers: 42,
      watch_ticket: watchTicket,
    }),
  });

const flushAsyncWork = async (): Promise<void> => {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
  });
};

describe('PersistentWatchPlayer raid following', () => {
  let container: HTMLDivElement | null = null;
  let root: Root | null = null;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.append(container);
    root = createRoot(container);
    FakeEventSource.instances = [];
    vi.stubGlobal('EventSource', FakeEventSource);
    vi.mocked(createWatchTicket).mockResolvedValue({ watch_url: `/watch/${NEW_TICKET}` });
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    vi.clearAllMocks();
    vi.unstubAllGlobals();
  });

  const renderPlayer = (path: string, routeTicket = ''): void => {
    act(() => {
      root?.render(<PersistentWatchPlayer path={path} routeTicket={routeTicket} />);
    });
  };

  const dispatchRaid = (event = raidMessage()): void => {
    act(() => {
      FakeEventSource.instances.at(LAST_INDEX)?.dispatchEvent(event);
    });
  };

  const markSessionReady = (): void => {
    act(() => {
      container?.querySelector<HTMLButtonElement>('button:nth-of-type(1)')?.click();
    });
  };

  it('creates a normal ticket and replaces the full player on a raid', async () => {
    renderPlayer(`/watch/${OLD_TICKET}`, OLD_TICKET);
    expect(FakeEventSource.instances).toHaveLength(NO_SOURCES);
    markSessionReady();

    dispatchRaid();
    await flushAsyncWork();

    expect(createWatchTicket).toHaveBeenCalledWith(RAID_DESTINATION, OLD_TICKET);
    expect(createWatchTicket).toHaveBeenCalledTimes(ONE_CALL);
    expect(container?.querySelector('[data-testid="watch-page"]')?.textContent).toBe(NEW_TICKET);
    expect(navigate).toHaveBeenCalledWith(`/watch/${NEW_TICKET}`, { replace: true });
    expect(FakeEventSource.instances.at(FIRST_INDEX)?.closed).toBe(true);
  });

  it('replaces a minimized player without leaving the Twitch overview', async () => {
    renderPlayer(`/watch/${OLD_TICKET}`, OLD_TICKET);
    markSessionReady();
    renderPlayer('/twitch');

    dispatchRaid();
    await flushAsyncWork();

    const watchPage = container?.querySelector<HTMLElement>('[data-testid="watch-page"]');
    expect(watchPage?.textContent).toBe(NEW_TICKET);
    expect(watchPage?.dataset.minimized).toBe('true');
    expect(navigate).not.toHaveBeenCalled();
  });

  it('ignores an event belonging to another ticket', async () => {
    renderPlayer(`/watch/${OLD_TICKET}`, OLD_TICKET);
    markSessionReady();

    dispatchRaid(raidMessage('unrelated-event', 'another-ticket'));
    await flushAsyncWork();

    expect(createWatchTicket).not.toHaveBeenCalled();
    expect(container?.querySelector('[data-testid="watch-page"]')?.textContent).toBe(OLD_TICKET);
  });

  it('does not switch twice for a duplicate event', async () => {
    renderPlayer(`/watch/${OLD_TICKET}`, OLD_TICKET);
    markSessionReady();
    const duplicate = raidMessage();

    dispatchRaid(duplicate);
    dispatchRaid(duplicate);
    await flushAsyncWork();

    expect(createWatchTicket).toHaveBeenCalledTimes(ONE_CALL);
  });

  it('keeps the current player and reports a failed ticket creation', async () => {
    vi.mocked(createWatchTicket).mockRejectedValue(new Error('destination is unavailable'));
    renderPlayer(`/watch/${OLD_TICKET}`, OLD_TICKET);
    markSessionReady();

    dispatchRaid();
    await flushAsyncWork();

    expect(container?.querySelector('[data-testid="watch-page"]')?.textContent).toBe(OLD_TICKET);
    expect(container?.querySelector('[role="alert"]')?.textContent).toContain(
      'destination is unavailable',
    );
    expect(navigate).not.toHaveBeenCalled();
  });

  it('closes the event stream and does not resurrect a player after leaving Twitch', () => {
    renderPlayer(`/watch/${OLD_TICKET}`, OLD_TICKET);
    markSessionReady();
    const [firstSource] = FakeEventSource.instances;

    renderPlayer('/youtube');
    renderPlayer('/twitch');

    expect(firstSource.closed).toBe(true);
    expect(container?.querySelector('[data-testid="watch-page"]')).toBeNull();
  });

  it('closing the minimized player tears down raid following', () => {
    renderPlayer(`/watch/${OLD_TICKET}`, OLD_TICKET);
    markSessionReady();
    renderPlayer('/twitch');
    const activeSource = FakeEventSource.instances.at(LAST_INDEX);
    const closeButton = container?.querySelector<HTMLButtonElement>('[aria-label="Close stream"]');

    act(() => {
      closeButton?.click();
    });

    expect(activeSource?.closed).toBe(true);
    expect(container?.querySelector('[data-testid="watch-page"]')).toBeNull();
  });

  it('does not start raid following when watch session initialization fails', () => {
    renderPlayer(`/watch/${OLD_TICKET}`, OLD_TICKET);

    act(() => {
      container?.querySelector<HTMLButtonElement>('button:nth-of-type(2)')?.click();
    });

    expect(FakeEventSource.instances).toHaveLength(NO_SOURCES);
    expect(container?.querySelector('[data-testid="watch-page"]')?.textContent).toBe(OLD_TICKET);
  });
});
