# Security policy

## Reporting a vulnerability

Report privately through GitHub's private vulnerability reporting:
<https://github.com/gointerop/frevo/security/advisories/new>.

Please do not open a public issue for a suspected vulnerability.

Include, as far as you can: the affected version or commit, the configuration
(`application.yaml` with secrets removed), a request sequence that reproduces
the problem, and the impact you observed.

## What to expect

| Stage | Target |
|---|---|
| Acknowledgement | 3 working days |
| Initial assessment | 10 working days |
| Fix or mitigation plan for a confirmed high or critical issue | 30 days |

We will credit reporters who want credit, and coordinate disclosure timing.

## Scope

In scope: the Frevo server and its crates, the published container image, the
Helm chart, and the release pipeline.

Out of scope: findings that require a configuration the documentation warns
against; vulnerabilities in PostgreSQL itself; denial of service achieved only
by setting configuration limits deliberately unbounded; and scanner output with
no demonstrated impact.

## Handling health data

Frevo stores clinical data. Please do not include real patient data in a report.
Reproduce with Synthea output or specification examples. If a report
necessarily involves production data, say so and we will arrange a private
channel rather than having it posted.

## Supported versions

Before 1.0, only the latest release and `main` receive fixes.
