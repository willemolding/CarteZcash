# Instructions for deployment

## Non CI Deployment

### on-chain deployment

Build the machine docker file and deploy to fly.io image registry

```shell
cartesi build
export IMAGE_TAG=$(cartesi deploy build --platform linux/amd64 | grep "Application node Docker image" | awk '{print $NF}')
docker tag $IMAGE_TAG registry.fly.io/cartezcash:latest
docker push registry.fly.io/cartezcash:latest
```

Take note of the machine/template hash.

Next visit https://sunodo.io/deploy and use the helper to deploy the on-chain component to Sepolia

Copy the ENV vars from the fly.toml produced and past them into the fly.toml in the repo.

### Node deployment

Clear the database if it exists already by running

```shell
fly postgres connect -a cartezcash-database 
```

and then in the database shell run

```sql
DROP SCHEMA public CASCADE;
CREATE SCHEMA public;
     
GRANT ALL ON SCHEMA public TO postgres;
GRANT ALL ON SCHEMA public TO public;
```

Finally run

```shell
fly deploy
```

### Fullnode deployment

Update the DAPP_ADDRESS in the fly.fullnode.toml

Build and deploy the fullnode in one step by running

```shell
fly deploy --config fly.fullnode.toml 
fly restart cartezcash-fullnode    
```

### Bridge UI deployment

Update the dApp address in the [config.json](./bridge-frontend/src/config.json) then commit the changes and push to main.

Run the Deploy bridge Github Pages action.