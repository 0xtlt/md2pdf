# Couverture Markdown

| Élément | Support | Notes |
| --- | :---: | --- |
| Titres `#` à `######` | Oui | Le premier `#` devient le titre PDF |
| Paragraphes | Oui | Espacement vertical typographique |
| Gras et italique | Oui | Combinaisons prises en charge |
| Texte barré | Oui | Extension CommonMark activée |
| Liens | Oui | Annotations PDF cliquables |
| Images locales | Oui | Chemins relatifs à la source |
| Images distantes | Non | Aucun téléchargement réseau |
| Listes ordonnées | Oui | Imbrication prise en charge |
| Listes non ordonnées | Oui | Imbrication prise en charge |
| Cases à cocher | Oui | Symboles Unicode |
| Tableaux | Oui | En-tête sombre et lignes alternées |
| Citations | Oui | Encadré avec barre d'accent |
| Séparateurs | Oui | Ligne horizontale |
| Code en ligne | Oui | Police monospace |
| Blocs de code | Oui | Langage lu après les trois accents graves |
| HTML brut | Partiel | Affiché comme texte |
| Notes de bas de page | Partiel | Référence en exposant uniquement |
| Mathématiques | Partiel | Affichées comme contenu brut |

## Blocs de code

Les identifiants de langage reconnus par Typst bénéficient d'une coloration
syntaxique. Les longues lignes sont repliées visuellement. L'option
`--line-numbers` numérote les lignes visuelles ; elle est désactivée par défaut.

## Images

Les formats pris en charge dépendent du moteur d'image de Typst, notamment PNG,
JPEG et SVG. Une image seule dans un paragraphe devient un bloc centré. Une
image au milieu d'un texte est dimensionnée sur la hauteur de la ligne.

## Encadré de résultat

Un paragraphe commençant par `Résultat attendu :`, `Résultat attendu:` ou
`Expected result:` reçoit automatiquement un style de validation vert.
