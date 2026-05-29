# Configuration

For basic configuration instructions, see [this documentation](https://developers.openai.com/codex/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/codex/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/codex/config-reference).

## Discord Rich Presence

Interactive TUI sessions can publish Rich Presence to Discord when Codex is
running under WSL and Discord Desktop is running on Windows. Configure it under
`[tui.discord_presence]`:

```toml
[tui.discord_presence]
enabled = true
application_id = "..."
large_image = "codex_logo"
large_text = ""
```

## Desktop Pet Overlay

Interactive TUI sessions can show the Codex desktop pet overlay when Codex is
running under WSL on Windows. Configure it under `[tui.pets]`:

```toml
[tui.pets]
enabled = true
selected_pet = "codex"
```

Use `/pet` in the TUI to show or hide the overlay. The Windows helper must be
built on Windows before WSL-hosted Codex can launch it.

## Lifecycle hooks

Admins can set top-level `allow_managed_hooks_only = true` in
`requirements.toml` to ignore user, project, and session hook configs while
still allowing managed hooks from requirements and managed config layers. This
setting is only supported in `requirements.toml`; putting it in `config.toml`
does not enable managed-hooks-only mode.
