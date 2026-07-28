# Guide de rendu md2pdf

Un PDF **net**, compact et reproductible, avec des [liens
cliquables](https://www.rust-lang.org/) et du `code inline`.

## Éléments Markdown

> Cet encadré attire l’attention sans interrompre la lecture.

1. Les listes ordonnées conservent leur séquence.
2. Les listes peuvent contenir des éléments techniques :
   - sous-élément avec *emphase* ;
   - second sous-élément.

| Fonction | État | Détail |
| --- | :---: | --- |
| Unicode | OK | Accents : éèàçù, symboles : € → ✓ |
| Tableaux | OK | En-tête répété si le tableau change de page |
| Code | OK | Coloration et repli visuel |

**Résultat attendu :** le document reste lisible à l’écran et à l’impression.

## Coloration syntaxique

```rust
#[derive(Debug)]
struct Message<'a> {
    recipient: &'a str,
    body: &'a str,
}

fn render(message: &Message<'_>) -> String {
    // Une longue ligne est repliée visuellement sans être tronquée dans le PDF.
    format!("Bonjour {} : {}", message.recipient, message.body)
}

fn main() {
    let message = Message { recipient: "Monde", body: "Le PDF est prêt." };
    println!("{}", render(&message));
}
```

```rust
let labels: Vec<_> = items
    .iter()
    .filter(|item| item.active)
    .map(|item| item.label.to_uppercase())
    .collect();
```

---

### Fin du document

La pagination, l’en-tête et le pied de page sont ajoutés automatiquement.
