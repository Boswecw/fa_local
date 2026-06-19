# FA Local

FA Local is the app-support governed local execution boundary for Forge
applications.

It is an implementation repo for the FA Local service. Shared local-runtime
doctrine and vocabulary remain owned by `forge-local-runtime`.

## Status

Ticket 1 scaffold is present. The Rust crate builds, exposes typed baseline
vocabulary, and defaults toward fail-closed admission. Contract schemas,
artifact loaders, and execution coordination are intentionally not complete.

## Documentation

- `docs/fa-local_architecture_spec.md`
- `docs/fa-local_extended_roadmap.md`
- `docs/fa_local_codex_build_plan_v_1.md`

## Code Mirror

`doc/system/` is the canonical code mirror. Rebuild the assembled mirror with:

```bash
bash doc/system/BUILD.sh
```
