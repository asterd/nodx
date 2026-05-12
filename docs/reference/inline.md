# Inline reference

Inline constructs are the small markers you sprinkle inside paragraphs,
headings, list items, and cells. They never cross a line they did not
mean to cross: an unclosed delimiter is treated as text.

## Emphasis and decoration

| Source | AST `type` | HTML |
|---|---|---|
| `**bold**` | `strong` | `<strong>bold</strong>` |
| `*italic*` | `em` | `<em>italic</em>` |
| `` `code` `` | `code` | `<code>code</code>` |
| `==marked==` | `mark` | `<mark>marked</mark>` |
| `~sub~` | `sub` | `<sub>sub</sub>` |
| `^sup^` | `sup` | `<sup>sup</sup>` |
| `$$x+1$$` | `math-inline` | `<code class="math-inline">x+1</code>` |

The delimiter pairs must close on the same line in the same paragraph; an
unmatched marker is treated as a literal character. This is intentional:
NODX trades a few extra escapes for *predictable* tokenization.

## Links

```nodx
[label](target)
[label](target){title="…" rel="help"}
```

Parsed as `Inline::Link`:

```json
{
  "type": "link",
  "target": "https://example.com",
  "label": [ /* inlines */ ],
  "attrs": {"title": "…", "rel": "help"}
}
```

URL classification happens at render and validate time. A link with an
unsafe scheme raises `NODX-E020` and renders as a blocked `<a
class="nodx-blocked-link">`. Safe schemes today: `https`, `http`, `mailto`,
`tel`, package-relative paths (`./assets/foo.png`), in-document anchors
(`#section-id`).

`data:` URIs are allowed up to `limits.data_uri_bytes` (5 MiB default) and
only for image MIME types.

## Inline spans

```nodx
[[styled text]]{.pill bg="#eef"}
[[Approved]].status.success
```

Renders as `<span>`. Spans accept the full attribute syntax plus the
*class-suffix sugar* (`.foo.bar` immediately after `]]`). Use spans when
you need a small visual tag, a language switch (`{lang="ar"}`), or a
component-like inline annotation that does not warrant a full block.

`[[ ... ]]` is the preferred form. The legacy `[label]{...}` form is
still parsed but considered grandfathered — it conflicts with link
parsing and is less obvious to read.

## Variables

```nodx
{{name}}            ← shorthand for {{vars.name}}
{{vars.name}}       ← explicit
{{meta.title}}      ← read from front matter
{{attrs.title}}     ← read from the enclosing block's attributes
```

In the AST every variable carries an explicit `namespace`:

```json
{"type": "var", "namespace": "vars", "name": "reviewer"}
```

Variables that resolve to nothing render as a `<var>` fallback containing
the original name (`<var>missing</var>`). The validator emits `NODX-E013`
for variables referenced but never declared.

## References

| Source | AST `type` | Meaning |
|---|---|---|
| `@[section-id]` | `ref` | Cross-reference to a block by id. |
| `[^fn-1]` | `footnote-ref` | Pointer to a footnote definition. |
| `[@smith2024]` | `citation-ref` | Pointer to a `citation-entry`. |
| `@{user:alice}` | `mention` | Typed mention, free-form `kind:target`. |

References do not silently fail. The validator emits `NODX-E007` for any
reference that does not resolve, and `NODX-E020` for unsafe targets.

## Escaping

A backslash escapes the next significant character:

```nodx
\* not italic *
\[label not a link]
\#not-an-id
```

Escapable characters: ``` ` * [ ] ( ) { } # @ ~ ^ = : | ```. Any other
character after a backslash keeps both the backslash and the character.

## Composability

Nesting is allowed where it makes sense:

```nodx
**bold with *italic* inside**
[link with **bold**](target)
[[span with `code`]]{.note}
```

Code spans never re-enter the inline parser — `` `*literal*` `` is the
literal three characters. Math, code, and style block bodies are equally
opaque.

## Things you can't do

- Reach an attribute from inline syntax. There is no `**{lang="en"}bold**`
  form; promote to `[[bold]]{lang="en"}` instead.
- Cross block boundaries with an inline marker. An unclosed `**` becomes
  literal text at the end of its paragraph.
- Inject raw HTML via inline syntax. The parser does not recognize `<a
  href="…">` — it stays text.

These constraints are why two implementations can produce identical
canonical ASTs from the same input.
