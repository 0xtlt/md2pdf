# Architecture

## Vue d'ensemble

`md2pdf` utilise un pipeline sans processus externe :

```text
Markdown
  -> événements pulldown-cmark
  -> document Typst en mémoire
  -> mise en page Typst
  -> sérialisation PDF
```

Le binaire embarque les polices DejaVu et le thème sombre. La machine cible n'a
donc besoin ni de Python, ni de navigateur, ni de LaTeX, ni de l'exécutable
Typst.

## Modules

| Module | Responsabilité |
| --- | --- |
| `cli` | Déclaration des arguments et valeurs autorisées |
| `markdown` | Conversion des événements Markdown en source Typst |
| `pdf` | Compilation Typst, résolution des fichiers et écriture PDF |
| `error` | Erreurs structurées et messages destinés à la CLI |
| `main` | Validation, lecture de l'entrée et orchestration |

## Conversion Markdown

Le parseur `pulldown-cmark` produit une suite d'événements. Le convertisseur
maintient uniquement l'état nécessaire au contexte courant : paragraphe, titre,
bloc de code, image, liste et tableau.

Le texte utilisateur n'est jamais injecté directement dans la syntaxe Typst.
Les guillemets, barres obliques inverses, retours à la ligne et tabulations sont
échappés avant génération.

## Mise en page du code

Typst ne replie pas automatiquement les blocs `raw`. Le convertisseur :

1. calcule la largeur disponible selon la page et les marges ;
2. replie les lignes trop longues sur une frontière Unicode sûre ;
3. découpe les très grands blocs en fragments tenant sur une page ;
4. conserve une faible séparation entre fragments du même bloc ;
5. applique un espacement plus large entre deux blocs Markdown distincts.

Ce découpage évite le texte rogné et maintient les en-têtes et pieds de page sur
les documents multipages.

## Ressources

Les chemins d'images sont résolus relativement au dossier du fichier Markdown.
Avec stdin, ils sont résolus depuis le dossier courant. Les ressources réseau
ne sont pas récupérées.

## Invariants

- les numéros de ligne sont désactivés par défaut ;
- aucune ligne de code ne doit dépasser la zone imprimable ;
- les fragments de code sont indivisibles à l'intérieur d'une page ;
- les dossiers parents du PDF sont créés automatiquement ;
- une erreur produit le code de sortie `2` et aucun faux succès.
