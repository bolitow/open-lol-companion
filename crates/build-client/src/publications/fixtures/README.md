# Certificats de recette WSS (#125)

Ces fichiers DER sont des fixtures synthétiques publiques. `server-key.der` est
une **clé privée de test publiée volontairement** ; elle ne doit jamais servir
à un déploiement. La clé de signature de l'autorité n'est pas distribuée.

- `authority.der` : autorité de test, approuvée uniquement par un client de test.
- `server.der` : certificat serveur avec `DNS:localhost`, valable jusqu'en 2126.
- `server-key.der` : clé PKCS#8 du serveur de test.

Les tests utilisent Rustls et des ports loopback éphémères sur les deux OS.
Aucun OpenSSL, trousseau système, réseau externe ou vérificateur TLS permissif
n'est nécessaire à leur exécution. Le client de production conserve son magasin
d'autorités publiques. Un second test utilise ce magasin inchangé et vérifie
que la chaîne synthétique est refusée avant l'authentification WebSocket.
