# Ulrichstern — sources Aidoku en Rust

[![Build and publish](https://github.com/Ulrichstern8888/aidoku/actions/workflows/build.yml/badge.svg)](https://github.com/Ulrichstern8888/aidoku/actions/workflows/build.yml)

Portage Rust pour Aidoku 0.7+ de cinq sources issues du projet Paperback. Le dépôt produit des paquets `.aix` installables et une liste de sources mise à jour automatiquement avec GitHub Actions et GitHub Pages.

> [!WARNING]
> Ces sources donnent accès à du contenu strictement réservé à un public majeur. Les sources externes sont indépendantes d'Aidoku et restent soumises à la disponibilité des sites concernés.

## Installation rapide dans Aidoku

Après la première publication GitHub Pages, ajoutez cette adresse dans **Aidoku → Réglages → Listes de sources** :

```text
https://ulrichstern8888.github.io/aidoku/
```

Activez ensuite **Réglages → Parcourir → Afficher les sources NSFW**, puis installez les sources depuis l'onglet **Parcourir**.

Les fichiers `.aix` peuvent aussi être téléchargés depuis l'artefact du dernier build ou depuis une release GitHub, puis ouverts avec Aidoku.

## Sources disponibles

| Source             | Langue | Fonctions principales                                      | Particularité                                   |
| :----------------- | :----: | :--------------------------------------------------------- | :---------------------------------------------- |
| HentaiOrigines     |   FR   | Recherche, accueil, listes, pagination, lecture            | Moteur Madara et cookie de validation adulte    |
| Hentai Scantrad VF |   FR   | Recherche, accueil, listes, pagination, lecture            | Challenge Cloudflare possible                   |
| ScansFR NSFW       |   FR   | Accueil progressif, recherche, filtres dynamiques, lecture | Jetons d'images signés et contrôle NSFW         |
| OrtegaScans        |   FR   | API paginée, genres dynamiques, listes, lecture            | Exclusion Premium et secours JSON des chapitres |
| FreeComics.XXX     |   EN   | Genres/artistes dynamiques, cinq rubriques, séries         | Rubriques parallèles et regroupement des livres |

## Fonctions du portage

- API Rust moderne `aidoku-rs` 0.3 et cible `wasm32-unknown-unknown`.
- Accueils Aidoku, listes paginées, recherche et filtres dynamiques.
- Affichage progressif des fiches et squelettes de chargement pour réduire l'attente perçue.
- Chargement parallèle des rubriques indépendantes OrtegaScans et FreeComics.
- Liens profonds vers les mangas et chapitres pris en charge.
- En-têtes `Referer`, cookie adulte et requêtes d'images spécifiques aux sites.
- Déduplication des mangas, chapitres et pages.
- Validation automatique de chaque paquet avec le CLI Aidoku.

## Cloudflare et Hentai Scantrad VF

Aidoku détecte certaines réponses Cloudflare 403/503. L'application ouvre une WebView, affiche le captcha lorsqu'une intervention est nécessaire, récupère le cookie `cf_clearance`, puis relance la requête.

Ce mécanisme dépend de la page renvoyée par Cloudflare. Un challenge non reconnu ou une règle de protection plus stricte peut encore bloquer temporairement la source.

## Compiler localement sous Windows

Prérequis :

- Rust stable ;
- la cible WebAssembly ;
- le CLI Aidoku.

```powershell
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/Aidoku/aidoku-rs --rev 1a6bb691dd67c7151fc76fc852fb5a364d325f72 aidoku-cli
cd D:\Downloads\aidoku-main
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo check --workspace
.\scripts\package.ps1
```

Le script crée :

- `packages/*.aix` pour l'installation manuelle ;
- `public/index.min.json` pour la liste Aidoku ;
- `public/sources/*.aix` et les icônes nécessaires à la publication.

## Publier le dépôt sur GitHub

Le dépôt public utilisé est `Ulrichstern8888/aidoku`. Pour envoyer les modifications :

```powershell
cd D:\Downloads\aidoku-main
git init
git add .
git commit -m "Initial Aidoku sources"
git branch -M main
git remote add origin https://github.com/Ulrichstern8888/aidoku.git
git push -u origin main
```

Le workflow `.github/workflows/build.yml` effectue automatiquement :

1. le contrôle du formatage ;
2. Clippy avec les avertissements interdits ;
3. la compilation WebAssembly des cinq sources ;
4. la création et la vérification des `.aix` ;
5. la génération de la liste Aidoku ;
6. l'envoi d'un artefact téléchargeable ;
7. la publication de `public/` sur la branche `gh-pages`.

Après le premier build réussi, ouvrez **GitHub → Settings → Pages** et sélectionnez :

- **Source** : `Deploy from a branch` ;
- **Branch** : `gh-pages` ;
- **Folder** : `/ (root)`.

L'adresse publique devient alors :

```text
https://ulrichstern8888.github.io/aidoku/
```

La forme explicite `https://ulrichstern8888.github.io/aidoku/index.min.json` fonctionne également.

## Publier une release

Un tag commençant par `v` crée automatiquement une release contenant les cinq `.aix` :

```powershell
git tag v2.0.0
git push origin v2.0.0
```

Pour qu'Aidoku propose une mise à jour, augmentez aussi le champ `info.version` dans le `res/source.json` de la source modifiée.

## Limites connues

- Les sites peuvent changer leurs routes, leur HTML ou leur API sans préavis.
- Les filtres dynamiques OrtegaScans et FreeComics nécessitent une requête lors de leur première ouverture.
- FreeComics ne fournit qu'une route serveur principale à la fois ; certaines combinaisons de facettes sont affinées localement et peuvent produire des pages moins remplies.
- Le fonctionnement exact des cookies, du cache et de la WebView Cloudflare doit être confirmé dans Aidoku sur iPhone ou iPad.
- Certains champs Paperback sans équivalent direct dans Aidoku, notamment quelques titres alternatifs, sont omis.

## Structure du dépôt

```text
aidoku-main/
├── .github/workflows/build.yml
├── scripts/package.ps1
├── sources/
│   ├── en.freecomicsxxx/
│   ├── fr.hentaiorigines/
│   ├── fr.hentaiscantradvf/
│   ├── fr.ortegascans/
│   └── fr.scansfrnsfw/
├── templates/madara/
├── Cargo.toml
├── Cargo.lock
└── LICENSE
```

Les dossiers `target/`, `packages/`, `public/` et les `package.aix` intermédiaires sont générés automatiquement et ne doivent pas être ajoutés au dépôt.

## Licence

Projet maintenu sous l'identité publique **Ulrichstern**. Le code dérivé de `paperback-main` est distribué sous GPL-3.0-or-later conformément au fichier `LICENSE`.
