import { expect, test } from "bun:test";
import { execFileSync, spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const macosTest = process.platform === "darwin" ? test : test.skip;

macosTest(
  "simulated MSVC discovery replaces cached libraries together with headers",
  () => {
    const directory = mkdtempSync(join(tmpdir(), "ruf-openssl-msvc-"));
    try {
      writeFileSync(
        join(directory, "CMakeLists.txt"),
        `
cmake_minimum_required(VERSION 3.22)
project(simulated_msvc C)
set(WIN32 TRUE)
set(MSVC TRUE)
set(APPLE FALSE)
set(CMAKE_FIND_LIBRARY_SUFFIXES ".lib")
set(CMAKE_FIND_LIBRARY_PREFIXES "")
add_subdirectory("${resolve("native/openssl-probe")}" probe)
`,
      );
      const build = join(directory, "build");
      for (const [name, version] of [
        ["first", "3.0.0"],
        ["replacement", "3.1.0"],
      ] as const) {
        const root = join(directory, name);
        mkdirSync(join(root, "include/openssl"), { recursive: true });
        mkdirSync(join(root, "lib"));
        writeFileSync(join(root, "include/openssl/ssl.h"), "\n");
        writeFileSync(
          join(root, "include/openssl/opensslv.h"),
          `#define OPENSSL_VERSION_STR "${version}"\n`,
        );
        for (const name of ["ssl", "crypto"])
          writeFileSync(
            join(root, `lib/lib${name}.lib`),
            "discovery fixture; never linked\n",
          );
        const result = spawnSync(
          "cmake",
          [
            "-S",
            directory,
            "-B",
            build,
            "-DCMAKE_BUILD_TYPE=Release",
            `-DOPENSSL_ROOT_DIR=${root}`,
          ],
          { encoding: "utf8" },
        );
        expect(result.status).toBe(0);
        const resolved = JSON.parse(
          readFileSync(join(build, "openssl-Release.json"), "utf8"),
        );
        expect(resolved.include).toBe(join(root, "include"));
        expect(resolved.version).toBe(version);
        expect(resolved.ssl).toBe(join(root, "lib/libssl.lib"));
        expect(resolved.crypto).toBe(join(root, "lib/libcrypto.lib"));
      }
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  },
);

macosTest(
  "OpenSSL probe rejects newer object targets in either archive",
  () => {
    const directory = mkdtempSync(join(tmpdir(), "ruf-openssl-"));
    try {
      const source = join(directory, "fixture.c");
      writeFileSync(source, "int fixture(void) { return 0; }\n");
      const prefix = (name: string, ssl: string[], crypto: string[]) => {
        const root = join(directory, name);
        mkdirSync(join(root, "include/openssl"), { recursive: true });
        mkdirSync(join(root, "lib"));
        writeFileSync(join(root, "include/openssl/ssl.h"), "\n");
        writeFileSync(
          join(root, "include/openssl/opensslv.h"),
          '#define OPENSSL_VERSION_STR "3.0.0"\n',
        );
        for (const [library, versions] of [
          ["ssl", ssl],
          ["crypto", crypto],
        ] as const) {
          const objects = versions.map((version, index) => {
            const object = join(root, `${library}-${index}.o`);
            execFileSync("clang", [
              "-c",
              source,
              `-mmacosx-version-min=${version}`,
              "-o",
              object,
            ]);
            return object;
          });
          execFileSync("ar", [
            "rcs",
            join(root, `lib/lib${library}.a`),
            ...objects,
          ]);
        }
        return root;
      };
      const build = join(directory, "probe");
      const probe = (root: string) =>
        spawnSync(
          "cmake",
          [
            "-S",
            resolve("native/openssl-probe"),
            "-B",
            build,
            "-DCMAKE_BUILD_TYPE=Release",
            "-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0",
            `-DOPENSSL_ROOT_DIR=${root}`,
          ],
          { encoding: "utf8" },
        );
      const compatible = prefix("compatible", ["14.0"], ["14.0"]);
      expect(probe(compatible).status).toBe(0);
      for (const [name, ssl, crypto] of [
        ["new-ssl", ["15.0"], ["14.0"]],
        ["new-crypto", ["14.0"], ["27.0"]],
        ["mixed-archive", ["14.0", "15.0"], ["14.0"]],
      ] as const) {
        const result = probe(prefix(name, [...ssl], [...crypto]));
        expect(result.status).not.toBe(0);
        expect(result.stderr).toContain("requires macOS");
      }
      const replacement = prefix("replacement", ["14.0"], ["14.0"]);
      expect(probe(replacement).status).toBe(0);
      const resolved = JSON.parse(
        readFileSync(join(build, "openssl-Release.json"), "utf8"),
      );
      expect(resolved.ssl).toBe(join(replacement, "lib/libssl.a"));
      expect(resolved.crypto).toBe(join(replacement, "lib/libcrypto.a"));
      expect(resolved.include).toBe(join(replacement, "include"));
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  },
);
