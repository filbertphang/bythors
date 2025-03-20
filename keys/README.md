these are sample keys to provide the node identity for each libp2p node

key generation:

- (keypair in pem): `openssl genrsa -out private.pem 2048`
- (keypair in pkcs8 as der): `openssl pkcs8 -in private.pem -inform PEM -topk8 -out private.pk8 -outform DER -nocrypt`
- (pubkey in x509 as der): `openssl rsa -in private.pem -pubout -out public.der -outform DER`
