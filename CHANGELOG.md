# Changelog

Toutes les modifications notables sont documentées dans ce fichier. Le format
suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/).

## [Unreleased]

### Added

- documentation de l'architecture et de la couverture Markdown ;
- workflow CI pour le formatage, Clippy, les tests et la documentation ;
- tests d'intégration pour stdin, les images relatives et les erreurs CLI.

### Changed

- documentation Rust des API publiques ;
- rythme vertical plus aéré entre les éléments de bloc ;
- nettoyage des constantes et états internes du convertisseur Markdown.

## [3.0.0] - 2026-07-28

### Added

- conversion Markdown vers PDF entièrement en Rust ;
- moteur Typst, polices DejaVu et thème sombre embarqués ;
- tableaux, listes, images, liens, citations et coloration syntaxique ;
- pagination des grands blocs et repli des longues lignes ;
- formats A4 et Letter, métadonnées et personnalisation visuelle ;
- exécutable CLI avec entrée fichier ou stdin.

[Unreleased]: https://github.com/0xtlt/md2pdf/compare/v3.0.0...HEAD
[3.0.0]: https://github.com/0xtlt/md2pdf/tree/v3.0.0
