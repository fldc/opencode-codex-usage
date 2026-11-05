# Codex Usage for OpenDeck

Status: initial Linux implementation completed, 2026-11-05. Live quota retrieval verified; OpenDeck installation and physical key appearance remain to be verified. See README.md for build and validation instructions.

## Goal and scope

Build an OpenDeck action that shows Codex subscription quota consumption and time until reset. Follow the reference plugin's circular ring design. Initial target: Linux, one locally signed-in Codex account, with multiple keys able to show different quota windows.

Confirmed v1 requirement: subscription quota usage is mandatory. Show authoritative account quota percentages and available reset times. API spend and per-session token totals are separate possible features, outside v1, and cannot substitute for subscription quota data.

## Research

- [Reference plugin](https://github.com/ImDevinC/opendeck-opencode-usage), inspected at main tree `f2095f9bc33cbc05a1b2f9c2c060571d1d9359b2`: Rust, OpenAction 2.1, Tokio, SVG key rendering, HTML property inspector, and plugin packaging. Its current HTTP client and settings are specific to OpenCode.
- [Codex app-server documentation](https://learn.chatgpt.com/docs/app-server): `account/rateLimits/read` supplies quota snapshots; `account/rateLimits/updated` supplies notifications. Windows expose `usedPercent`, `windowDurationMins`, and `resetsAt` in Unix seconds. Prefer `rateLimitsByLimitId` when available; retain compatibility with the single `rateLimits` snapshot. Primary and secondary windows can be absent.
- This workspace was empty. Installed CLI: `codex-cli 0.166.0`; its help confirms stdio app-server support. Live account access and both quota windows were subsequently verified with the Python diagnostic and Rust provider. OpenDeck wire-protocol integration is covered by a simulated-host smoke test; real application/device verification remains outstanding.
- The inspected upstream tree has no LICENSE file and GitHub reports no license. Before copying source/assets for distribution, resolve reuse permission; otherwise implement the behavior independently using appropriately licensed dependencies.

## Proposed user experience

- Plugin/category: **Codex Usage** / **Codex**; action: **Usage**.
- A key shows one quota window: ring for percentage used, compact window label, reset countdown, optional percentage text. Two keys can display short and long windows side by side.
- Match the reference colors: green below 50%, yellow from 50% to below 85%, red at 85% and above. Always describe the figure as usage; a full ring means quota consumed.
- Default to the primary window of the `codex` bucket. Offer secondary only when reported. Derive labels from actual duration (for example `5H` or `7D`), with generic labels if duration is unavailable. Do not invent a monthly quota or combine percentages from different windows.
- Pressing the key refreshes usage. It does not start an inference turn or consume a reset credit.
- Inspector: window selection, percentage toggle, connection status and last successful refresh. Include an advanced Codex executable path override for desktop environments with a different PATH. Select a bucket if the account reports several.
- Setup uses an existing Codex login. Missing CLI: `NO CLI`; missing login: `SIGN IN`; unavailable selected window: `N/A`; first fetch: `LOADING`. Provide explanatory inspector text.
- On temporary failure, retain the last value with a visible `STALE` indicator and its age in the inspector. With no cached value, show an error state. Missing data never becomes zero usage.

## Architecture

Use Rust + OpenAction with separate modules for the action, Codex client, quota normalization, refresh coordination, and SVG rendering.

The plugin owns one `codex app-server --listen stdio://` child shared by all keys. Perform the initialize/initialized handshake, check account state, then fetch quota snapshots. Let Codex manage its credentials; do not read auth files or store tokens in OpenDeck settings. Use the normal Codex environment and document any supported CODEX_HOME override.

Implement newline-delimited JSON message framing, request IDs, response routing, notifications, timeouts, bounded restart backoff, and child cleanup on plugin exit. Keep stderr separate from protocol stdout. Log sanitized status information rather than account payloads or credentials.

Normalize each snapshot into bucket ID, window ID, optional duration/reset time, percentage, and freshness timestamp. Ignore unknown fields. Escape dynamic SVG text and validate numeric values. Clamp ring geometry without disguising invalid data as healthy usage.

Poll once per 60 seconds while keys are visible, with one in-flight request shared across keys. Update countdowns locally every 30 seconds. Consume notifications when received, but keep polling: another Codex process may produce usage without this process receiving an event. Debounce repeated key presses. Refresh when a reset deadline passes; do not locally assume quota has returned to zero. Clear cached account data on account changes and re-check identity during refresh/reconnect.

## Implementation sequence

1. **Prove the data path.** Build a minimal bounded probe that initializes the installed app-server and calls account/read and account/rateLimits/read. Record sanitized response fixtures, verify normal login and unsupported authentication behavior, and compare the values with Codex's own status display. Establish the minimum tested CLI version. If quota access fails, resolve that before UI work; do not silently substitute session logs.
2. **Create the plugin foundation.** Set a unique plugin/action ID and crate name, add manifest and packaging, and establish the source reuse approach. Split responsibilities into modules rather than expanding a single main file.
3. **Implement the provider.** Add the managed child, protocol handling, normalization, shared cache, refresh coordinator, and recovery states. Verify no inference requests are made.
4. **Build the key and inspector.** Add dynamic windows, ring/countdown rendering, setup feedback, freshness indication, and settings updates. Changing the display window should reuse the cached snapshot.
5. **Validate and package.** Test parsing nullable/multiple buckets, out-of-order RPC responses, timeout/restart cleanup, shared polling, account changes, reset boundaries, SVG escaping, and stale/error transitions. Run formatting, tests, and clippy; manually check two keys, key presses, disconnect/reconnect, and OpenDeck restart. Produce a Linux installable bundle and setup documentation.
6. **Expand platforms later.** Add macOS/Windows builds only with verification of CLI discovery, process lifecycle, and installation on those systems.

## v1 acceptance criteria

- Release gate: live subscription quota retrieval and display must work end to end for a supported signed-in account. V1 is incomplete until this is verified; token counts or estimates do not satisfy this requirement.
- A signed-in user can add a key without pasting credentials.
- Displayed percentage and reset agree with the selected live Codex quota window.
- Multiple keys share the same provider and polling request.
- Missing windows, authentication failures, offline operation, and expired snapshots remain distinguishable from zero usage.
- Plugin shutdown leaves no child process running; reconnect recovers without restarting OpenDeck.
- The Linux package installs under a distinct identity and can coexist with the OpenCode plugin.

## Deferred

Combined dual-ring display, automatic selection of the most consumed window, remaining-percentage mode, token history, API spend, credit balances, multiple accounts, and embedded login UI. Keep v1 focused on quota visibility.

## Implementation notes

- Source and SVG assets were implemented independently; no reference-plugin code or artwork was copied.
- The initial build uses OpenAction 2.8.0 (locked), resolved from the compatible 2.x dependency.
- CLI path override is currently provided through CODEX_USAGE_CLI in OpenDeck's environment, rather than a per-key setting, keeping a single provider for all keys.
- Quota is polled every 60 seconds; local displays update every five seconds. Notifications are drained, while authoritative reads drive cache updates.
- Live provider verification satisfies the quota retrieval milestone. Comparison with Codex's interactive status display and end-to-end verification inside OpenDeck remain manual release checks.
