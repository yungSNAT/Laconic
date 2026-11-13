# CI tool dependencies

These private packages lock the tools used by the Claude Code and Pi load checks,
including their resolved dependencies and integrity hashes. Workflows use `npm ci
--ignore-scripts`. Claude's Linux binary is installed directly because its wrapper
normally exposes that binary through a postinstall script. This folder sits outside
`.github/` because Dependabot's npm updates could not open pull requests for the pins
there: every update from 31 October to 7 November 2026 got HTTP 400 at pull request
creation.

Update manifests and regenerate the corresponding lockfile on Linux with
`npm install --package-lock-only --ignore-scripts --prefix ci/claude-code`
or `npm install --package-lock-only --ignore-scripts --prefix ci/pi`.
Check `npm audit` and run the isolated load checks before changing the pins.

`requirements-coverage.txt` pins coverage for the Codecov upload in `tests.yml` by
version and SHA-256 hash, and the workflow installs it with `--require-hashes`. It
lists the Linux x86_64 wheels and the pure-Python wheel from the release's PyPI
files; Dependabot checks it weekly.

Pi 1.5.2 requires Undici 8.12.3 and minimatch 11.0.3, so it no longer needs the
overrides that excluded Undici 8.0.0 through 8.12.1 and minimatch before 11.0.1
(GHSA-g3hv-75wg-m249). The contract tests in `tools/laconic-tools` fail if the Pi
lockfile brings either range back.
