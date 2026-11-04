import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const pins = JSON.parse(
  readFileSync(new URL("sources.json", import.meta.url), "utf8"),
);
const targets = {
  "darwin-arm64": "aarch64-apple-darwin",
  "win32-x64": "x86_64-pc-windows-msvc",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
};
const target = targets[`${process.platform}-${process.arch}`];
if (!target)
  throw new Error(
    "Unsupported native build host; use macOS arm64, Windows x64 or Linux x64/arm64.",
  );
const windows = process.platform === "win32";
const build = join(root, "native/build", target);
const prefix = join(root, "native/install", target);
const sources = join(root, "native/build/sources");
const stageFile = (source, destination) => {
  if (resolve(source) === resolve(destination)) return;
  rmSync(destination, { force: true });
  copyFileSync(source, destination);
};
const run = (exe, args, cwd = root) =>
  execFileSync(exe, args, { cwd, stdio: "inherit" });
const output = (exe, args, cwd = root) =>
  execFileSync(exe, args, { cwd, encoding: "utf8" }).trim();
mkdirSync(build, { recursive: true });
mkdirSync(prefix, { recursive: true });

for (const [name, pin] of Object.entries(pins)) {
  const source = join(sources, name);
  if (!existsSync(join(source, ".git"))) {
    mkdirSync(source, { recursive: true });
    run("git", ["init", source]);
    run("git", ["remote", "add", "origin", pin.url], source);
    run("git", ["fetch", "--depth", "1", "origin", pin.commit], source);
    run("git", ["checkout", "--detach", "FETCH_HEAD"], source);
  }
  if (
    output("git", ["rev-parse", "HEAD"], source) !== pin.commit ||
    output("git", ["status", "--porcelain"], source)
  ) {
    throw new Error(
      `Refusing changed or incorrectly pinned native source: ${name}`,
    );
  }
}

let openssl = process.env.OPENSSL_ROOT_DIR;
if (!openssl && process.platform === "darwin")
  openssl = output("brew", ["--prefix", "openssl@3"]);
if (windows && !openssl) {
  const vcpkg = process.env.VCPKG_INSTALLATION_ROOT ?? process.env.VCPKG_ROOT;
  if (!vcpkg)
    throw new Error(
      "Set VCPKG_INSTALLATION_ROOT or OPENSSL_ROOT_DIR for a static-MD OpenSSL installation.",
    );
  const installed = join(build, "vcpkg");
  run(join(vcpkg, "vcpkg.exe"), [
    "install",
    `--x-manifest-root=${join(root, "native")}`,
    `--x-install-root=${installed}`,
    "--triplet=x64-windows-static-md",
  ]);
  openssl = join(installed, "x64-windows-static-md");
}
const common = [
  "-DCMAKE_BUILD_TYPE=Release",
  `-DCMAKE_INSTALL_PREFIX=${prefix}`,
  "-DCMAKE_INSTALL_LIBDIR=lib",
  "-DOPENSSL_USE_STATIC_LIBS=TRUE",
  "-DUSE_OPENSSL=ON",
  "-DCMAKE_DISABLE_FIND_PACKAGE_ZLIB=TRUE",
];
if (openssl) common.push(`-DOPENSSL_ROOT_DIR=${resolve(openssl)}`);
if (windows)
  common.push("-A", "x64", "-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL");
if (process.platform === "darwin")
  common.push(
    "-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0",
    "-DCMAKE_OSX_ARCHITECTURES=arm64",
  );

const reBuild = join(build, "re");
run("cmake", [
  "-S",
  join(sources, "re"),
  "-B",
  reBuild,
  ...common,
  "-DLIBRE_BUILD_SHARED=OFF",
  "-DLIBRE_BUILD_STATIC=ON",
]);
run("cmake", [
  "--build",
  reBuild,
  "--config",
  "Release",
  "--target",
  "re",
  "--parallel",
  "4",
]);
run("cmake", ["--install", reBuild, "--config", "Release"]);

const modules = ["g711", "auconv", "auresamp", "ausine", "aufile"];
if (process.platform === "darwin") modules.push("audiounit");
if (windows) modules.push("wasapi");
const bareBuild = join(build, "baresip");
const reLibrary = join(prefix, "lib", windows ? "re-static.lib" : "libre.a");
run("cmake", [
  "-S",
  join(sources, "baresip"),
  "-B",
  bareBuild,
  ...common,
  `-DCMAKE_PREFIX_PATH=${prefix}`,
  `-DRE_INCLUDE_DIR=${join(prefix, "include/re")}`,
  `-DRE_LIBRARY=${reLibrary}`,
  "-DSTATIC=ON",
  `-DMODULES=${modules.join(";")}`,
]);
run("cmake", [
  "--build",
  bareBuild,
  "--config",
  "Release",
  "--target",
  "baresip",
  "--parallel",
  "4",
]);
run("cmake", [
  "--install",
  bareBuild,
  "--config",
  "Release",
  "--component",
  "Development",
]);

// Normalize archive names for Rust's platform-independent static link directives.
const probe = join(build, "openssl-probe");
run("cmake", [
  "-S",
  join(root, "native/openssl-probe"),
  "-B",
  probe,
  ...common,
]);
const resolved = JSON.parse(
  readFileSync(join(probe, "openssl-Release.json"), "utf8"),
);
for (const name of ["ssl", "crypto"]) {
  const path = resolved[name];
  if (!path || (!path.endsWith(".a") && !path.endsWith(".lib")))
    throw new Error(`Static ${name} was not resolved`);
  stageFile(
    path,
    join(prefix, "lib", windows ? `${name}.lib` : `lib${name}.a`),
  );
}
if (windows) {
  stageFile(
    join(prefix, "lib/libbaresip.lib"),
    join(prefix, "lib/baresip.lib"),
  );
  stageFile(reLibrary, join(prefix, "lib/re.lib"));
}
const licenses = join(prefix, "licenses");
mkdirSync(licenses, { recursive: true });
for (const name of Object.keys(pins))
  stageFile(join(sources, name, "LICENSE"), join(licenses, `${name}-LICENSE`));
const opensslLicense =
  openssl &&
  [
    join(openssl, "share/doc/openssl/LICENSE.txt"),
    join(openssl, "share/openssl/copyright"),
    join(openssl, "LICENSE.txt"),
    join(openssl, "share/openssl/copyright"),
  ].find(existsSync);
if (opensslLicense)
  stageFile(opensslLicense, join(licenses, "openssl-LICENSE"));
writeFileSync(
  join(prefix, "sdk.json"),
  JSON.stringify(
    {
      target,
      sources: pins,
      modules,
      openssl: { prefix: openssl ?? "system", version: resolved.version },
      licenseInventoryComplete: false,
    },
    null,
    2,
  ) + "\n",
);
console.log(`Native SDK staged at ${prefix}`);
