## Railyard

A soon to be event streaming platform

[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)


# Running


Certificates for each node in the cluster need to be signed using the same CA. Generate
the CA and store it in Kubernetes:
```
# Generate a new private key
openssl genrsa -out ca.key 2048
openssl req -new -x509 -key ca.key -out ca.crt -subj "/CN=localhost"

# Generate a self-signed certificate
openssl req -new -x509 -key ca.key -out ca.crt -subj "/CN=localhost"

# Create a Kubernetes Secret containing the certificate and private key
kubectl create secret generic ca-secret --from-file=ca.crt --from-file=ca.key
```


Run a single docker container
```
docker build -t your-image-name .
docker run -p 5000:5000 your-image-name
```

Running the cluster
```
kubectl apply -f deployment.yaml
```

## License

This project is licensed under the [MIT license](LICENSE).
