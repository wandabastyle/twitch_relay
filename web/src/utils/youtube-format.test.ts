import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { formatDuration, formatTimeAgo, formatViewCount } from './youtube-format';

const FIXED_TIMESTAMP = 1_000_000_000;
const THIRTY_SECONDS = 30;
const FIFTY_NINE_SECONDS = 59;
const SIXTY_SECONDS = 60;
const ONE_THOUSAND_EIGHT_HUNDRED_SECONDS = 1800;
const THREE_THOUSAND_FIVE_HUNDRED_FORTY_SECONDS = 3540;
const THREE_THOUSAND_SIX_HUNDRED_SECONDS = 3600;
const FORTY_THREE_THOUSAND_TWO_HUNDRED_SECONDS = 43_200;
const EIGHTY_SIX_THOUSAND_THREE_HUNDRED_FORTY_SECONDS = 86_340;
const EIGHTY_SIX_THOUSAND_FOUR_HUNDRED_SECONDS = 86_400;
const FOUR_HUNDRED_THIRTY_TWO_THOUSAND_SECONDS = 432_000;
const SIX_HUNDRED_FOUR_THOUSAND_SEVEN_HUNDRED_FORTY_SECONDS = 604_740;
const SIX_HUNDRED_FOUR_THOUSAND_EIGHT_HUNDRED_SECONDS = 604_800;
const TWO_MILLION_FOUR_HUNDRED_NINETEEN_THOUSAND_TWO_HUNDRED_SECONDS = 2_419_200;
const TWO_MILLION_FIVE_HUNDRED_NINETY_TWO_THOUSAND_ONE_SECONDS = 2_592_001;
const FIFTEEN_MILLION_FIVE_HUNDRED_FIFTY_TWO_THOUSAND_SECONDS = 15_552_000;
const TWENTY_EIGHT_MILLION_NINE_HUNDRED_FORTY_FOUR_THOUSAND_SECONDS = 28_944_000;
const THIRTY_ONE_MILLION_FIVE_HUNDRED_THIRTY_SIX_THOUSAND_ONE_SECONDS = 31_536_001;
const SIXTY_THREE_MILLION_SEVENTY_TWO_THOUSAND_SECONDS = 63_072_000;

const MILLISECONDS_MULTIPLIER = 1000;

const testTimeAgoFalsy = (_mockNow: number): void => {
  const EMPTY = 0;
  expect(formatTimeAgo(EMPTY)).toBe('');
  expect(formatTimeAgo(Number.NaN)).toBe('');
};

const testTimeAgoJustNow = (mockNow: number): void => {
  expect(formatTimeAgo(mockNow - THIRTY_SECONDS)).toBe('Just now');
  expect(formatTimeAgo(mockNow - FIFTY_NINE_SECONDS)).toBe('Just now');
};

const testTimeAgoMinutes = (mockNow: number): void => {
  expect(formatTimeAgo(mockNow - SIXTY_SECONDS)).toBe('1m ago');
  expect(formatTimeAgo(mockNow - ONE_THOUSAND_EIGHT_HUNDRED_SECONDS)).toBe('30m ago');
  expect(formatTimeAgo(mockNow - THREE_THOUSAND_FIVE_HUNDRED_FORTY_SECONDS)).toBe('59m ago');
};

const testTimeAgoHours = (mockNow: number): void => {
  expect(formatTimeAgo(mockNow - THREE_THOUSAND_SIX_HUNDRED_SECONDS)).toBe('1h ago');
  expect(formatTimeAgo(mockNow - FORTY_THREE_THOUSAND_TWO_HUNDRED_SECONDS)).toBe('12h ago');
  expect(formatTimeAgo(mockNow - EIGHTY_SIX_THOUSAND_THREE_HUNDRED_FORTY_SECONDS)).toBe('23h ago');
};

const testTimeAgoDays = (mockNow: number): void => {
  expect(formatTimeAgo(mockNow - EIGHTY_SIX_THOUSAND_FOUR_HUNDRED_SECONDS)).toBe('1d ago');
  expect(formatTimeAgo(mockNow - FOUR_HUNDRED_THIRTY_TWO_THOUSAND_SECONDS)).toBe('5d ago');
  expect(formatTimeAgo(mockNow - SIX_HUNDRED_FOUR_THOUSAND_SEVEN_HUNDRED_FORTY_SECONDS)).toBe(
    '6d ago',
  );
};

const testTimeAgoWeeks = (mockNow: number): void => {
  expect(formatTimeAgo(mockNow - SIX_HUNDRED_FOUR_THOUSAND_EIGHT_HUNDRED_SECONDS)).toBe('1w ago');
  expect(
    formatTimeAgo(mockNow - TWO_MILLION_FOUR_HUNDRED_NINETEEN_THOUSAND_TWO_HUNDRED_SECONDS),
  ).toBe('4w ago');
};

const testTimeAgoMonths = (mockNow: number): void => {
  expect(formatTimeAgo(mockNow - TWO_MILLION_FIVE_HUNDRED_NINETY_TWO_THOUSAND_ONE_SECONDS)).toBe(
    '1mo ago',
  );
  expect(formatTimeAgo(mockNow - FIFTEEN_MILLION_FIVE_HUNDRED_FIFTY_TWO_THOUSAND_SECONDS)).toBe(
    '6mo ago',
  );
  expect(
    formatTimeAgo(mockNow - TWENTY_EIGHT_MILLION_NINE_HUNDRED_FORTY_FOUR_THOUSAND_SECONDS),
  ).toBe('11mo ago');
};

const testTimeAgoYears = (mockNow: number): void => {
  expect(
    formatTimeAgo(mockNow - THIRTY_ONE_MILLION_FIVE_HUNDRED_THIRTY_SIX_THOUSAND_ONE_SECONDS),
  ).toBe('1y ago');
  expect(formatTimeAgo(mockNow - SIXTY_THREE_MILLION_SEVENTY_TWO_THOUSAND_SECONDS)).toBe('2y ago');
};

const testTimeAgoFuture = (mockNow: number): void => {
  expect(formatTimeAgo(mockNow + SIXTY_SECONDS)).toBe('Just now');
};

describe('format', () => {
  describe('formatTimeAgo', () => {
    let mockNow: number = FIXED_TIMESTAMP;

    beforeEach(() => {
      mockNow = FIXED_TIMESTAMP;
      vi.spyOn(Date, 'now').mockImplementation(() => mockNow * MILLISECONDS_MULTIPLIER);
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });

    it('returns empty string for falsy timestamp', () => {
      testTimeAgoFalsy(mockNow);
    });

    it("returns 'Just now' for times less than 1 minute ago", () => {
      testTimeAgoJustNow(mockNow);
    });

    it('returns minutes for times within the last hour', () => {
      testTimeAgoMinutes(mockNow);
    });

    it('returns hours for times within the last day', () => {
      testTimeAgoHours(mockNow);
    });

    it('returns days for times within the last week', () => {
      testTimeAgoDays(mockNow);
    });

    it('returns weeks for times within the last month', () => {
      testTimeAgoWeeks(mockNow);
    });

    it('returns months for times within the last year', () => {
      testTimeAgoMonths(mockNow);
    });

    it('returns years for times over a year ago', () => {
      testTimeAgoYears(mockNow);
    });

    it('handles future timestamps gracefully', () => {
      testTimeAgoFuture(mockNow);
    });
  });

  describe('formatDuration', () => {
    const ONE = 1;
    const THIRTY = 30;
    const FIFTY_NINE = 59;
    const SIXTY = 60;
    const FIVE_HUNDRED_NINETY_NINE = 599;
    const THREE_THOUSAND_FIVE_HUNDRED_NINETY_NINE = 3599;
    const THREE_THOUSAND_SIX_HUNDRED = 3600;
    const THREE_THOUSAND_SIX_HUNDRED_SIXTY_ONE = 3661;
    const SEVEN_THOUSAND_TWO_HUNDRED = 7200;
    const SEVEN_THOUSAND_THREE_HUNDRED_TWENTY_THREE = 7323;

    it('formats durations under 1 hour as M:SS', () => {
      const ZERO = 0;
      expect(formatDuration(ZERO)).toBe('0:00');
      expect(formatDuration(THIRTY)).toBe('0:30');
      expect(formatDuration(FIFTY_NINE)).toBe('0:59');
      expect(formatDuration(SIXTY)).toBe('1:00');
      expect(formatDuration(FIVE_HUNDRED_NINETY_NINE)).toBe('9:59');
      expect(formatDuration(THREE_THOUSAND_FIVE_HUNDRED_NINETY_NINE)).toBe('59:59');
    });

    it('formats durations over 1 hour as H:MM:SS', () => {
      expect(formatDuration(THREE_THOUSAND_SIX_HUNDRED)).toBe('1:00:00');
      expect(formatDuration(THREE_THOUSAND_SIX_HUNDRED_SIXTY_ONE)).toBe('1:01:01');
      expect(formatDuration(SEVEN_THOUSAND_TWO_HUNDRED)).toBe('2:00:00');
      expect(formatDuration(SEVEN_THOUSAND_THREE_HUNDRED_TWENTY_THREE)).toBe('2:02:03');
    });

    it('handles edge cases', () => {
      expect(formatDuration(ONE)).toBe('0:01');
      expect(formatDuration(THREE_THOUSAND_SIX_HUNDRED + FIFTY_NINE * SIXTY + FIFTY_NINE)).toBe(
        '1:59:59',
      );
    });
  });

  describe('formatViewCount', () => {
    const ONE = 1;
    const NINE_HUNDRED_NINETY_NINE = 999;
    const ONE_THOUSAND = 1000;
    const ONE_THOUSAND_FIVE_HUNDRED = 1500;
    const NINE_HUNDRED_NINETY_NINE_THOUSAND_NINE_HUNDRED_NINETY_NINE = 999_999;
    const ONE_MILLION = 1_000_000;
    const TWO_MILLION_FIVE_HUNDRED_THOUSAND = 2_500_000;
    const TEN_MILLION = 10_000_000;

    it('returns numbers under 1000 as-is', () => {
      const ZERO = 0;
      expect(formatViewCount(ZERO)).toBe('0');
      expect(formatViewCount(ONE)).toBe('1');
      expect(formatViewCount(NINE_HUNDRED_NINETY_NINE)).toBe('999');
    });

    it('formats thousands with K suffix', () => {
      expect(formatViewCount(ONE_THOUSAND)).toBe('1.0K');
      expect(formatViewCount(ONE_THOUSAND_FIVE_HUNDRED)).toBe('1.5K');
      expect(formatViewCount(NINE_HUNDRED_NINETY_NINE_THOUSAND_NINE_HUNDRED_NINETY_NINE)).toBe(
        '1000.0K',
      );
    });

    it('formats millions with M suffix', () => {
      expect(formatViewCount(ONE_MILLION)).toBe('1.0M');
      expect(formatViewCount(TWO_MILLION_FIVE_HUNDRED_THOUSAND)).toBe('2.5M');
      expect(formatViewCount(TEN_MILLION)).toBe('10.0M');
    });

    it('handles exact boundaries', () => {
      expect(formatViewCount(NINE_HUNDRED_NINETY_NINE)).toBe('999');
      expect(formatViewCount(ONE_THOUSAND)).toBe('1.0K');
      expect(formatViewCount(NINE_HUNDRED_NINETY_NINE_THOUSAND_NINE_HUNDRED_NINETY_NINE)).toBe(
        '1000.0K',
      );
      expect(formatViewCount(ONE_MILLION)).toBe('1.0M');
    });
  });
});
