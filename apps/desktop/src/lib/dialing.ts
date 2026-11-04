/** The same destination length accepted by the native dial command. */
export const MAX_NUMBER_LENGTH = 64;

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
  if (!/^[0-9+*#]$/.test(digit) || next.length > MAX_NUMBER_LENGTH)
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
