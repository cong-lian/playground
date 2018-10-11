# Intro

This directory contains 3 bash files and 2 extfile.

**ownCAGen** is to create custom root CA (Certificate Authority).
**certGen** is to generate a private key and CSR (Certificate Signing Request) for server and sign the certificate with your custom root CA.
**clientCertGen** is to generate a private key and CSR (Certificate Signing Request) for client and sign the certificate with your custom root CA.

# Usage
If necessary, edit the files to meet your own needs.

Generate custom root CA first.
```bash
./ownCAGen
```
Enter pass phrase for MyRootCA.key twice.

And then generate a private key for your web service and sign it with the above generated custom root CA.
```bash
./certGen
```
Enter pass phrase for MyRootCA.key.

And then generate a private key for your client and sign it with the above generated custom root CA.
```bash
./clientCertGen client1
```
Enter pass phrase for MyRootCA.key.
