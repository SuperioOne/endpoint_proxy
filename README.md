# Endpoint Proxy

Endpoint Proxy is a simple HTTP proxy that allows you to skip CORS when possible, rewrite paths, method type, request
bodies, and headers using a YAML configuration file.

- [Usage](#usage)
  - [Binary](#binary)
  - [Docker](#docker)
  - [Kubernetes basic example](#kubernetes-basic-example)
- [Server Configuration](#server-configuration)
  - [CLI arguments](#cli-arguments)
  - [Container environment variables](#container-environment-variables)
- [Endpoint Configuration](#endpoint-configuration)
  - [Configuration options](#configuration-options)
- [Building From Source](#building-from-source)
- [License](#license)

## Usage

### Binary

1. Create a `config.yaml` file with URL configurations.
    ```yaml
    # config.yaml
    proxy_urls:
      - path: "/my-ip"
        url: "https://api.my-ip.io/ip.json"
        headers:
          - name: "Accept"
            value: "application/json"
    ```
2. Start `endpoint_proxy` executable.
    ```shell
     endpoint_proxy --config-file config.yaml
    ``` 
3. Test the service
    ```shell
    curl http://localhost:8080/my-ip
    ```

> See [CLI Arguments](#cli-arguments) for available options.

### Docker

1. Create a `config.yaml` file.
2. Start container with config file mounted to `/etc/endpoint_proxy/config.yaml`.
    ```shell
     docker run -v $(pwd)/config.yaml:/etc/endpoint_proxy/config.yaml -p 8080:8080 endpoint_proxy:latest
    ```
3. Test the service.
    ```shell
    curl http://localhost:8080/my-ip
    ```
> See [Container Environment Variables](#container-environment-variables) for available options.

### Kubernetes basic example

```yaml
# Endpoint configuration file as config map.
apiVersion: v1
kind: ConfigMap
metadata:
  name: endpoint-proxy-configmap
data:
  config.yaml: |
    proxy_urls:
      - path: /my-ip
        url: https://api.my-ip.io/ip.json
        headers:
          - name: Accept
            value: application/json
      - path: /posts
        url: https://jsonplaceholder.typicode.com/posts
        headers:
          - name: Accept
            value: application/json
      - path: /posts
        url: https://jsonplaceholder.typicode.com/posts
        method: post
        default_body: |
          {
            "title": "foo",
            "body": "bar",
            "userId": 1
          }
        headers:
          - name: "Accept"
            value: "application/json"
          - name: "Content-Type"
            value: "application/json"

---
apiVersion: v1
kind: Pod
metadata:
  name: myapp
  labels:
    name: myapp
spec:
  volumes:
    - name: endpoint-config-file
      configMap:
        name: endpoint-proxy-configmap
  containers:
    - name: myapp
      image: <Image>
      ports:
        - containerPort: 8080             # Expose the same port from 'HTTP_PORT' env variable.
      volumeMounts:
        - name: endpoint-config-file
          mountPath: /etc/endpoint_proxy  # Make sure mount point is valid for 'ROUTE_CONF_LOCATION' env variable.
```

## Server Configuration

### CLI arguments

| Name                | Default Value       | Allowed values                                                            |
|---------------------|---------------------|---------------------------------------------------------------------------|
| `--log-level`       | `INFO`              | `INFO`, `DEBUG`, `WARN`, `ERROR`, `OFF`, `TRACE`                          |
| `--bind`            | `0.0.0.0`           | `IP Address`                                                              |
| `--port`            | `8080`              | 1-65535                                                                   |
| `--proxy-url`       | -                   | Proxy server URL. `socks5://xyz.com`, `http://xyz.com`, `https://xyz.com` |
| `--proxy-auth-user` | -                   | (Optional) Proxy server authentication user                               |
| `--proxy-auth-pass` | -                   | (Optional) Proxy server authentication password                           |
| `--enable-cookies`  | -                   | `true`, `false`                                                           |
| `--worker-count`    | All available cores | number                                                                    |
| `--config-file`     | `config.yaml`       | `Path`                                                                    |

### Container environment variables

| Name                  | Default Value                     | Allowed values                                      |
|-----------------------|-----------------------------------|-----------------------------------------------------|
| `LOG_LEVEL`           | `INFO`                            | `INFO`, `DEBUG`, `WARN`, `ERROR`, `OFF`, `TRACE`    |
| `HTTP_BIND`           | `0.0.0.0`                         | `IP Address`                                        |
| `HTTP_PORT`           | `8080`                            | 1-65535                                             |
| `HTTP_PROXY_URL`      | -                                 | Proxy server URL `socks5://`, `http://`, `https://` |
| `HTTP_PROXY_USER`     | -                                 | (Optional) Proxy server authentication user         |
| `HTTP_PROXY_PASS`     | -                                 | (Optional) Proxy server authentication password     |
| `ENABLE_COOKIES`      | -                                 | `true`, `false`                                     |
| `HTTP_WORKER_COUNT`   | All available cores               | number                                              |
| `ROUTE_CONF_LOCATION` | `/etc/endpoint_proxy/config.yaml` | `Path`                                              |

## Endpoint Configuration

Create a YAML configuration file to specify the proxy rules. For example, create a file named `config.yaml` with the
following content:

```yaml
proxy_urls:
  - path: "/my-ip"
    url: "https://api.my-ip.io/ip.json"
    method: "get"
    headers:
      - name: "Accept"
        value: "application/json"
```

In this example, any request to `http://localhost:8080/my-ip` will be forwarded to `https://api.my-ip.io/ip.json`
using the HTTP method `GET`. Additionally, the request will include an `Accept: application/json` header.

### Configuration options

### `proxy_urls`

- List of objects representing proxy rules.
- Each rule contains:
    - `path`: The path on the local server that triggers this rule.
    - `url`: The target URL where the request will be forwarded.
    - `method`: (optional) The HTTP method to use for the incoming request. Default value is `get`.
    - `target_method` (optional): The HTTP method to use for the forwarded request. If not defined, `method` value is
      used.
    - `default_body` (optional): The default request body to use if one is not provided in the incoming request.
    - `headers` (optional): A list of objects representing headers to be added to the request.
    - `query` (optional): A list of objects representing query parameters to be added to the request.

> `target_method` and `method` properties are case-sensitive and accepts the following **lowercase** HTTP
> verbs; `get`, `post`, `put`, `delete`, `head` and `patch`.

**Example**:

```yaml
proxy_urls:
  - path: /my-ip
    url: https://api.my-ip.io/ip.json
    headers:
      - name: Accept
        value: application/json
  - path: /posts
    url: https://jsonplaceholder.typicode.com/posts
    headers:
      - name: Accept
        value: application/json

  - path: /posts
    url: https://jsonplaceholder.typicode.com/posts
    method: post
    default_body: |
      {
        "title": "foo",
        "body": "bar",
        "userId": 1
      }
    headers:
      - name: "Accept"
        value: "application/json"
      - name: "Content-Type"
        value: "application/json"
  - path: /search
    url: https://duckduckgo.com
    method: post
    target_method: get
    query:
      - name: "t"
        value: "ffab"
      - name: "q"
        value: "alpine+linux"
      - name: "ia"
        value: "web"
```

Mappings are:

- GET: http://localhost:8080/my-ip **translates to -->** GET: https://api.my-ip.io/ip.json
- GET: http://localhost:8080/posts **translates to -->** GET: https://jsonplaceholder.typicode.com/posts
- POST: http://localhost:8080/posts **translates to -->** POST: https://jsonplaceholder.typicode.com/posts
- POST: http://localhost:8080/search **translates to -->** GET: https://duckduckgo.com/?t=ffab&q=alpine+linux&ia=web

## Building From Source

- **Binary:**
    ```bash
    git clone http://update-this.git
    cd endpoint_proxy
    cargo build --release
    ```
- **Container image**
    ```bash
    git clone http://update-this.git
    cd endpoint_proxy
    buildah build -f ./container/Dockerfile -t "endpoint_proxy:latest" .
    ```

## But Why?

![alt](https://media.tenor.com/KjJTBQ9lftsAAAAC/why-huh.gif)

Technically there is not much use case.

I simply use it as a sidecar container to duck-tape my RSS aggregator to work with some finicky sites, a quick dirty way to bypass CORS on some 
APIs and allow request proxying for a service that lacks built-in support.

## License

This project is licensed under the [MIT License](LICENSE).
