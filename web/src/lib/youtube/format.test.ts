import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { formatTimeAgo, formatDuration, formatViewCount } from "./format";

describe("format", () => {
  describe("formatTimeAgo", () => {
    let mockNow: number;

    beforeEach(() => {
      mockNow = 1000000000; // Fixed timestamp
      vi.spyOn(Date, "now").mockImplementation(() => mockNow * 1000);
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });

    it("returns empty string for falsy timestamp", () => {
      expect(formatTimeAgo(0)).toBe("");
      expect(formatTimeAgo(NaN)).toBe("");
    });

    it("returns 'Just now' for times less than 1 minute ago", () => {
      expect(formatTimeAgo(mockNow - 30)).toBe("Just now");
      expect(formatTimeAgo(mockNow - 59)).toBe("Just now");
    });

    it("returns minutes for times within the last hour", () => {
      expect(formatTimeAgo(mockNow - 60)).toBe("1m ago");
      expect(formatTimeAgo(mockNow - 1800)).toBe("30m ago");
      expect(formatTimeAgo(mockNow - 3540)).toBe("59m ago");
    });

    it("returns hours for times within the last day", () => {
      expect(formatTimeAgo(mockNow - 3600)).toBe("1h ago");
      expect(formatTimeAgo(mockNow - 43200)).toBe("12h ago");
      expect(formatTimeAgo(mockNow - 86340)).toBe("23h ago");
    });

    it("returns days for times within the last week", () => {
      expect(formatTimeAgo(mockNow - 86400)).toBe("1d ago");
      expect(formatTimeAgo(mockNow - 432000)).toBe("5d ago");
      expect(formatTimeAgo(mockNow - 604740)).toBe("6d ago");
    });

    it("returns weeks for times within the last month", () => {
      expect(formatTimeAgo(mockNow - 604800)).toBe("1w ago");
      expect(formatTimeAgo(mockNow - 2419200)).toBe("4w ago"); // 28 days
    });

    it("returns months for times within the last year", () => {
      expect(formatTimeAgo(mockNow - 2592001)).toBe("1mo ago"); // ~30 days
      expect(formatTimeAgo(mockNow - 15552000)).toBe("6mo ago"); // ~180 days
      expect(formatTimeAgo(mockNow - 28944000)).toBe("11mo ago"); // ~335 days, less than a year
    });

    it("returns years for times over a year ago", () => {
      expect(formatTimeAgo(mockNow - 31536001)).toBe("1y ago");
      expect(formatTimeAgo(mockNow - 63072000)).toBe("2y ago");
    });

    it("handles future timestamps gracefully", () => {
      // Future times will show negative durations
      expect(formatTimeAgo(mockNow + 60)).toBe("Just now");
    });
  });

  describe("formatDuration", () => {
    it("formats durations under 1 hour as M:SS", () => {
      expect(formatDuration(0)).toBe("0:00");
      expect(formatDuration(30)).toBe("0:30");
      expect(formatDuration(59)).toBe("0:59");
      expect(formatDuration(60)).toBe("1:00");
      expect(formatDuration(599)).toBe("9:59");
      expect(formatDuration(3599)).toBe("59:59");
    });

    it("formats durations over 1 hour as H:MM:SS", () => {
      expect(formatDuration(3600)).toBe("1:00:00");
      expect(formatDuration(3661)).toBe("1:01:01");
      expect(formatDuration(7200)).toBe("2:00:00");
      expect(formatDuration(7323)).toBe("2:02:03");
    });

    it("handles edge cases", () => {
      expect(formatDuration(1)).toBe("0:01");
      expect(formatDuration(3600 + 59 * 60 + 59)).toBe("1:59:59");
    });
  });

  describe("formatViewCount", () => {
    it("returns numbers under 1000 as-is", () => {
      expect(formatViewCount(0)).toBe("0");
      expect(formatViewCount(1)).toBe("1");
      expect(formatViewCount(999)).toBe("999");
    });

    it("formats thousands with K suffix", () => {
      expect(formatViewCount(1000)).toBe("1.0K");
      expect(formatViewCount(1500)).toBe("1.5K");
      expect(formatViewCount(999999)).toBe("1000.0K");
    });

    it("formats millions with M suffix", () => {
      expect(formatViewCount(1000000)).toBe("1.0M");
      expect(formatViewCount(2500000)).toBe("2.5M");
      expect(formatViewCount(10000000)).toBe("10.0M");
    });

    it("handles exact boundaries", () => {
      expect(formatViewCount(999)).toBe("999");
      expect(formatViewCount(1000)).toBe("1.0K");
      expect(formatViewCount(999999)).toBe("1000.0K");
      expect(formatViewCount(1000000)).toBe("1.0M");
    });
  });
});
