# Bug: Panic in interpreter when using multiple schemas without `RESOLVE_INCLUDES`

## Summary

When processing multiple XSD schemas that share a common schema via
`<xsd:import>`, `xsd-parser` 1.5.2 panics in the interpreter if
`ParserFlags::RESOLVE_INCLUDES` is **not** set.

The panic occurs in `crate_node_cache.rs:127` because the interpreter
unconditionally calls `.unwrap()` on a `dependencies.get()` lookup.
Without `RESOLVE_INCLUDES`, the `dependencies` map is empty, but the
schema content still contains `<xsd:import>` elements with
`schemaLocation` attributes.

## Reproduction

This example uses three XSD schemas:

- `common_types.xsd` — shared types (`HeaderT`, `StatusT`)
- `request_v1.xsd` — imports `common_types`, defines `Request` v1
- `request_v2.xsd` — imports `common_types`, defines `Request` v2 (adds
  `Priority` field)

All schemas are provided as explicit `Schema::File` inputs and the
config sets `ParserFlags::DEFAULT_NAMESPACES` (without
`RESOLVE_INCLUDES`).

```
cargo build
```

### Expected behavior

Code generation succeeds. Since all schemas are provided as explicit
inputs, the parser should have all type information available without
needing to resolve imports from disk.

### Actual behavior (xsd-parser 1.5.2)

```
thread 'main' panicked at xsd-parser-1.5.2/src/pipeline/interpreter/state/crate_node_cache.rs:127:73:
called `Option::unwrap()` on a `None` value
```

## Why `RESOLVE_INCLUDES` is not a viable workaround

`RESOLVE_INCLUDES` instructs the parser to fetch every
`xs:import` / `xs:include` from its declared `schemaLocation`. That
location is whatever the schema author put in the file and there is
no guarantee it points at anything reachable from the build environment.

The user needs to be in control of where every schema is sourced
from, which is exactly what providing all schemas as explicit
`Schema::File` (or `Schema::NamedSchema`) inputs is supposed to
deliver. That control is meaningless if the parser still panics
unless `RESOLVE_INCLUDES` is enabled.

## Root cause and proposed fix

`crate_node_cache.rs:127` (and the matching `Include` / `Override` /
`Redefine` arms a few lines below):

```rust
// Current code — panics when dependencies is empty:
let base = **info.dependencies.get(schema_location).unwrap();

// Fix — gracefully skip unresolved imports:
if let Some(base) = info.dependencies.get(schema_location) {
    self.process_schema(**base)?;
}
```

When schemas are provided as explicit inputs without
`RESOLVE_INCLUDES`, the dependency graph is empty, so the lookup
returns `None`. Skipping silently is correct here because every schema
the interpreter cares about is already in the input set — the import
reference is informational, not an instruction to fetch.

## Cross-schema type resolution

A second problem surfaces in `resolve_type_ident()`
(`crate_node_cache.rs:930`) once the panic is fixed:
`resolve_for_schema()` only searches the current schema and its direct
dependencies. When a type from an imported schema is referenced
(here: `ct:HeaderT` in the request schemas), it cannot be found
without falling back to the global `resolve()`:

```rust
// Current code — only searches current schema + dependencies:
let ident = self.ident_cache
    .resolve_for_schema(self.current_schema(), ident.clone())?;

// Fix — fall back to global resolution:
let ident = self.ident_cache
    .resolve_for_schema(self.current_schema(), ident.clone())
    .or_else(|_| self.ident_cache.resolve(ident))?;
```

## Context

This worked in xsd-parser 1.4.x because the interpreter did not have
the `create_node_cache()` step — type resolution was global by
default. The issue was introduced in 1.5.0 with the new
schema-scoped resolution architecture.
