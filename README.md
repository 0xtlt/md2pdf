# md2pdf

[![CI](https://github.com/0xtlt/md2pdf/actions/workflows/ci.yml/badge.svg)](https://github.com/0xtlt/md2pdf/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)](https://www.rust-lang.org/)

Un convertisseur Markdown vers PDF rapide et autonome, écrit intégralement en
Rust. Il produit des documents nets avec pagination, images locales, tableaux,
liens cliquables et coloration syntaxique.

**Aucun Python, navigateur, LaTeX ou runtime externe n'est nécessaire.**

![Aperçu du PDF généré](docs/assets/preview.png)

## Démarrage rapide

```console
cargo build --release
./target/release/md2pdf example.md
```

Le PDF est créé à côté du fichier Markdown. Pour choisir son emplacement :

```console
md2pdf document.md --output build/document.pdf
```

## Fonctionnalités

- moteur PDF Typst embarqué et polices DejaVu intégrées ;
- coloration syntaxique sombre ou claire ;
- repli automatique des longues lignes de code ;
- pagination sûre des grands blocs de code ;
- liens PDF cliquables et images résolues relativement au Markdown ;
- formats A4 et Letter, portrait ou paysage ;
- métadonnées, en-tête, pied de page et couleur d'accent personnalisables ;
- lecture depuis un fichier ou l'entrée standard.

Les numéros de ligne sont **désactivés par défaut**. Ils n'apparaissent qu'avec
`--line-numbers`.

## Installation

### Depuis les sources

Rust stable est requis :

```console
git clone https://github.com/0xtlt/md2pdf.git
cd md2pdf
cargo install --path .
```

### Exécutable local optimisé

```console
cargo build --release
./target/release/md2pdf --version
```

L'exécutable final se trouve dans `target/release/md2pdf`.

## Utilisation

```text
md2pdf [OPTIONS] [SOURCE]
```

Exemples :

```console
# Réglages par défaut
md2pdf document.md

# Thème clair et couleur personnalisée
md2pdf document.md --code-theme light --accent '#2563EB'

# Letter paysage
md2pdf document.md --page-size letter --landscape

# Entrée standard
cat document.md | md2pdf - --output document.pdf

# Numéros de ligne explicitement demandés
md2pdf document.md --line-numbers
```

Options principales :

| Option | Valeur par défaut | Description |
| --- | --- | --- |
| `-o, --output PATH` | source avec extension `.pdf` | Chemin du PDF |
| `--title TEXT` | premier titre `#` | Métadonnée de titre |
| `--author TEXT` | vide | Métadonnée d'auteur |
| `--page-size a4\|letter` | `a4` | Format de page |
| `--landscape` | désactivé | Orientation paysage |
| `--margin MM` | `17` | Marges entre 8 et 45 mm |
| `--accent '#RRGGBB'` | `#C94C35` | Couleur des titres |
| `--code-theme dark\|light` | `dark` | Thème des blocs de code |
| `--line-numbers` | désactivé | Ajoute les numéros de ligne |
| `--no-header` | désactivé | Masque l'en-tête |
| `--page-break-before PREFIX` | aucun | Nouvelle page avant certains `##` |
| `-q, --quiet` | désactivé | Masque le message de succès |

Consultez `md2pdf --help` pour la liste complète.

## Markdown pris en charge

Titres, paragraphes, emphase, texte barré, liens, images locales, listes
imbriquées, cases à cocher, tableaux, citations, séparateurs, code en ligne et
blocs de code balisés sont pris en charge.

La [matrice Markdown détaillée](docs/markdown-support.md) documente les
comportements et limites connus.

## Architecture

Le pipeline reste volontairement simple :

```text
Markdown -> pulldown-cmark -> source Typst -> moteur Typst embarqué -> PDF
```

Les polices et le thème de coloration sont inclus dans l'exécutable. Consultez
la [documentation d'architecture](docs/architecture.md) pour les choix de
conception et les invariants de mise en page.

## Développement

```console
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
RUSTDOCFLAGS='-D warnings' cargo doc --no-deps
```

Les tests couvrent le parseur Markdown, l'échappement Typst, le repli et la
pagination du code, les images relatives, l'entrée standard, les erreurs CLI et
la production effective d'un PDF.

Voir [CONTRIBUTING.md](CONTRIBUTING.md) pour le workflow complet.

## Limites

- les images distantes ne sont pas téléchargées ;
- le HTML brut est rendu comme texte, pas interprété ;
- le repli des longues lignes de code est visuel et peut ajouter des lignes ;
- les polices embarquées privilégient la lisibilité et la couverture Unicode
  courante plutôt qu'un choix typographique configurable.

## Licence

Le code est distribué sous licence [MIT](LICENSE). Les polices DejaVu embarquées
conservent leur propre licence dans
[`assets/fonts/LICENSE_DEJAVU`](assets/fonts/LICENSE_DEJAVU).
