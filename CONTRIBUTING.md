# Contributing

Start with [development setup](docs/development.md). Keep changes small, typed
and consistent with the existing packages. Read the code before changing it.

- Branch from `main`; submit a pull request with behavior, checks and limitations.
- Reuse existing DTOs, helpers and platform APIs before introducing dependencies.
- Keep frontend types generated from Rust; never hand-edit generated contracts.
- Document every IPC change, including validation, errors and enum meaning.
- Add behavior tests when introducing real features or fixing regressions.
  Configuration and generated scaffolding are checked with their actual tools.
- Run the checks in the README. Desktop changes need macOS and Windows CI builds.
- Never submit SIP credentials, personal contact lists, private keys or raw call
  traces with identifying data. Use synthetic fixtures in interoperability tests.
- Generated scaffold icons are temporary; retain their third-party attribution.

Contributions are under the project's MIT license. Native and other dependencies
retain their own licenses; review added modules and transitive dependencies.
