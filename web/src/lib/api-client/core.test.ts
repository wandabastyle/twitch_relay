import { describe, it, expect, vi } from "vitest";
import { isObject, safeJson, readApiError, request } from "./core";

describe("api-client/core", () => {
  describe("isObject", () => {
    it("returns true for plain objects", () => {
      expect(isObject({})).toBe(true);
      expect(isObject({ a: 1 })).toBe(true);
      expect(isObject({ nested: { value: "test" } })).toBe(true);
    });

    it("returns false for null", () => {
      expect(isObject(null)).toBe(false);
    });

    it("returns true for arrays (arrays are objects in JS)", () => {
      // Note: arrays are technically objects in JavaScript
      expect(isObject([])).toBe(true);
      expect(isObject([1, 2, 3])).toBe(true);
    });

    it("returns false for primitives", () => {
      expect(isObject("string")).toBe(false);
      expect(isObject(123)).toBe(false);
      expect(isObject(true)).toBe(false);
      expect(isObject(undefined)).toBe(false);
      expect(isObject(Symbol("test"))).toBe(false);
      expect(isObject(BigInt(123))).toBe(false);
    });

    it("returns false for functions", () => {
      expect(isObject(() => {})).toBe(false);
      expect(isObject(function () {})).toBe(false);
    });
  });

  describe("safeJson", () => {
    it("returns parsed JSON on success", async () => {
      const data = { test: "value", number: 42 };
      const response = new Response(JSON.stringify(data));

      const result = await safeJson(response);
      expect(result).toEqual(data);
    });

    it("returns null on parse error", async () => {
      const response = new Response("not valid json");

      const result = await safeJson(response);
      expect(result).toBeNull();
    });

    it("returns null on network error", async () => {
      const response = {
        json: () => Promise.reject(new Error("Network error")),
      } as Response;

      const result = await safeJson(response);
      expect(result).toBeNull();
    });

    it("handles empty string response", async () => {
      const response = new Response("");

      const result = await safeJson(response);
      expect(result).toBeNull();
    });
  });

  describe("readApiError", () => {
    it("extracts error message from valid error object", () => {
      const payload = { error: "Something went wrong" };
      expect(readApiError(payload)).toBe("Something went wrong");
    });

    it("returns default message for non-object payload", () => {
      expect(readApiError("string")).toBe("request failed");
      expect(readApiError(123)).toBe("request failed");
      expect(readApiError(null)).toBe("request failed");
      expect(readApiError(undefined)).toBe("request failed");
      expect(readApiError([])).toBe("request failed");
    });

    it("returns default message when error property is missing", () => {
      expect(readApiError({})).toBe("request failed");
      expect(readApiError({ message: "test" })).toBe("request failed");
      expect(readApiError({ success: true })).toBe("request failed");
    });

    it("returns default message when error property is not a string", () => {
      expect(readApiError({ error: 123 })).toBe("request failed");
      expect(readApiError({ error: null })).toBe("request failed");
      expect(readApiError({ error: true })).toBe("request failed");
      expect(readApiError({ error: {} })).toBe("request failed");
      expect(readApiError({ error: [] })).toBe("request failed");
    });

    it("handles complex error objects", () => {
      const payload = {
        error: "Validation failed",
        details: { field: "name", reason: "required" },
      };
      expect(readApiError(payload)).toBe("Validation failed");
    });
  });

  describe("request", () => {
    it("calls fetch with correct parameters", async () => {
      const mockFetch = vi.fn().mockResolvedValue(new Response("{}"));
      globalThis.fetch = mockFetch;

      await request("/api/test");

      expect(mockFetch).toHaveBeenCalledWith("/api/test", {
        credentials: "same-origin",
      });
    });

    it("merges custom init options", async () => {
      const mockFetch = vi.fn().mockResolvedValue(new Response("{}"));
      globalThis.fetch = mockFetch;

      const init: RequestInit = {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ test: true }),
      };

      await request("/api/test", init);

      expect(mockFetch).toHaveBeenCalledWith("/api/test", {
        credentials: "same-origin",
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ test: true }),
      });
    });

    it("preserves credentials when init is provided", async () => {
      const mockFetch = vi.fn().mockResolvedValue(new Response("{}"));
      globalThis.fetch = mockFetch;

      await request("/api/test", { method: "GET" });

      expect(mockFetch).toHaveBeenCalledWith("/api/test", {
        credentials: "same-origin",
        method: "GET",
      });
    });

    it("returns the fetch response", async () => {
      const expectedResponse = new Response("{}", { status: 200 });
      globalThis.fetch = vi.fn().mockResolvedValue(expectedResponse);

      const result = await request("/api/test");

      expect(result).toBe(expectedResponse);
    });
  });
});
