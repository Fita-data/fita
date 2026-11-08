# Contributing to Fita

Thanks for helping. This file covers how changes get in. For how the code is organized and how to work on it, see [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Before you start

- Check [docs/ROADMAP.md](docs/ROADMAP.md) and the open issues. If what you want is not there, open an issue first so we can agree on the approach.
- Small pull requests get reviewed faster. One topic per pull request.

## Branches

Work on a short lived branch from `main` with a plain name, such as `cif-validation` or `fix-blocking-limit`. Rebase on `main` before asking for review.

## Commits

Keep them short and plain, written like a person explaining the change to a colleague.

- Subject line in the imperative, under 60 characters, no period at the end. For example `Add NIF check digit validation` or `Fix stable id on cluster split`.
- If the change needs context, leave a blank line and write two or three sentences about why. Skip the body when the subject says it all.
- No prefixes, tags, emojis or generated summaries. No trailers except `Signed-off-by`.
- One logical change per commit. Squash fixup commits before merging.

## Pull requests

The description answers three things in plain sentences.

1. What changes.
2. Why.
3. How you tested it, including benchmark numbers if matching changed.

A pull request merges when CI is green and a maintainer approves it. Only maintainers push to `main`.

## Developer Certificate of Origin

Sign off your commits (`git commit -s`) to certify that you wrote the change or have the right to submit it under the Apache 2.0 license. See https://developercertificate.org.

## Licensing of dependencies

Only permissive licenses (Apache-2.0, MIT, BSD, ISC, Zlib, Unicode). `cargo deny check` fails on anything else.

## Security

Do not open public issues for vulnerabilities. See [SECURITY.md](SECURITY.md).
