# Contributing

Code, documentation, bug reports, interoperability testing and reviews are welcome.
Read the [Code of Conduct](CODE_OF_CONDUCT.md) and
[development setup](docs/development.md). Check the [README](README.md) before
reporting a planned feature as a regression: this app is currently a development
preview. The [release checklist](docs/release-checklist.md) records distribution gaps.

## Before starting

Search existing issues and PRs. Small fixes can go straight to a PR. Discuss larger
features, architecture changes, new dependencies and supported-platform changes
in an issue first so the work fits the MVP roadmap. Acceptance of an idea is not
a release-date commitment or an assignment to implement it.

Fork the repository and branch from `main`, using a name such as
`fix/account-status` or `docs/windows-setup`. Keep one concern per PR; unrelated
refactoring belongs in a separate PR. Draft PRs are welcome for early feedback.

## Issues

Use the issue forms where applicable. Bug reports should include app version/commit,
OS version and architecture, minimal reproduction steps, expected/actual behavior
and checks actually performed. For telephone issues, include PBX/provider product
and version, transport, codec and network scenario when known.

Feature requests should explain a user's problem and concrete use case before
proposing an implementation. General questions can use a regular issue. Maintainers
may request a smaller reproduction or close duplicates and out-of-scope requests
with an explanation. There is no guaranteed review or support response time.

Ordinary issues are public. Never upload SIP passwords, tokens, personal contacts,
real call histories or unredacted SIP/SDP captures. Redact numbers, names, Call-IDs,
addresses and credentials. Use synthetic data. Follow [SECURITY.md](SECURITY.md)
for security-sensitive reports.

## Code and documentation

- Read the affected code; follow existing structure, names, DTOs and utilities.
- Keep core logic independent of Tauri. Native C/unsafe work belongs in
  `baresip-adapter`; audio stays out of the frontend.
- Keep Rust and TypeScript strictly typed. Generate frontend IPC types from Rust;
  never hand-edit generated contracts.
- Document changed IPC fields, validation, error cases and semantics in `docs/api`,
  and public Rust APIs with documentation comments. Call out breaking changes.
- Add meaningful behavior tests for features and regressions. Check configuration
  and generated scaffolding with their actual tools rather than mirror tests.
- Explain dependency additions and intentional lockfile changes. Check licenses
  and preserve upstream attribution for copied code and assets; see
  [licensing](docs/licensing.md).

## Checks, PRs and review

Use the commands in [development setup](docs/development.md), selecting checks
appropriate to the change. Documentation-only PRs need formatting and link checks;
runtime changes need relevant typecheck/build/tests. Desktop implementation changes
also need macOS and Windows CI builds. Disclose unperformed checks and unavailable
hardware/platform tests.

Explain the problem, resulting behavior, related issue, verification results and
limitations in the PR description. Complete its checklist honestly. Prefer clear
titles such as `fix: preserve the selected account` or `docs: explain Windows setup`.
Working commits can be iterative; maintainers may squash on merge. Rebase when
requested or when conflicts block review. Cryptographic signatures and DCO
sign-offs are welcome but not required.

Maintainers review correctness, simplicity, architecture, security and compatibility.
Address feedback or explain a technical disagreement. Merging requires maintainer
review and passing applicable checks. GitHub repository rules must be enabled to
enforce this automatically; documentation alone does not configure them.

## AI-assisted contributions

You remain responsible for everything submitted. Understand and review generated
code/text, verify APIs and sources, and run the checks you claim. Mention substantial
assistance when it helps review. Do not submit fabricated results, invented
citations, credentials or code with unclear provenance or redistribution rights.

## Contribution license

By intentionally submitting an original contribution for inclusion, you agree to
license it under the project's [MIT license](LICENSE). You retain your copyright;
no assignment, separate contributor license agreement (CLA), mandatory sign-off
or cryptographic signature is required. Submit only work you have the right to
contribute, including any necessary employer permission.

Clearly identify third-party material and its original license. It retains those
terms and must be reviewed before inclusion. Do not add extra restrictions to an
otherwise MIT contribution. See [licensing and attribution](docs/licensing.md).

## References

These original, project-specific rules were informed by the
[Tauri guide](https://github.com/tauri-apps/tauri/blob/dev/.github/CONTRIBUTING.md),
[Tokio guidance](https://github.com/tokio-rs/tokio/blob/master/CONTRIBUTING.md) and
[Tokio's MIT contribution model](https://github.com/tokio-rs/tokio/blob/master/tokio/README.md#contribution).
Their project-specific signing requirements, release processes and support
promises are not adopted here.
