# MCP Installer Matrix

PR-D3 uses host configuration files as global client registration points. The
registered command is always project-scoped through an absolute, canonical
`--project` argument. Project files are never modified by the installer.

| Client | macOS / Linux | Windows | Entry schema | Scope |
| --- | --- | --- | --- | --- |
| Gemini / Antigravity | `~/.gemini/antigravity-cli/mcp.json` | same under home | `mcpServers` | global registration, project command |
| Grok | `~/.grok/plugins/scientist-in-loop/.mcp.json` | same under home | `mcpServers` | global registration, project command |
| Claude Desktop | platform application-support/config directory | `%APPDATA%/Claude/claude_desktop_config.json` | `mcpServers` | global registration, project command |
| Cursor | `~/.cursor/mcp.json` | same under home | `mcpServers` | global registration, project command |
| OpenCode | `~/.config/opencode/opencode.json` | `%APPDATA%/opencode/opencode.json` | `mcp` local provider | global registration, project command |
| Custom | `--path` | `--path` | selected by adapter as `mcpServers`, except OpenCode | explicit path |

The paths and schemas follow the clients' user configuration conventions:

- Claude Desktop: <https://modelcontextprotocol.io/docs/develop/connect-local-servers>
- Cursor: <https://docs.cursor.com/context/model-context-protocol>
- Gemini CLI: <https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/configuration.md>
- OpenCode: <https://opencode.ai/docs/mcp-servers/>

Grok and Antigravity remain conservative adapters based on their existing
`.mcp.json` and `mcp.json` conventions; unknown schema changes fail closed.

No hook is claimed for any host in this release. `--hook` returns an explicit
unsupported error and cannot write a blocking or background process hook.
