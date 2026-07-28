# Syntax Highlighting Catalog

Production TextMate grammars highlight standalone languages and embedded
languages inside Liquid templates.

## HTML

```html
<article class="product-card" data-active="true">
  <h2>Clean markup</h2>
</article>
```

## CSS

```css
.product-card {
  display: grid;
  color: #0969da;
}
```

## Liquid with embedded HTML

```liquid
{% if product.available %}
  <article class="product-card">
    <h2>{{ product.title | escape }}</h2>
    <span>{{ product.price | money }}</span>
  </article>
{% endif %}
```

## JavaScript

```javascript
const activeProducts = products.filter((product) => product.available);
```

## Python

```python
def render_product(name: str) -> str:
    return f"Product: {name}"
```

## Rust

```rust
fn render_product(name: &str) -> String {
    format!("Product: {name}")
}
```
