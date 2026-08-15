# Stage 15 migration

Template packs are now described by `template.yaml`, pinned by
`.sil/template.lock`, and installed into the project-local immutable cache.
The old `standard`, `article`, `cvpr`, and legacy venue names remain accepted
by `sil paper template apply` for existing workflows; they do not install or
stage a package and do not identify official/current venue files. New
workflows should use `sil paper template install <directory> --approve`, then
`verify`, `stage`, and `remove` by manifest id. Updates are explicit and must
pass the same manifest, digest, and license checks.

The bundled standard fixture is an MIT-licensed structural example only. No
official venue files are bundled or downloaded by this implementation.

## Existing skills

Existing `agent/skills/*.md` files are not silently overwritten. The registry
stores managed projections under `agent/skills/managed/`, preserves changed
legacy files under `agent/skills/local/` during migration, and records package
selection in `.sil/skills.lock`. Future updates require an explicit diff and
approval; local skills remain user-owned.

## Existing digest data

The legacy global `journal_digest` table remains readable as derived local
memory. It is not a query-independent "top journals" claim. Stage 15
discovery runs add query-scoped provider requests, immutable provider records,
canonical works, venues, candidates, and append-only candidate events. A
rebuildable SQLite database is not the portable provenance contract; export
and tracked lock/config files are.

## MCP configuration

Run `sil mcp install --project <path>` for installed clients. The installer
embeds the canonical project root, preserves unknown configuration, backs up
before atomic writes, and supports `status` and `uninstall`. Direct
`sil project mcp` invocation may discover the root from CWD for interactive
compatibility, but reports that fallback; installed configurations do not
depend on CWD.
