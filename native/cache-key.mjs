// Fingerprint the hosted compiler image separately from the tracked SDK recipes.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { appendFileSync, readFileSync } from "node:fs";
import { join } from "node:path";

const output = (command, args = []) =>
  execFileSync(command, args, { encoding: "utf8" }).trim();
const identity = {
  platform: process.platform,
  arch: process.arch,
  image: process.env.ImageOS,
  imageVersion: process.env.ImageVersion,
  cmake: output("cmake", ["--version"]),
};
if (!identity.image || !identity.imageVersion)
  throw new Error(
    "Native CI caching requires the hosted runner image identity",
  );

if (process.platform === "win32") {
  const installation = output(
    join(
      process.env["ProgramFiles(x86)"],
      "Microsoft Visual Studio/Installer/vswhere.exe",
    ),
    [
      "-latest",
      "-products",
      "*",
      "-requires",
      "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
      "-property",
      "installationPath",
    ],
  );
  const version = readFileSync(
    join(
      installation,
      "VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt",
    ),
    "utf8",
  ).trim();
  identity.compiler = createHash("sha256")
    .update(
      readFileSync(
        join(installation, "VC/Tools/MSVC", version, "bin/Hostx64/x64/cl.exe"),
      ),
    )
    .digest("hex");
  identity.vcpkg = output(
    join(
      process.env.VCPKG_INSTALLATION_ROOT ?? process.env.VCPKG_ROOT,
      "vcpkg.exe",
    ),
    ["version"],
  );
} else if (process.platform === "darwin") {
  identity.compiler = output("cc", ["--version"]);
  identity.xcode = output("xcodebuild", ["-version"]);
  identity.sdk = output("xcrun", ["--show-sdk-version"]);
} else {
  throw new Error("Native SDK caching is configured for desktop hosts only");
}

const fingerprint = createHash("sha256")
  .update(JSON.stringify(identity))
  .digest("hex");
console.log(JSON.stringify(identity, null, 2));
console.log(`Native SDK toolchain fingerprint: ${fingerprint}`);
if (process.env.GITHUB_OUTPUT)
  appendFileSync(process.env.GITHUB_OUTPUT, `fingerprint=${fingerprint}\n`);
