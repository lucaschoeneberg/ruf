// Build an isolated Mac App Store candidate; never upload or submit it.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  cpSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const tauri = join(root, "apps/desktop/src-tauri");

// Only call this on staged packaging inputs, never on the installed app.
export function assertNoQuarantineAttributes(path) {
  const attributes = execFileSync("/usr/bin/xattr", ["-lrs", path], {
    encoding: "utf8",
  });
  if (attributes.includes("com.apple.quarantine:"))
    throw new Error(
      "App Store payload contains com.apple.quarantine attributes.",
    );
}

export function cleanAppStoreBundle(path) {
  // -s prevents a bundle symlink from changing files outside the staging tree.
  execFileSync("/usr/bin/xattr", ["-crs", path]);
  assertNoQuarantineAttributes(path);
}

export function validateProvisioningProfile(
  profile,
  identifier,
  certificate,
  now = new Date(),
) {
  const team = profile.TeamIdentifier?.[0];
  const entitlements = profile.Entitlements ?? {};
  if (!profile.Platform?.includes("OSX"))
    throw new Error("A macOS profile is required.");
  if (!(new Date(profile.ExpirationDate) > now))
    throw new Error(
      "The provisioning profile is expired or has no valid expiry.",
    );
  if (
    entitlements["get-task-allow"] ||
    profile.ProvisionedDevices?.length ||
    profile.ProvisionsAllDevices
  )
    throw new Error(
      "A Mac App Store distribution profile is required; development/ad-hoc profiles are not accepted.",
    );
  if (
    !/^[A-Z0-9]{10}$/.test(team ?? "") ||
    entitlements["com.apple.developer.team-identifier"] !== team
  )
    throw new Error(
      "The provisioning profile has inconsistent team identifiers.",
    );
  if (
    entitlements["com.apple.application-identifier"] !== `${team}.${identifier}`
  )
    throw new Error(
      "The provisioning profile Bundle ID must exactly match Ruf.",
    );
  if (!profile.DeveloperCertificates?.includes(certificate.toUpperCase()))
    throw new Error(
      "The selected app signing certificate is not included in the provisioning profile.",
    );
  return team;
}

function main() {
  if (process.platform !== "darwin")
    throw new Error("Mac App Store packaging requires macOS.");
  const args = process.argv.slice(2);
  if (args.some((arg) => !["--unsigned", "--check"].includes(arg)))
    throw new Error("Usage: bun run appstore:prepare [--unsigned] [--check]");
  const unsigned = args.includes("--unsigned");
  const env = process.env;
  const run = (command, arguments_, options = {}) =>
    execFileSync(command, arguments_, {
      cwd: root,
      encoding: "utf8",
      timeout: 600000,
      ...options,
    });
  const json = (path) => JSON.parse(readFileSync(path, "utf8"));
  const base = json(join(tauri, "tauri.conf.json"));
  const version = json(join(root, "package.json")).version;
  const build = env.RUF_APPSTORE_BUILD_NUMBER ?? "1";
  if (!/^\d+(?:\.\d+){0,2}$/.test(build))
    throw new Error(
      "RUF_APPSTORE_BUILD_NUMBER must contain one to three numeric components.",
    );
  const encryption = env.RUF_APPSTORE_NON_EXEMPT_ENCRYPTION;
  if (encryption !== undefined && !["true", "false"].includes(encryption))
    throw new Error(
      "RUF_APPSTORE_NON_EXEMPT_ENCRYPTION must be true or false.",
    );
  const target = run("rustc", ["-vV"]).match(/^host: (.+)$/m)?.[1];
  if (!target?.endsWith("-apple-darwin"))
    throw new Error("A native macOS Rust toolchain is required.");
  const output = join(root, "artifacts/app-store/build");
  const configuration = json(join(tauri, "tauri.appstore.conf.json"));
  let profile, team, applicationIdentity, installerIdentity;
  const missing = [];
  if (!unsigned) {
    for (const key of [
      "RUF_APPSTORE_APP_IDENTITY",
      "RUF_APPSTORE_INSTALLER_IDENTITY",
      "RUF_APPSTORE_PROFILE",
      "RUF_APPSTORE_NON_EXEMPT_ENCRYPTION",
    ])
      if (!env[key]) missing.push(key);
    if (missing.length)
      throw new Error(
        `Store signing inputs missing: ${missing.join(", ")}. Use --unsigned for a clearly labelled local candidate.`,
      );
    const identities = [
      ...run("/usr/bin/security", ["find-identity", "-v"]).matchAll(
        /([A-Fa-f0-9]{40}) "([^"]+)"/g,
      ),
    ].map((match) => ({ sha1: match[1].toUpperCase(), name: match[2] }));
    const identity = (value, pattern) => {
      const matches = identities.filter(
        (item) =>
          (item.sha1 === value.toUpperCase() || item.name === value) &&
          pattern.test(item.name),
      );
      if (matches.length !== 1)
        throw new Error(
          "Select one valid, locally available Mac App Store signing identity of the correct certificate type.",
        );
      return matches[0];
    };
    applicationIdentity = identity(
      env.RUF_APPSTORE_APP_IDENTITY,
      /^(Apple Distribution|3rd Party Mac Developer Application):/,
    );
    installerIdentity = identity(
      env.RUF_APPSTORE_INSTALLER_IDENTITY,
      /^(3rd Party Mac Developer Installer|Mac Installer Distribution):/,
    );
    profile = resolve(env.RUF_APPSTORE_PROFILE);
    // Decode only signing metadata. Never serialize the private keychain or credentials.
    const metadata = JSON.parse(
      run("python3", [
        "-c",
        `
import datetime, hashlib, json, plistlib, subprocess, sys
p = plistlib.loads(subprocess.check_output(['/usr/bin/security', 'cms', '-D', '-i', sys.argv[1]]))
p['ExpirationDate'] = p.get('ExpirationDate', datetime.datetime.min).replace(tzinfo=datetime.timezone.utc).isoformat()
p['DeveloperCertificates'] = [hashlib.sha1(c).hexdigest().upper() for c in p.get('DeveloperCertificates', [])]
print(json.dumps({k:p.get(k) for k in ['Platform','ExpirationDate','TeamIdentifier','Entitlements','DeveloperCertificates','ProvisionedDevices','ProvisionsAllDevices']}))
`,
        profile,
      ]),
    );
    team = validateProvisioningProfile(
      metadata,
      base.identifier,
      applicationIdentity.sha1,
    );
    if (!installerIdentity.name.endsWith(`(${team})`))
      throw new Error(
        "The installer signing identity belongs to a different team.",
      );
  }
  if (args.includes("--check")) {
    console.log(
      `APP_STORE_PREFLIGHT_OK ${unsigned ? "unsigned candidate only" : "distribution inputs verified"} ${base.identifier} ${target}`,
    );
    return;
  }
  mkdirSync(output, { recursive: true });
  rmSync(join(output, "readiness.json"), { force: true });
  const readPlist = (path) =>
    JSON.parse(run("/usr/bin/plutil", ["-convert", "json", "-o", "-", path]));
  const writePlist = (path, value) =>
    run("/usr/bin/plutil", ["-convert", "xml1", "-o", path, "-"], {
      input: JSON.stringify(value),
    });
  const entitlements = readPlist(join(tauri, "Entitlements.appstore.plist"));
  if (team) {
    entitlements["com.apple.application-identifier"] =
      `${team}.${base.identifier}`;
    entitlements["com.apple.developer.team-identifier"] = team;
    entitlements["keychain-access-groups"] = [`${team}.${base.identifier}`];
  }
  const entitlementsPath = join(output, "Entitlements.plist");
  writePlist(entitlementsPath, entitlements);
  const info = readPlist(join(tauri, "Info.plist"));
  if (encryption !== undefined)
    info.ITSAppUsesNonExemptEncryption = encryption === "true";
  const infoPath = join(output, "Info.plist");
  writePlist(infoPath, info);
  let stagedProfile;
  if (profile) {
    stagedProfile = join(output, "embedded.provisionprofile");
    copyFileSync(profile, stagedProfile);
    cleanAppStoreBundle(stagedProfile);
  }
  const configPath = join(output, "tauri.appstore.conf.json");
  configuration.bundle.macOS = {
    ...configuration.bundle.macOS,
    bundleVersion: build,
    signingIdentity: applicationIdentity?.name ?? "-",
    entitlements: entitlementsPath,
    infoPlist: infoPath,
    ...(stagedProfile
      ? { files: { "embedded.provisionprofile": stagedProfile } }
      : {}),
  };
  writeFileSync(configPath, JSON.stringify(configuration, null, 2) + "\n");
  // Store signing uses the selected local identities. Never import a certificate
  // or trigger notarization from unrelated direct-distribution environment keys.
  const buildEnvironment = { ...process.env };
  for (const key of [
    "APPLE_SIGNING_IDENTITY",
    "APPLE_CERTIFICATE",
    "APPLE_CERTIFICATE_PASSWORD",
    "APPLE_ID",
    "APPLE_PASSWORD",
    "APPLE_TEAM_ID",
    "APPLE_API_ISSUER",
    "APPLE_API_KEY",
    "APPLE_API_KEY_PATH",
  ])
    delete buildEnvironment[key];
  run(
    process.execPath,
    [
      join(root, "scripts/build-desktop.mjs"),
      "--target",
      target,
      "--features",
      "native,app-store",
      "--bundles",
      "app",
      "--config",
      configPath,
    ],
    { stdio: "inherit", env: buildEnvironment },
  );
  const metadata = JSON.parse(
    run("cargo", [
      "metadata",
      "--locked",
      "--no-deps",
      "--format-version",
      "1",
    ]),
  );
  const release = join(metadata.target_directory, target, "release");
  const app = join(output, "Ruf.app");
  rmSync(app, { recursive: true, force: true });
  cpSync(join(release, "bundle/macos/Ruf.app"), app, {
    recursive: true,
    force: true,
  });
  cleanAppStoreBundle(app);
  run("/usr/bin/codesign", [
    "--force",
    "--sign",
    applicationIdentity?.sha1 ?? "-",
    "--options",
    "runtime",
    "--entitlements",
    entitlementsPath,
    ...(!unsigned ? ["--timestamp"] : []),
    app,
  ]);
  run("/usr/bin/codesign", ["--verify", "--deep", "--strict", app]);
  const signedEntitlements = JSON.parse(
    run("/usr/bin/plutil", ["-convert", "json", "-o", "-", "-"], {
      input: run(
        "/usr/bin/codesign",
        ["--display", "--entitlements", "-", "--xml", app],
        { stdio: ["ignore", "pipe", "pipe"] },
      ),
    }),
  );
  for (const [key, value] of Object.entries(entitlements))
    if (JSON.stringify(signedEntitlements[key]) !== JSON.stringify(value))
      throw new Error(`The signed app lost required entitlement: ${key}`);
  const bundledInfo = readPlist(join(app, "Contents/Info.plist"));
  if (
    bundledInfo.CFBundleIdentifier !== base.identifier ||
    bundledInfo.CFBundleVersion !== build
  )
    throw new Error(
      "The packaged app has the wrong Bundle ID or build number.",
    );
  if (!unsigned) {
    const certificatePrefix = join(output, "signing-certificate-");
    run(
      "/usr/bin/codesign",
      ["--display", `--extract-certificates=${certificatePrefix}`, app],
      { stdio: ["ignore", "pipe", "pipe"] },
    );
    const actualCertificate = createHash("sha1")
      .update(readFileSync(`${certificatePrefix}0`))
      .digest("hex")
      .toUpperCase();
    if (actualCertificate !== applicationIdentity.sha1)
      throw new Error(
        "The actual app signature does not use the selected distribution certificate.",
      );
  }
  const pkg = join(
    output,
    `Ruf_${version}_${build}_${target}_${unsigned ? "unsigned" : "appstore"}.pkg`,
  );
  rmSync(pkg, { force: true });
  run(
    "/usr/bin/productbuild",
    [
      ...(installerIdentity ? ["--sign", installerIdentity.name] : []),
      "--component",
      app,
      "/Applications",
      pkg,
    ],
    { stdio: "inherit" },
  );
  if (!unsigned)
    run("/usr/sbin/pkgutil", ["--check-signature", pkg], { stdio: "inherit" });
  assertNoQuarantineAttributes(app);
  assertNoQuarantineAttributes(pkg);
  const expandedPackage = join(output, "package-contents-check");
  rmSync(expandedPackage, { recursive: true, force: true });
  run("/usr/sbin/pkgutil", ["--expand-full", pkg, expandedPackage]);
  assertNoQuarantineAttributes(expandedPackage);
  const hashes = {
    appExecutable: createHash("sha256")
      .update(readFileSync(join(app, "Contents/MacOS/sip-desktop")))
      .digest("hex"),
    package: createHash("sha256").update(readFileSync(pkg)).digest("hex"),
  };
  for (const extension of ["cdx.json", "build.json"])
    cpSync(
      join(release, `Ruf_${version}_${target}_native.${extension}`),
      join(output, `Ruf_${version}_${build}.${extension}`),
    );
  writeFileSync(
    join(output, "readiness.json"),
    JSON.stringify(
      {
        version,
        build,
        identifier: base.identifier,
        target,
        signedForAppStore: !unsigned,
        quarantineAttributesAbsent: true,
        expandedPackageChecked: true,
        encryptionClassificationDeclared: encryption !== undefined,
        hashes,
        package: pkg,
        remaining: unsigned
          ? [
              "Mac App Distribution certificate with private key",
              "Mac Installer Distribution certificate with private key",
              "Mac App Store provisioning profile",
              ...(encryption === undefined
                ? ["Encryption export classification"]
                : []),
              "Sandbox runtime verification with final distribution identity",
            ]
          : [
              "Sandbox runtime verification with final distribution identity",
              "Apple upload validation and review",
            ],
      },
      null,
      2,
    ) + "\n",
  );
  console.log(
    `APP_STORE_CANDIDATE_OK ${pkg}${unsigned ? " (UNSIGNED: not uploadable)" : ""}`,
  );
}

if (import.meta.main) {
  try {
    main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
