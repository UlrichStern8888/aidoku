# Développement

## Prérequis

- Rust stable ;
- la cible `wasm32-unknown-unknown` ;
- le CLI Aidoku installé depuis la révision utilisée par le workflow.

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/Aidoku/aidoku-rs \
  --rev 1a6bb691dd67c7151fc76fc852fb5a364d325f72 aidoku-cli
```

## Contrôles locaux

```sh
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo check --workspace
```

Les tests réseau reposent sur `aidoku-test-runner`. Ils doivent vérifier une image réellement téléchargeable et décodable, car une URL de page non vide ne garantit pas que le serveur accepte les en-têtes Aidoku.

## Créer les paquets sous Windows

Depuis PowerShell :

```powershell
cd D:\Downloads\aidoku-main
.\scripts\package.ps1
```

Le script génère :

- `packages/*.aix` pour l’installation manuelle ;
- `public/index.json` et `public/index.min.json` pour la liste ;
- `public/sources/*.aix` et les icônes destinées à GitHub Pages.

## Isolation des modules WebAssembly

Le CLI Aidoku peut sélectionner le premier WASM trouvé dans une cible Cargo partagée. Le workflow construit donc chaque source dans son propre dossier `target`, puis compare le hash du WASM compilé à celui inclus dans le `.aix`.

## Publication

Le workflow [`.github/workflows/build.yml`](../.github/workflows/build.yml) :

1. vérifie le formatage, Clippy et la compilation ;
2. construit et valide les cinq paquets ;
3. génère la liste Aidoku ;
4. publie `public/` avec GitHub Pages ;
5. joint les `.aix` à toute release créée par un tag commençant par `v`.

Pour qu’Aidoku détecte une nouvelle version, augmentez `info.version` dans le fichier `res/source.json` de chaque source modifiée.

Les dossiers `target/`, `packages/`, `public/` et les `package.aix` intermédiaires sont générés et ne doivent pas être commités.
