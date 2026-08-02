# Dépannage et limites connues

## Hentai Scantrad VF et Cloudflare

Aidoku peut ouvrir une WebView lorsqu’une réponse Cloudflare exige une vérification humaine. La source ne contourne pas ce captcha.

Après validation :

1. fermez la WebView ;
2. actualisez ou rouvrez la source pour rejouer la requête avec le cookie `cf_clearance` ;
3. redémarrez Aidoku uniquement si la requête ne repart toujours pas.

Le cookie peut persister, ce qui explique que la source fonctionne parfois immédiatement après un redémarrage. Un challenge expiré, non reconnu ou lié à une autre session peut néanmoins bloquer temporairement le site.

## Images absentes

- Vérifiez que la source est à jour dans Aidoku.
- Fermez puis rouvrez le chapitre pour renouveler les URL ou jetons temporaires.
- Si une vérification Cloudflare vient d’être validée, rejouez d’abord la requête comme indiqué ci-dessus.
- Confirmez que l’image s’ouvre encore sur le site d’origine.

Les sources ajoutent les `Referer`, cookies et en-têtes nécessaires. ScansFR utilise également des jetons d’images signés, et OrtegaScans peut fournir des JPEG très longs dépassant 40 000 pixels et 10 Mo.

## Chargement lent

La recherche, les filtres dynamiques et le premier affichage d’une image nécessitent des requêtes vers les sites externes. Le chargement peut donc varier selon leur disponibilité, Cloudflare, la connexion et la mémoire de l’appareil.

Les requêtes indépendantes sont déjà parallélisées avec une limite prudente. Augmenter fortement cette limite risquerait de déclencher davantage de protections ou de refus côté serveur.

## Changements des sites

Un domaine peut changer son HTML, ses routes, son API ou ses protections sans préavis. Dans ce cas, ouvrez un rapport avec :

- le nom de la source ;
- l’écran concerné ;
- le message d’erreur ;
- si possible, une œuvre et un chapitre reproductibles.
