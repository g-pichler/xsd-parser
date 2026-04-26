# Bug: `GENERATE_PREFIXES` registers the generated prefix but doesn't assign it to the namespace info

## Summary

When two schemas declare the same prefix (`xmlns:rq`) for *different*
target namespaces, `ParserFlags::GENERATE_PREFIXES` is supposed to
synthesize a fresh prefix for the second namespace so that each one
can land in its own generated module.

The fallback in `parser/mod.rs::determine_prefixes`
(see `xsd-parser/src/pipeline/parser/mod.rs:820-843`) inserts the
generated prefix into `self.schemas.known_prefixes` but does **not**
write it back to `info.prefix`. Compare against the surrounding
`alternative_prefixes` branch (~line 813), which correctly does
`info.prefix = Some(...)`.

As a consequence, the second namespace ends up with
`info.prefix == None`, which propagates through
`prepare_modules.rs::prepare_modules` into a
`ModuleMeta { prefix: None, ... }`. The renderer then drops that
namespace's types into the root module, where they collide with the
first schema's `Request` definition.

## Reproduction

This example uses three XSD schemas:

- `request_v1.xsd` — `xmlns:rq="…/request/01"`, defines `Request`
- `request_v2.xsd` — `xmlns:rq="…/request/02"`, defines `Request`
- `request_v3.xsd` — `xmlns:rq="…/request/03"`, defines `Request`

All three request schemas pick the same prefix `rq` for distinct
target namespaces. Three schemas (rather than two) are needed to
turn the bug into a compile error: with the bug, schema v1 wins the
`rq` module, and schemas v2 and v3 both collapse into the root, where
their `Request` definitions collide. With two schemas the bug is
present but silent, because v2 alone in the root doesn't collide
with anything.

The build config sets
`DEFAULT_NAMESPACES | ALTERNATIVE_PREFIXES | GENERATE_PREFIXES`.

```
cargo build
```

### Expected behavior

`GENERATE_PREFIXES` invents a synthetic prefix (e.g. `rq_2`) for the
second namespace, both `Request` types end up in their own modules,
and the build succeeds.

### Actual behavior

The build fails with duplicate `Request` / `RequestElementType`
definitions in the root module — the second namespace lost its
prefix and dumped its types into the root.

## Root cause and proposed fix

`xsd-parser/src/pipeline/parser/mod.rs:820-843`:

```rust
// Fallback to generated prefix
if self.generate_prefixes {
    for (id, info) in &mut self.schemas.namespace_infos {
        if info.prefix.is_some() {
            continue;
        }

        let entry = &mut self.prefix_cache.get(&info.namespace).unwrap();
        let prefix = entry
            .prefix
            .clone()
            .or_else(|| entry.alt_prefixes.iter().next().cloned());
        if let Some(prefix) = prefix {
            let ext = format!("_{}", id.0);
            let ext = ext.as_bytes();

            let mut p = prefix.0.into_owned();
            p.extend_from_slice(ext);

            let prefix = NamespacePrefix(Cow::Owned(p));
            self.schemas.known_prefixes.insert(prefix, *id);
            // BUG: info.prefix is never updated here.
        }
    }
}
```

Fix: also assign back to `info.prefix`, mirroring the
`alternative_prefixes` branch a few lines above:

```rust
            let prefix = NamespacePrefix(Cow::Owned(p));
            info.prefix = Some(prefix.clone());
            self.schemas.known_prefixes.insert(prefix, *id);
```
