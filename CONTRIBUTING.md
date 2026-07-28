# Contribuer à md2pdf

Merci de contribuer au projet. Une modification doit rester ciblée, testée et
compatible avec le caractère autonome du binaire.

## Préparer l'environnement

Rust stable et Git sont requis :

```console
git clone https://github.com/0xtlt/md2pdf.git
cd md2pdf
cargo test --locked
```

## Workflow

1. Créez une branche courte depuis `main`.
2. Ajoutez ou adaptez les tests avant de modifier le comportement.
3. Formatez le code avec `cargo fmt`.
4. Exécutez la suite qualité complète.
5. Ouvrez une pull request décrivant le problème et la solution.

## Suite qualité

```console
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps
```

Pour une modification visuelle, générez également `example.pdf`, rendez toutes
ses pages en images avec Poppler et vérifiez l'absence de débordement, de
collision ou d'espacement incohérent.

## Style

- préférez les fonctions courtes et les noms explicites ;
- documentez toute API publique ;
- évitez les dépendances lorsqu'une petite implémentation sûre suffit ;
- conservez les messages CLI destinés aux utilisateurs en français ;
- n'ajoutez aucun runtime externe au chemin d'exécution.

## Tests

Un correctif doit inclure un test qui échoue avant la correction. Les tests
d'intégration doivent invoquer le véritable exécutable et utiliser un dossier
temporaire.
