# Mise à jour facultative des genres

`update_genres.py` récupère les genres d'un site Madara et met à jour le fichier `filters.json` indiqué :

```sh
python update_genres.py /chemin/vers/filters.json https://domaine.example
```

Le script n'est nécessaire que pour une source qui conserve une liste statique de genres. Les sources de ce dépôt utilisent actuellement des filtres dynamiques et n'en ont donc pas besoin pour leur fonctionnement normal.

Pour l'architecture et les règles de modification du moteur partagé, voir [`../README.md`](../README.md).
