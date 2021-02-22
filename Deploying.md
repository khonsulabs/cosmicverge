# Deploying Cosmic Verge

This page walks through how Cosmic Verge is deployed into a Kubernetes cluster. The [kubernetes yaml files](./kubernetes) have some specific options that configure DigialOcean's LoadBalancer, but beyond that they should be generic.

## PostgreSQL

Currently PostgreSQL is being hosted outside of the Kubernetes cluster. This may not always be the case (I want to investigate [postgres-operator](https://github.com/CrunchyData/postgres-operator)), but for now you must create a PostgreSQL 12.x or 13.x database. Previous versions may work, but they aren't actively used.

Once you have a PostgreSQL cluster running, here's an example of how to create a user and database:

```sql
CREATE ROLE cv_user LOGIN PASSWORD '***' CONNECTION LIMIT -1;
CREATE DATABASE cosmicverge OWNER cv_user CONNECTION LIMIT -1;
```

Once you've created your database, you'll need to store the DATABASE_URL in Kubernetes as a secret. Here's an example using `kubectl`:

```bash
kubectl create secret generic database-url --from-literal=url='postgresql://cv_user:***@hostname:port/cosmicverge?sslmode=require'
```

## Twitch OAuth

Create an OAuth application in Twitch's developer console. Once you have your client ID and client secret, add them to Kubernetes as a secret. If using `kubectl`:

```bash
kubectl create secret generic twitch-oauth \
  --from-literal=id='***' \
  --from-literal=secret='***'
```

## Redis

Apply the Redis service:

```bash
kubectl apply -f kubernetes/redis.yml
```

## Building a Docker image

You'll need to build a docker image that contains the built `cosmicverge-server` executable, pre-built static assets, and the current WASM build.

The [cosmicverge-build](https://github.com/khonsulabs/cosmicverge-build/) repository covers the basic steps of building the project and assets, and the [Dockerfile](./Dockerfile) should be buildable with `docker build .`.

You'll need to push this image to a repository that Kubernetes can access. To see how this project does it with DigitalOcean, you can review the [GitHub Actions Workflow](./.github/workflows/deploy.yml).

## Deploying the App

Edit the [production.yml](./kubernetes/production.yml) file to reference the repository that contains your image.

Once it's updated to the latest tag/image, you can apply it similarly to the Redis service:

```bash
kubectl apply -f kubernetes/production.yml
```
