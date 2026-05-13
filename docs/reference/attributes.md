# Attribute reference

The attribute block is the one syntactic primitive that touches almost
every NODX construct. It is also the one place where authors can make a
document fail to parse, so it is worth understanding precisely.

## The grammar

```ebnf
attr-block      = "{" attr-list "}"
attr-list       = attr ("space" attr)*
attr            = id-shorthand | class-shorthand | named-attr | bare-flag
id-shorthand    = "#" name
class-shorthand = "." name
named-attr      = name "=" value
bare-flag       = name                    ; today only "highlight" is a bare flag
value           = quoted-string | bare-token
quoted-string   = '"' (escaped-char | ?any except '"'?)* '"'
bare-token      = ?any non-whitespace non-quote char?+
name            = [A-Za-z] [A-Za-z0-9-]*
```

- `#id` may appear at most once per block.
- `.class` may appear multiple times; classes are merged, sorted, deduped.
- `name=value` values that contain whitespace must be quoted.
- Unknown attributes are not an error; they pass through to the AST and
  the renderer.

## Reserved attribute keys

These names have meaning across the system. Authoring tools should not
overload them.

| Key | Where it applies | Behavior |
|---|---|---|
| `id` | any block | Stable anchor. Validator enforces uniqueness and shape. |
| `class` | any block | Space-separated extra classes. |
| `lang` | any block | BCP-47 language tag. Passed through to HTML. |
| `dir` | any block | `ltr`, `rtl`, or `auto`. |
| `title` | links, blocks | Accessible name / tooltip. |
| `role` | any block | ARIA role. Renderer keeps as-is. |
| `level` | `heading` | 1–6. Set automatically by `#`-count, do not author manually. |
| `kind` | `list` | `unordered`, `ordered`, `task`. |
| `checked` | `item` (in task list) | `true` or `false`. |
| `scope` | `cell` | `col` or `row`. Set automatically on header cells. |
| `header` | `cell` | `true` for header cells. |
| `caption` | `table` | Visible table caption. HTML renderers also accept `title` as a caption alias. |
| `background` | `page`, front matter `page` | Package-local background image shortcut. Uses normal asset URL policy. |
| `colspan` | `cell` | Positive integer column span. Counts toward table grid validation. |
| `rowspan` | `cell` | Positive integer row span. Preserved for renderers. |
| `align` | table/cell | `left`, `center`, `right`, `start`, `end` for structural table alignment. Use `text-align` for a CSS style shorthand. |
| `valign` | `cell` | `top`, `middle`, `bottom`, or `baseline`. |
| `type` | `note` | `info`, `warning`, `danger`, `success`, `note`. |
| `src` | `image`, `media`, `embed` | Source URL or package-relative path. |
| `alt` | `image` | Required. Validator emits `NODX-E009` if missing. |
| `cite` | `quote` | Source URL. |
| `depth` | `toc` | Maximum heading depth to include. Default `2`. |
| `target` | links | Required keyword for browser link target (currently only `_blank`). |
| `rel` | links | Renderer hardens external links by adding `noopener noreferrer`. |
| `data-*` | any block | Pass-through. Useful for downstream tools. |

## Safe style shorthands

Several attribute keys are translated into inline CSS at parse time. The
shorthand is convenient for authors, and the parser refuses values that
look like CSS breakouts:

| Shorthand | CSS property |
|---|---|
| `bg` | `background-color` |
| `color` | `color` |
| `border` | `border` |
| `radius` | `border-radius` |
| `pad`, `padding` | `padding` |
| `m`, `margin` | `margin` |
| `gap` | `gap` |
| `width` | `width` |
| `height` | `height` |
| `display` | `display` |
| `columns` | `grid-template-columns` |
| `text-align` | `text-align` |
| `font` | `font` |
| `weight` | `font-weight` |
| `background-color` | (passthrough) |
| `border-radius` | (passthrough) |
| `font-weight` | (passthrough) |
| `grid-template-columns` | (passthrough) |

The bare flag `highlight` expands to a default-styled highlight (rounded
corners, accent color background). Values that contain `<`, `>`, `{`, `}`,
`;`, exceed 240 bytes, or look like `expression(`, `javascript:`,
`vbscript:`, `@import`, `url(` are silently dropped — they do not enter
the AST.

## Attribute value caps

The parser enforces `limits.attribute_value_bytes` (default 64 KiB) on
every attribute value. Values exceeding the cap are dropped from the AST
before the attribute is recorded. This is hostile-input defense: a
runaway attribute should not be able to amplify a small input into a huge
AST.

Authors will not hit this cap in any realistic document.

## Class-suffix sugar (inline spans only)

After a `]]` span close, classes may be chained without braces:

```nodx
[[Approved]].status.success
```

Equivalent to:

```nodx
[[Approved]]{.status .success}
```

This works only for spans. For block opens, write the full `{…}` form.

## How attributes flow through the system

1. **Parser** produces the canonical map: id, sorted classes, attrs,
   styles. Validates basic shape (`id` matches `[A-Za-z][A-Za-z0-9-]*`,
   classes are valid identifiers, …).
2. **Validator** enforces semantic rules: `id` uniqueness, required
   attributes (`alt` on `image`), shape of `type` enums.
3. **Renderer** escapes each value into its HTML context: text for text,
   attribute for attribute, URL for URL, style for style. There is no
   way for an attribute value to escape its context.

If you need a deeper look, the parsing code lives in
[`crates/nodx-core/src/attrs.rs`](../../crates/nodx-core/src/attrs.rs)
(~200 lines, no surprises).
