/** The same destination length accepted by the native dial command. */
export const MAX_NUMBER_LENGTH = 64;
/** Allow readable formatting without interpreting vanity names or post-dial text. */
export function normalizeDialNumber(value: string): string | null {
  if (/[\u0000-\u001f\u007f-\u009f]/u.test(value)) return null;
  const compact = value.replace(/\s/gu, "");
  if (
    (compact.startsWith("+") || compact.startsWith("00")) &&
    compact.includes("(0)")
  )
    return null;
  if (!/^[0-9+*#()./\-]*$/.test(compact)) return null;
  const number = compact.replace(/[()./\-]/g, "");
  return number.length <= MAX_NUMBER_LENGTH &&
    /\d/.test(number) &&
    /^\+?[0-9*#]+$/.test(number) &&
    !(number.startsWith("+") && /[*#]/.test(number))
    ? number
    : null;
}

interface NumberEdit {
  value: string;
  caret: number;
}

/** Insert one telephone character without truncating a selected or full number. */
export function insertDigit(
  value: string,
  digit: string,
  start: number,
  end: number,
): NumberEdit {
  const caret = Math.max(0, Math.min(start, value.length));
  const selectionEnd = Math.max(caret, Math.min(end, value.length));
  const next = value.slice(0, caret) + digit + value.slice(selectionEnd);
  if (
    !/^[0-9+*#]$/.test(digit) ||
    next.replace(/[\s()./\-]/gu, "").length > MAX_NUMBER_LENGTH ||
    next.length > 128 ||
    (digit === "+" && (caret !== 0 || next.indexOf("+", 1) !== -1))
  )
    return { value, caret };
  return { value: next, caret: caret + 1 };
}

/** Backspace a selection or the digit immediately before the caret. */
export function removeDigit(
  value: string,
  start: number,
  end: number,
): NumberEdit {
  const caret = Math.max(0, Math.min(start, value.length));
  const selectionEnd = Math.max(caret, Math.min(end, value.length));
  const from = selectionEnd > caret ? caret : Math.max(0, caret - 1);
  return {
    value: value.slice(0, from) + value.slice(selectionEnd),
    caret: from,
  };
}
