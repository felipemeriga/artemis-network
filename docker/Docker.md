# Documentation for Dockerization of Artemis Network

This documentation explains how to configure and run Artemis Network nodes in a **Docker Compose** environment or as a standalone container. It provides instructions on creating configuration files, running the containers, and leveraging the public Docker image.

---

## Configuration File Description

The nodes in Artemis Network use a YAML configuration file to define important parameters. Below is an explanation of the fields in the configuration file:

```yaml
tcpAddress: "node1:5000"                # The TCP address for internal network communication.
httpAddress: "0.0.0.0:8080"             # HTTP address for exposing the node's API. 
                                       
bootstrapAddress: null                  # Add an address of other node if you are running this node after the network already exists
                                     
nodeId: "node-1"                        # Unique identifier for this node in the network.
minerWalletAddress: "30114c915aae70..." # Address for miner wallet. Used in scenarios with blockchain mining.
```

### Notes on Configuration
- Each node in the network should have its own unique `nodeId` and configuration file.
- If running the network in **bootstrap mode** (i.e., without an external bootstrap node), set `bootstrapAddress` to `null`.
- The configuration file is mounted into the container using Docker's volume mechanism.

---

## Building the Docker Image

All the files for building the docker image, running docker-compose are located over `./docker` directory.
```
artemis-network/
├── docker/
│   ├── config/
│   │   ├── node-1.yaml
│   │   ├── node-2.yaml
│   │   ├── node-3.yaml
│   ├── Docker.md
│   ├── docker-compose.yml
│   ├── Dockerfile

```

In order to build the docker image, be sure you are in the root of this repo, and run the following command:
```shell
docker build -f ./docker/Dockerfile -t artemis-network .
```

If you want, you can use the [public image](https://hub.docker.com/r/felipemeriga1/artemis-network), instead of building the docker image yourself.

```shell
docker pull felipemeriga1/artemis-network
```
---
## Running Artemis Network as a Standalone Container

To run Artemis network in **standalone mode**, you can run the following steps inside `./docker`
directory:


1. **Create Configuration File**  
   Save a configuration file similar to the example above (for example, `config-1.yaml`), and ensure it defines the correct parameters specific to the node.  You can use the same configuration file we have in this repo.

2. **Run with Standalone Command**  
   Use this command to run the Artemis network as a standalone container (inside `./docker` directory):

```shell script
docker run \
       -p 8080:8080 \
       -v $(pwd)/config/config-1.yaml:/app/config/config-1.yaml \
       felipemeriga1/artemis-network:latest \
       --config ./config/config-1.yaml
```

- `-p 8080:8080` maps the container's internal port 8080 to your host machine's port 8080.
- `-v $(pwd)/config/config-1.yaml:/app/config/config-1.yaml` mounts your configuration file into the container.
- Use the `--config` argument to specify the path to the configuration file inside the container.

3. **Pull the Public Image**  
   If you do not wish to build your own Docker image, you can use the public image available on Docker Hub:

   **Image URL**:  
   [https://hub.docker.com/r/felipemeriga1/artemis-network](https://hub.docker.com/r/felipemeriga1/artemis-network)

   You can replace `artemis-network:latest` with `felipemeriga1/artemis-network:latest` in your commands to use the public image.

---

## Running Artemis Network with Docker Compose

To simplify managing multiple nodes, use **Docker Compose** with the following configuration:

### Example `docker-compose.yaml`

```yaml
version: "3.8"

services:
  node1:
    image: artemis-network:latest
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080" # External (host) port mapped to container's 8080
    volumes:
      - ./config/node-1.yaml:/app/config/node-1.yaml
    command: --config ./config/node-1.yaml
    networks:
      app-network:
        aliases:
          - node1 # Internal hostname

  node2:
    image: artemis-network:latest
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8081:8080" # External port mapped to container's 8080
    volumes:
      - ./config/node-2.yaml:/app/config/node-2.yaml
    command: --config ./config/node-2.yaml
    networks:
      app-network:
        aliases:
          - node2 # Internal hostname

  node3:
    image: artemis-network:latest
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8082:8080" # External port mapped to container's 8080
    volumes:
      - ./config/node-3.yaml:/app/config/node-3.yaml
    command: --config ./config/node-3.yaml
    networks:
      app-network:
        aliases:
          - node3 # Internal hostname

networks:
  app-network:
    driver: bridge
```

### Instructions for Using `docker-compose.yaml`

1. **Create Node Configuration Files**  
   Create separate configuration files for each node (`node-1.yaml`, `node-2.yaml`, `node-3.yaml`) and place them in the `config` directory. Update the `nodeId` and other parameters as needed.

   Example for `node-1.yaml`:
```yaml
tcpAddress: "node1:5000"
   httpAddress: "0.0.0.0:8080"
   bootstrapAddress: null
   nodeId: "node-1"
   minerWalletAddress: "abcdef1234567890..."
```

2. **Start the Nodes**  
   Run the following command to start all nodes defined in the `docker-compose.yaml` file:

```shell script
docker-compose up
```

3. **Access the nodes**
    - Node 1 will be accessible on `http://localhost:8080`
    - Node 2 will be accessible on `http://localhost:8081`
    - Node 3 will be accessible on `http://localhost:8082`

4. **Using the Public Docker Image**  
   Replace `artemis-network:latest` with `felipemeriga1/artemis-network:latest` in the `docker-compose.yaml` file if you wish to use the pre-built public Docker image.

---

## Network Setup

- **Bridge Network**  
  The nodes communicate internally using a Docker bridge network named `app-network`. Each node is assigned an alias (e.g., `node1`, `node2`) that can be used as an **internal hostname**.

- **External Ports**  
  Each node exposes its API on unique ports on the host machine (e.g., `8080`, `8081`, `8082`) as defined in the `ports` section.

---

By following the instructions outlined above, you can configure and run the Artemis Network either as standalone containers (for individual nodes) or as a fully networked solution using Docker Compose. For simplified runtime management, leveraging the provided public image is recommended.