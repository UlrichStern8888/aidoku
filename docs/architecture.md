# Architecture

## Vue d’ensemble

Chaque dossier de `sources/` est une bibliothèque Rust `no_std` compilée en WebAssembly et emballée dans un paquet Aidoku `.aix`. Il contient :

- `src/lib.rs` : comportement de la source ;
- `res/source.json` : identité, version, langues, listes et limites réseau ;
- `res/icon.png` : icône 128 × 128 du paquet ;
- `Cargo.toml` : dépendances et type de bibliothèque.

OrtegaScans, ScansFR NSFW et FreeComics.XXX possèdent des adaptateurs dédiés. HentaiOrigines et Hentai Scantrad VF utilisent la bibliothèque Rust partagée [`templates/madara`](../templates/madara/README.md).

## Parcours d’une lecture

1. `get_search_manga_list` interroge le catalogue, applique les filtres et retourne une page de mangas.
2. `get_manga_update` complète la fiche et charge les chapitres uniquement lorsqu’Aidoku les demande.
3. `get_page_list` extrait, normalise et déduplique les URL d’images du chapitre.
4. `get_image_request` ajoute le `Referer`, les cookies et les en-têtes nécessaires au serveur d’images.
5. `PageImageProcessor` retente une requête ayant reçu une erreur HTTP.

Les URL relatives et les attributs d’images différées (`data-src`, `data-lazy-src`, `srcset`) sont convertis en URL absolues avant d’être transmis au lecteur.

## Moteur Madara

Le dossier `templates/madara` est une dépendance active, pas un exemple à supprimer. Il centralise :

- la recherche classique et AJAX ;
- les fiches et chapitres ;
- l’accueil et les filtres dynamiques ;
- l’extraction des pages ;
- les chapitres Manga Protector ;
- les liens profonds et migrations d’identifiants.

Une source Madara configure principalement ses sélecteurs dans `Params` et surcharge uniquement les comportements propres au domaine, comme le cookie adulte de HentaiOrigines.

## Performances

Les rubriques indépendantes de l’accueil Madara sont envoyées ensemble. Les sources limitent généralement Aidoku à quatre requêtes parallèles : cette valeur évite une attente entièrement séquentielle sans surcharger les sites externes.

La première ouverture d’un filtre dynamique ou d’une image très longue reste dépendante de la vitesse du serveur distant et de l’appareil.
