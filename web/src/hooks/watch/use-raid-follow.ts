import { useEffect, useRef } from 'react';
import { createWatchTicket } from '../../api-client';
import { isObject } from '../../api-client/core';

interface RaidEvent {
  eventId: string;
  fromBroadcasterUserId: string;
  toBroadcasterUserLogin: string;
  watchTicket: string;
}

interface RaidFollowOptions {
  enabled: boolean;
  onError: (message: string) => void;
  onFollow: (watchUrl: string) => void;
  ticket: string;
}

const parseRaidEvent = (data: string): RaidEvent | null => {
  try {
    const payload: unknown = JSON.parse(data);
    if (!isObject(payload)) {
      return null;
    }

    const event = payload;
    if (
      typeof event.event_id !== 'string' ||
      typeof event.watch_ticket !== 'string' ||
      typeof event.from_broadcaster_user_id !== 'string' ||
      typeof event.to_broadcaster_user_login !== 'string' ||
      event.event_id === '' ||
      event.watch_ticket === '' ||
      event.from_broadcaster_user_id === '' ||
      event.to_broadcaster_user_login === ''
    ) {
      return null;
    }

    return {
      eventId: event.event_id,
      fromBroadcasterUserId: event.from_broadcaster_user_id,
      toBroadcasterUserLogin: event.to_broadcaster_user_login,
      watchTicket: event.watch_ticket,
    };
  } catch {
    return null;
  }
};

export const useRaidFollow = ({ enabled, onError, onFollow, ticket }: RaidFollowOptions): void => {
  const handledEventIdsRef = useRef(new Set<string>());

  useEffect((): (() => void) | undefined => {
    if (!enabled || ticket === '') {
      return undefined;
    }

    const eventSource = new EventSource(`/api/watch-events/${encodeURIComponent(ticket)}`, {
      withCredentials: true,
    });
    let active = true;
    let switching = false;

    const handleRaid = (event: Event): void => {
      if (!active || switching || !(event instanceof MessageEvent)) {
        return;
      }

      const raid = parseRaidEvent(String(event.data));
      if (
        raid === null ||
        raid.watchTicket !== ticket ||
        handledEventIdsRef.current.has(raid.eventId)
      ) {
        return;
      }

      handledEventIdsRef.current.add(raid.eventId);
      switching = true;
      void createWatchTicket(raid.toBroadcasterUserLogin, ticket)
        .then((response) => {
          if (active) {
            onFollow(response.watch_url);
          }
        })
        .catch((error: unknown) => {
          if (active) {
            const message = error instanceof Error ? error.message : String(error);
            onError(message);
          }
        })
        .finally(() => {
          switching = false;
        });
    };

    eventSource.addEventListener('raid', handleRaid);

    return (): void => {
      active = false;
      eventSource.removeEventListener('raid', handleRaid);
      eventSource.close();
    };
  }, [enabled, onError, onFollow, ticket]);
};
