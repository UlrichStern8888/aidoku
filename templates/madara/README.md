# Moteur partagé Madara

Ce dossier est une **bibliothèque Rust active**, pas un exemple généré ni un dossier temporaire. Les sources `fr.hentaiorigines` et `fr.hentaiscantradvf` la déclarent comme dépendance locale dans leur `Cargo.toml` :

```toml
madara = { path = "../../templates/madara" }
```

Il faut donc la conserver. La supprimer casserait la compilation de ces deux sources. Son contenu est suffisant pour les sites Madara actuellement pris en charge ; il doit être modifié seulement lorsqu'un comportement commun à ces sites évolue.

## Répartition des responsabilités

- `src/lib.rs` expose `Madara<T>`, branche les traits Aidoku et contient les paramètres/sélecteurs par défaut.
- `src/imp.rs` contient les parcours réseau et HTML communs.
- `src/helpers.rs` construit les requêtes, résout les URL et extrait les attributs d'image paresseuse.
- `src/crypto.rs` déchiffre les chapitres protégés par Manga Protector.
- `src/models.rs` décrit uniquement les données JSON nécessaires à ces chapitres.
- `scripts/update_genres.py` est un outil de maintenance facultatif pour les sources qui stockent encore leurs genres dans un fichier JSON.

Chaque source fournit une structure qui implémente `Impl`. Sa méthode `params()` remplace les sélecteurs qui diffèrent du thème Madara standard. Une surcharge n'est utile que pour un comportement réellement propre au site, par exemple le cookie de validation adulte de HentaiOrigines.

## Méthodes principales

| Méthode | Rôle |
| :-- | :-- |
| `get_search_manga_list` | Construit la recherche classique ou AJAX, applique les filtres et transforme les cartes HTML en mangas. |
| `get_manga_update` | Charge la fiche, les métadonnées et, si demandé, les chapitres via la page ou l'endpoint AJAX. |
| `get_page_list` | Extrait les images d'un chapitre, qu'un sélecteur vise directement une image ou son conteneur, déduplique les URL et gère Manga Protector. |
| `get_image_request` | Réutilise `modify_request`, ajoute le `Referer` du chapitre et les en-têtes acceptés par les serveurs d'images. |
| `modify_request` | Point d'extension commun pour un cookie ou des en-têtes propres au site. |
| `get_home` | Envoie ensemble les requêtes indépendantes des rubriques afin de réduire le temps total de l'accueil. |
| `get_dynamic_filters` | Lit les genres du site et expose les filtres de tri, statut et type à Aidoku. |
| `handle_deep_link` / migrations | Convertit les URL externes et les anciens identifiants en clés Aidoku. |

Le `get_manga_list` par défaut de `Impl` est volontairement non implémenté : le nom et le tri des listes sont un choix de chaque source. HentaiOrigines et Hentai Scantrad VF fournissent toutes deux cette adaptation.

## Choisir où faire une modification

- Corriger `templates/madara` si le changement est valable pour toutes les variantes Madara : images paresseuses, URL relatives, chapitres AJAX, filtres ou protection.
- Corriger `sources/<id>/src/lib.rs` si le domaine exige un cookie, un sélecteur ou une règle propre.
- Ne pas copier une méthode entière dans une source pour une simple différence de sélecteur ; ajouter ou ajuster un champ de `Params`.

Après une modification, exécuter au minimum :

```sh
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo check --workspace
```

Les tests réseau de chaque source doivent aussi vérifier au moins une image décodable, car une liste de pages non vide ne garantit pas que le serveur d'images accepte la requête Aidoku.
