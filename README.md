<div align="center">

<p>
  <img src="sources/fr.hentaiorigines/res/icon.png" width="72" alt="HentaiOrigines">
  &nbsp;
  <img src="sources/fr.hentaiscantradvf/res/icon.png" width="72" alt="Hentai Scantrad VF">
  &nbsp;
  <img src="sources/fr.scansfrnsfw/res/icon.png" width="72" alt="ScansFR NSFW">
  &nbsp;
  <img src="sources/fr.ortegascans/res/icon.png" width="72" alt="OrtegaScans">
  &nbsp;
  <img src="sources/en.freecomicsxxx/res/icon.png" width="72" alt="FreeComics.XXX">
</p>

# Ulrichstern Aidoku Sources

### Cinq sources adultes, un lecteur moderne, une installation en quelques secondes.

[![Build](https://github.com/Ulrichstern8888/aidoku/actions/workflows/build.yml/badge.svg)](https://github.com/Ulrichstern8888/aidoku/actions/workflows/build.yml)
[![Aidoku](https://img.shields.io/badge/Aidoku-0.7%2B-7c5cff?style=flat-square)](https://aidoku.app/)
[![Rust](https://img.shields.io/badge/Rust-WebAssembly-f74c00?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Sources](https://img.shields.io/badge/sources-5-22c55e?style=flat-square)](#-sources-disponibles)
[![License](https://img.shields.io/badge/licence-GPL--3.0-blue?style=flat-square)](LICENSE)

**[Installer la liste](https://ulrichstern8888.github.io/aidoku/)** · **[Découvrir Aidoku](https://aidoku.app/)** · **[Lire la documentation](docs/README.md)**

</div>

> [!WARNING]
> Ces sources donnent accès à du contenu strictement réservé à un public majeur. Les sites référencés sont indépendants d’Aidoku et peuvent modifier leur disponibilité ou leurs protections.

## ⚡ Installation express

1. Installez [Aidoku](https://aidoku.app/) sur iPhone ou iPad. Pour le sideload, utilisez la [source AltStore officielle d’Aidoku](https://raw.githubusercontent.com/Aidoku/Aidoku/altstore/apps.json).
2. Dans **Aidoku → Réglages → Listes de sources**, ajoutez :

   ```text
   https://ulrichstern8888.github.io/aidoku/
   ```

3. Ouvrez **Parcourir → Ulrichstern Aidoku Sources** et installez les sources souhaitées.

Besoin d’aide ? Consultez le [guide d’installation détaillé](docs/installation.md).

## ✨ Pourquoi cette collection ?

- **Lecture complète** : accueil, recherche, catégories, fiches, chapitres et pages.
- **Rapide et native** : sources Rust compilées en WebAssembly pour Aidoku.
- **Résistante aux changements** : URL relatives, images différées, cookies, jetons et réponses JSON pris en charge.
- **Vérifiée automatiquement** : formatage, Clippy, compilation et validation de chaque paquet `.aix` sur GitHub Actions.
- **Mise à jour simple** : les nouvelles versions apparaissent directement dans la liste Aidoku.

## 📚 Sources disponibles

| | Source | Langue | Points forts |
| :-: | :-- | :-: | :-- |
| <img src="sources/fr.hentaiorigines/res/icon.png" width="36" alt=""> | **HentaiOrigines** | 🇫🇷 | Accueil, tendances, filtres et lecteur Madara |
| <img src="sources/fr.hentaiscantradvf/res/icon.png" width="36" alt=""> | **Hentai Scantrad VF** | 🇫🇷 | Catalogue complet et session Cloudflare |
| <img src="sources/fr.scansfrnsfw/res/icon.png" width="36" alt=""> | **ScansFR NSFW** | 🇫🇷 | Filtres dynamiques et images signées |
| <img src="sources/fr.ortegascans/res/icon.png" width="36" alt=""> | **OrtegaScans** | 🇫🇷 | API paginée, genres et couvertures dédiées |
| <img src="sources/en.freecomicsxxx/res/icon.png" width="36" alt=""> | **FreeComics.XXX** | 🇬🇧 | Genres, artistes et regroupement des livres |

## 🔁 Vous utilisez Paperback ?

Une collection alternative est également disponible pour **Paperback**, application de lecture proposée sur l’App Store :

**→ [UlrichStern8888/paperback](https://github.com/UlrichStern8888/paperback)**

Les projets Aidoku et Paperback sont maintenus séparément afin de respecter les capacités et le format d’extension propres à chaque application.

## 🧭 Documentation

| Guide | Contenu |
| :-- | :-- |
| [Installation](docs/installation.md) | Aidoku, sideload, ajout de la liste et mises à jour |
| [Architecture](docs/architecture.md) | Organisation des sources, requêtes, lecteur et moteur Madara |
| [Développement](docs/development.md) | Prérequis, compilation, tests, paquets et publication |
| [Dépannage](docs/troubleshooting.md) | Cloudflare, lenteurs, images et limites connues |

<details>
<summary><strong>Informations pour les développeurs</strong></summary>

Le dépôt cible `wasm32-unknown-unknown` avec `aidoku-rs`. Les sources Madara partagent une bibliothèque interne documentée dans [`templates/madara`](templates/madara/README.md). Les cinq paquets sont compilés dans des cibles isolées afin de garantir que chaque `.aix` embarque le bon module WebAssembly.

</details>

## 🤝 Projet

Maintenu sous l’identité publique **Ulrichstern** et distribué sous licence [GPL-3.0](LICENSE).

Les sources externes, Aidoku et Paperback sont des projets indépendants. Ce dépôt ne contourne pas les captchas et ne fournit aucun contenu : il permet uniquement à Aidoku de lire les réponses des sites configurés.
