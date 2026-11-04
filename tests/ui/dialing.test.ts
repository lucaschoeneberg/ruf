import { describe, expect, test } from "bun:test";
import { insertDigit, removeDigit } from "../../apps/desktop/src/lib/dialing";

describe("softphone number editing", () => {
  test("inserts at the caret and replaces selected digits", () => {
    expect(insertDigit("1234", "9", 2, 2)).toEqual({
      value: "12934",
      caret: 3,
    });
    expect(insertDigit("1234", "+", 0, 2)).toEqual({ value: "+34", caret: 1 });
  });
  test("deletes the selection or the character before the caret", () => {
    expect(removeDigit("1234", 1, 3)).toEqual({ value: "14", caret: 1 });
    expect(removeDigit("1234", 2, 2)).toEqual({ value: "134", caret: 1 });
    expect(removeDigit("1234", 0, 0)).toEqual({ value: "1234", caret: 0 });
  });
  test("preserves a full number rather than truncating it", () => {
    const full = "1".repeat(64);
    expect(insertDigit(full, "2", 64, 64)).toEqual({ value: full, caret: 64 });
    expect(insertDigit(full, "2", 0, 1).value).toBe("2" + "1".repeat(63));
  });
  test("accepts telephone keys and rejects unsupported input", () => {
    for (const digit of ["0", "9", "*", "#", "+"]) {
      expect(insertDigit("", digit, 0, 0).value).toBe(digit);
    }
    expect(insertDigit("123", "a", 3, 3).value).toBe("123");
    expect(insertDigit("123", "12", 3, 3).value).toBe("123");
  });
});
