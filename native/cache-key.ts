// Fingerprint the hosted compiler image separately from the tracked SDK recipes.
import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { join } from "node:path";

const output = (command: string, args: string[] = []): string =>
  execFileSync(command, args, { encoding: "utf8" }).trim();
const identity: {
  platform: NodeJS.Platform;
  arch: string;
  image: string | undefined;
  imageVersion: string | undefined;
  cmake: string;
  compiler?: string;
  vcpkg?: string;
  xcode?: string;
  sdk?: string;
} = {
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
  const programFiles = process.env["ProgramFiles(x86)"];
  const vcpkg = process.env.VCPKG_INSTALLATION_ROOT ?? process.env.VCPKG_ROOT;
  if (!programFiles || !vcpkg)
    throw new Error(
      "Native Windows caching requires ProgramFiles(x86) and a vcpkg installation root",
    );
  const installation = output(
    join(programFiles, "Microsoft Visual Studio/Installer/vswhere.exe"),
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
  const version = (
    await Bun.file(
      join(
        installation,
        "VC/Auxiliary/Build/Microsoft.VCToolsVersion.default.txt",
      ),
    ).text()
  ).trim();
  identity.compiler = new Bun.CryptoHasher("sha256")
    .update(
      await Bun.file(
        join(installation, "VC/Tools/MSVC", version, "bin/Hostx64/x64/cl.exe"),
      ).bytes(),
    )
    .digest("hex");
  identity.vcpkg = output(join(vcpkg, "vcpkg.exe"), ["version"]);
} else if (process.platform === "darwin") {
  identity.compiler = output("cc", ["--version"]);
  identity.xcode = output("xcodebuild", ["-version"]);
  identity.sdk = output("xcrun", ["--show-sdk-version"]);
} else {
  throw new Error("Native SDK caching is configured for desktop hosts only");
}

const fingerprint = new Bun.CryptoHasher("sha256")
  .update(JSON.stringify(identity))
  .digest("hex");
console.log(JSON.stringify(identity, null, 2));
console.log(`Native SDK toolchain fingerprint: ${fingerprint}`);
if (process.env.GITHUB_OUTPUT)
  appendFileSync(process.env.GITHUB_OUTPUT, `fingerprint=${fingerprint}\n`);
