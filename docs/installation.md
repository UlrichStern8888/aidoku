# Installation

## Installer Aidoku

Aidoku est disponible depuis son [site officiel](https://aidoku.app/). Pour une installation par sideload, ajoutez sa [source AltStore officielle](https://raw.githubusercontent.com/Aidoku/Aidoku/altstore/apps.json) dans AltStore, puis installez Aidoku depuis cette source.

## Ajouter la collection Ulrichstern

Dans **Aidoku → Réglages → Listes de sources**, ajoutez l’adresse suivante :

```text
https://ulrichstern8888.github.io/aidoku/
```

La forme explicite suivante fonctionne également :

```text
https://ulrichstern8888.github.io/aidoku/index.min.json
```

## Afficher les sources NSFW

Aidoku peut masquer les sources adultes selon la classification actuellement sélectionnée. Pour afficher toutes les sources NSFW de la collection :

1. ouvrez **Parcourir** ;
2. touchez le bouton **+** ;
3. ouvrez le **menu à trois traits en haut à droite** ;
4. sélectionnez **Classification du contenu** ;
5. choisissez **Principalement du contenu restreint**.

Sélectionnez ensuite **Ulrichstern Aidoku Sources** et installez les sources souhaitées.

## Installer un paquet manuellement

Chaque build GitHub Actions produit des paquets `.aix`. Téléchargez l’artefact du dernier build ou les fichiers joints à une release, puis ouvrez le paquet souhaité avec Aidoku.

## Recevoir les mises à jour

Aidoku compare la version installée au champ `info.version` publié dans la liste. Une mise à jour apparaît automatiquement lorsqu’une nouvelle version de la source est déployée sur GitHub Pages.

> [!IMPORTANT]
> Le contenu est réservé aux adultes. Selon la version d’Aidoku, aucun interrupteur NSFW séparé n’est nécessairement affiché.
