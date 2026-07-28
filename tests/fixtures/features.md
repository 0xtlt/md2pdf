# Validation complète

![Bannière de test](test.svg)

## Markdown

- [x] tâche terminée ;
- [ ] tâche restante ;
- lien [Rust](https://www.rust-lang.org/) ;
- texte ~~barré~~, **gras**, *italique* et `inline`.

| Option | Valeur |
| --- | --- |
| Format | A4 |
| Unicode | éèàçù € → ✓ |

> Un encadré qui doit rester lisible même lorsqu’il contient plusieurs mots et
> passe éventuellement sur une nouvelle ligne.

## Longue ligne de code

```rust
let valeur_extremement_longue = "abcdefghijklmnopqrstuvwxyz_abcdefghijklmnopqrstuvwxyz_abcdefghijklmnopqrstuvwxyz_abcdefghijklmnopqrstuvwxyz_abcdefghijklmnopqrstuvwxyz_abcdefghijklmnopqrstuvwxyz_abcdefghijklmnopqrstuvwxyz_abcdefghijklmnopqrstuvwxyz";
```

**Expected result:** image, lien, tableau, Unicode et code sont présents.
