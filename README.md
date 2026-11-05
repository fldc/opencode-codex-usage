# Codex Usage for OpenDeck

A Linux OpenDeck plugin showing **Codex subscription quota usage** on a deck key. It uses the local Codex login and reads quota through Codex app-server. No API key is entered into OpenDeck, and the plugin never starts an inference turn.

Each key shows a usage ring, window duration, reset countdown, and optional percentage used. Add two keys to show both available windows. The labels come from the account response (for example, `5H` and `7D`). Green means under 50% used, yellow means 50–84%, and red means 85% or more.

## Requirements

- Linux x86_64 and OpenDeck with OpenAction plugin support.
- Codex CLI installed and signed in with a ChatGPT account (`codex login`). Tested with `codex-cli 0.166.0`; older versions have not been verified.
- Rust stable and `make` to build; `zip` to package.

API-key-only authentication does not provide this subscription quota view. The plugin reports `AUTH MODE` in that case.

## Build and install

```sh
make package
```

In OpenDeck, choose **Install from file** and select `dist/codex-usage-linux-x86_64.zip`. Reload plugins or restart OpenDeck if necessary. Add **Codex → Usage** to a key, then select a quota bucket and window in its inspector.

Alternatively, copy the generated `dist/se.fredrik.codexusage.sdPlugin` directory into the `plugins` directory opened through OpenDeck's **Open config directory** setting.

The desktop process must be able to find `codex`. If its PATH differs from your shell, set `CODEX_USAGE_CLI` to the absolute executable path in OpenDeck's launch environment. Normal Codex configuration and `CODEX_HOME` apply; the plugin does not load shell startup files or parse credential files.

## Refresh and connection states

All visible keys share one Codex process and quota snapshot. Usage is fetched once a minute; countdowns redraw every five seconds. Press a key to request a refresh (repeated requests are limited to one every two seconds). A reset deadline also triggers a refresh.

- `NO CLI`: executable unavailable; check PATH or `CODEX_USAGE_CLI`.
- `SIGN IN`: run `codex login` using the same environment as OpenDeck.
- `AUTH MODE`: use a supported subscription login.
- `N/A`: the selected bucket/window is not reported for this account.
- `STALE`: the last successful value remains visible after a failed refresh or an elapsed reset deadline. The inspector shows the last successful update and connection error.
- `TIMEOUT`, `DISCONNECTED`, `QUOTA ERROR`, or `PROTOCOL`: reconnect is retried with bounded backoff. The plugin logs no response bodies, identity, or credentials.

A missing value is never interpreted as zero. Switching accounts clears the cached quota before accepting data for the new identity. Snapshots are kept only in memory. Restarting the plugin discards them.

## Development and verification

```sh
make check
cargo run -- --probe
cargo build
python scripts/smoke.py
```

`--probe` exercises the same Rust provider used by the plugin, prints normalized quota windows, and exits after stopping its child. It does not print credentials or account identity. `scripts/probe.py` is a separate Python diagnostic for the protocol.

The smoke test requires Python's `websockets` package (`python -m pip install -r requirements-dev.txt`, preferably in a virtual environment). It launches the real plugin with a simulated OpenDeck host and a fixture Codex process. It checks shared polling, settings changes, missing windows, stale values, account changes, sign-in recovery, disconnect/timeout recovery, and child cleanup.

## CI and releases

GitHub Actions runs formatting, unit tests, Clippy, and the OpenDeck protocol smoke test on pushes to `master` and on pull requests. The smoke test uses the release binary and a fixture Codex process; CI needs no Codex login. Each successful run provides a Linux x86_64 ZIP and SHA-256 checksum under its workflow artifacts.

To publish a release, update the version in `Cargo.toml` and `assets/manifest.json`, refresh `Cargo.lock`, and commit the changes. Push a matching tag such as `v0.1.0`. The workflow validates the tag and package versions, then publishes the tested ZIP and checksum to GitHub Releases with generated release notes. Tags with a prerelease suffix create prereleases. Existing releases are not overwritten by reruns.

Actions are pinned to verified release commits. Dependabot checks action, Rust, and Python test dependency updates weekly.

## Attribution

Behavior inspired by [ImDevinC/opendeck-opencode-usage](https://github.com/ImDevinC/opendeck-opencode-usage). This implementation and its assets were written independently; no upstream source or artwork is bundled. Codex quota integration follows the [official app-server documentation](https://learn.chatgpt.com/docs/app-server). This is an independent plugin, not an OpenAI product.
