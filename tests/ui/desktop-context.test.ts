import { expect, test } from "bun:test";
import { desktopContext } from "../../scripts/desktop-context.ts";

test("desktop context follows explicit target and both native feature flag spellings", () => {
  const windows = "x86_64-pc-windows-msvc";
  expect(
    desktopContext(
      ["--target", windows, "--features", "native"],
      "aarch64-apple-darwin",
    ),
  ).toEqual({
    target: windows,
    native: true,
    profile: "release",
    kind: "desktop",
  });
  expect(desktopContext(["--features=native", "--debug"], windows).native).toBe(
    true,
  );
  expect(desktopContext(["-f", "native", "-d"], windows).profile).toBe("debug");
  expect(desktopContext([], windows).native).toBe(false);
  expect(desktopContext([], windows, "aarch64-apple-darwin").target).toBe(
    "aarch64-apple-darwin",
  );
});

test("ambiguous multi-architecture outputs are rejected", () => {
  expect(() =>
    desktopContext(
      ["--target", "universal-apple-darwin"],
      "aarch64-apple-darwin",
    ),
  ).toThrow();
  expect(() =>
    desktopContext(
      [
        "--target",
        "aarch64-apple-darwin",
        "--target",
        "x86_64-pc-windows-msvc",
      ],
      "aarch64-apple-darwin",
    ),
  ).toThrow();
});
