# Security Policy

## Supported versions

Only the latest released version of mdprint receives security fixes.

## Reporting a vulnerability

Please use GitHub's **private vulnerability reporting**:
[Security → Report a vulnerability](https://github.com/filip-cokesh/mdprint/security/advisories/new).
Do not open public issues for security problems.

You can expect an initial response within a week.

## Security model

mdprint is a **build tool for your own documents**. It intentionally trusts
its inputs:

- raw HTML in Markdown passes through to the output unchanged,
- image paths are resolved on the local filesystem (including absolute paths
  and `..`) and the files are copied next to the output,
- template packs are code-equivalent: their CSS and fonts are embedded into
  every generated page.

**Do not run mdprint on untrusted Markdown, configs or template packs**, and
treat the generated HTML of an untrusted document as untrusted content.
