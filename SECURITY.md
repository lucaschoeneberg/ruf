# Security reporting

## Project status

This is a development preview with no supported production release yet. Build
success does not establish telephone, audio or security qualification. Reports
about existing code are welcome; there is no guaranteed response time, fix date
or long-term support policy at this stage.

## Private reports

If private vulnerability reporting is enabled on GitHub, use
**Security → Report a vulnerability**. Otherwise use a verified private contact
published by a maintainer. No security inbox is configured in this repository;
do not infer one from project names or contributor usernames.

If neither route is available, open a minimal issue requesting a private security
contact without exploit details, credentials or affected personal data. Ordinary
issue forms are public; do not post sensitive traces or a proof of concept there
while arranging a private channel.

Privately include affected commit/version, platform, reproduction with synthetic
data, expected/actual behavior and observed impact. Remove SIP passwords, tokens,
real numbers, contact lists and call histories. Maintainers review reports to agree
on a fix and coordinated disclosure when possible.

When connecting the repository, maintainers should enable private reporting and
publish a maintained fallback contact before inviting external security reports.
This file alone does not activate the feature.
[GitHub setup instructions](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository).
