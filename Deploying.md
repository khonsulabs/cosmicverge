# Deploying Cosmic Verge

This page walks through how Cosmic Verge is deployed into a Kubernetes cluster. The [kubernetes yaml files](./kubernetes) have some specific options that configure DigialOcean's LoadBalancer, but beyond that they should be generic.

## PostgreSQL

### Install postgres-operator

```bash
kubectl create namespace pgo
kubectl apply -f kubernetes/postgres-operator.yml
```

You will need to wait for the installer to finish, then run this cleanup step:

```bash
kubectl delete -f kubernetes/postgres-operator.yml
```

### Create the postgres cluster

Configure the [pgo client](https://access.crunchydata.com/documentation/postgres-operator/4.6.1/installation/pgo-client/) before proceeding/

```bash
pgo create cluster primary \
  --username=cosmicuser \
  --password='***' \
  --pvc-size=20Gi \
  --replica-count=1 \
  --sync-replication \
  --metrics \
  --pgbadger \
  --pgbouncer \
  --database=cosmicverge
```

Wait for the cluster to be ready:

```bash
pgo test primary
```

Once you've created your cluster, you'll need to store the DATABASE_URL in Kubernetes as a secret. Here's an example using `kubectl`:

```bash
kubectl create secret generic database-url --from-literal=url='postgresql://cosmicuser:***@primary-pgbouncer.pgo.svc.cluster.local:5432/cosmicverge'
```

Due to how postgres-operator deploys pgbouncer and replicas to try to ensure spread across nodes, you might have issues with all of those services running. For a "less nodes required" setup, remove `--pgbouncer` and change the hostname to not include `-pgbouncer`. The Cosmic Verge executables already do connection pooling, and the value of pgbouncer in this setup hasn't been tested.

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
