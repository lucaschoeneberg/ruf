# Release cleanup

Goal: make the existing 0.1.0 preview reproducible and reviewable for a release
draft, preserving the changes already present in the working tree.

Reuse Svelte/Vite, generated ts-rs contracts, Bun/Cargo checks, native SDK scripts
and the existing GitHub Actions. No dependencies, API changes or broad runtime
refactoring are needed for this cleanup.

- [x] Share portable and browser checks between branch CI and tag releases;
      require them before creating a draft. Validate with actionlint.
- [x] Replace obsolete foundation release notes with versioned 0.1.0 notes;
      validate their existence before building, mark the preview as a prerelease
      and attach SHA-256 checksums. Exercise the shell steps with fake installers.
- [x] Update README, architecture, contributor guidance and CI documentation to
      describe saved accounts, OS credential storage, contacts, history and the
      actual desktop identity. Keep historical verification records intact.
- [x] Run formatting, version checks, generated-contract comparison, frontend
      tests/typecheck/build, pinned Rust portable/native checks, native regression
      scripts, Asterisk interoperability, browser flows and macOS packaging.
- [x] Record observed results and public-distribution blockers in a verification
      report. Review the final patch for unintended changes and data exposure.

Review focus: tag releases must not bypass branch quality checks; missing release
notes and missing/extra installers must fail; checksum filenames must work after
download; source files under `apps/desktop/build` must remain versionable; the
project SBOM must not be described as a complete product attestation.
