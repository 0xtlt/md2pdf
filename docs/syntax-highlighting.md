# Syntax highlighting

## Guarantees

`md2pdf` uses production TextMate grammars. It does not approximate languages
with keyword lists or regular expressions maintained by this project.

The automated test suite and rendered fixture explicitly cover:

- `html` and `css`;
- `liquid` with embedded HTML;
- `javascript` and `typescript`;
- `json`, `yaml`, and `toml`;
- `python`, `rust`, `go`, `java`, and C/C++;
- shell scripts and SQL.

The wider Typst/two-face catalog also includes many additional application,
configuration, markup, and systems languages.

## Fenced blocks

Use the canonical language identifier after the opening fence:

````markdown
```html
<article class="product">Available</article>
```

```liquid
{% if product.available %}
  <h2>{{ product.title | escape }}</h2>
{% endif %}
```
````

Identifiers and common aliases are case-insensitive. A missing or unknown
identifier is rendered safely as plain text.

## Engines

The general catalog is Typst's Syntect/two-face integration running on the
native Oniguruma regex backend. Liquid is precompiled from the curated TextMate
grammar distributed by Shiki and loads only when a `liquid` or
`shopify-liquid` block is present. Its embedded dependencies preserve HTML,
CSS, JSON, and JavaScript scopes inside templates.

This split keeps normal PDFs compact while providing first-class Liquid
support that is absent from the upstream two-face catalog.
