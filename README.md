# PixelWeather Messaging Protocol Server
This is the server application for the PixelWeather Messaging Protocol, which is a custom protocol designed for communication between PixelWeather nodes. The server is responsible for handling incoming connections, processing requests, and managing the state of connected devices.

#### CI Stats
[![Run static analysis on commit](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/verify_commits.yml/badge.svg)](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/verify_commits.yml) | [![Release tagged versions](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/release.yml/badge.svg)](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/release.yml) | [![Build and push the Docker image](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/docker_build.yml/badge.svg)](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/docker_build.yml)

## Server configuration
The server will read the configuration file from `~/.config/pwmp-server/config.yml`. If it doesn't exist, it'll be created. Optionally, you can provide a `--config` flag, with an alternative path.

### Defaults
```yml
# Server binding
host: "0.0.0.0"
port: 55300

# Database connection settings.
# PostgreSQL or SQLite are supported.
database: !Postgres
  host: "123.456.789.012"
  port: 5432
  user: "root"
  password: "root"
  name: "pixelweather"
  ssl: false

# ... or SQLite:
database: !Sqlite
  # Path must be absolute
  file: "/path/to/database.db"

limits:
  # Maximum number of devices that can be connected at the same time.
  # This limit cannot be disabled.
  devices: 10

  # Sets how many settings can be requested using the `Message::GetSettings`  message.
  # This limit cannot be disabled.
  settings: 10

  # Maximum amount of time a client can stay connected without sending any requests. If the client stays connected for longer than this time, without communicating, it will be kicked.
  stall_time: 10

# Configuration for the built-in rate limiter.
rate_limiter:
  # Maximum amount of requests a client can make per second.
  max_requests: 20
  # Maximum amount of connections that may be accepted within 1 second.
  max_connections: 4

# Logging configuration
logging:
  # Path to the log file, or null to disable
  # Path must be absolute
  file: "/var/log/pwmp-server.log" # or null to disable

  # Whether to erase the log file on start
  erase_file_on_start: false
```

## Database support
|                      | **Supported** | **Tested version** |
|----------------------|:-------------:|:------------------:|
| PostgreSQL           |       ✅      |     16.x, 17.x     |
| SQLite               |       ✅      |       3.52.0       |
| MySQL                |       ❌      |                    |

### Chosing a database
PostgreSQL is the recommended database for production use with a large number of devices, while SQLite is suitable for small setups or testing purposes.
SQLite will perform a lot faster (min/max/avg response times <1ms) but offers less type safety and may result in slightly higher CPU and RAM usage due to not supporting certain features that have to be emulated with multiple queries or additional logic.
PostgreSQL causes higher latency (min/max/avg response times >10ms) but has higher priority when it comes to features.

## Using as a service
The CLI has a `service` subcommand, which allows managing a background service.

```
$ pwmp-server service help
Service management

Usage: pwmp-server service <COMMAND>

Commands:
  start      Start the service
  stop       Stop the service
  enable     Enable
  disable    Disable
  install    Install as service
  uninstall  Uninstall service
  check      Check if service is installed
  reinstall  Reinstall service
```

### Service manager support table
|                   **Service manager**                   |  **Supported** |
|:-------------------------------------------------------:|:--------------:|
|             [SystemD](https://systemd.io/)              |       ✅       |
|        [OpenRC](https://github.com/OpenRC/openrc)       |       ✅       |
|  [Homebrew Services](https://github.com/Homebrew/brew)  |       ❌       |
|                        Windows                          |       ❌       |

Service management on Windows is **not** and **will not** be supported.

## Signal handling
The server can be peacefully terminated using `SIGINT`:
```sh
kill -SIGINT $(pidof pwmp-server)
```

You can also send a simple "ping" request using `SIGUSR1`:
```sh
kill -SIGUSR1 $(pidof pwmp-server)
```

## Logging
You can force debug logging in release builds by using the `--debug` flag, or by setting the `PWMP_DEBUG` environment variable to `true` (case insensitive). Any other value will be ignored.
In debug builds, debug logging is enabled by default.

## Compiling caveats and portability
In general, *x86* and *aarch64* are well-supported with *Linux*. macOS *should* theoretically work. Windows is **not** supported and there are no plans for it.
Cross-compilation to macOS requires additional setup, due to licensing restrictions.

#### Platform support table
**Note**: This incomplete and may change in the future!
|          **Target**            | **Supported** | **`cargo-cross` support** |
| :----------------------------: | :-----------: | :-----------------------: |
|  `x86_64-unknown-linux-gnu`    |       ✅       |             ✅           |
|  `x86_64-unknown-linux-musl`   |       ✅       |             ✅           |
|   `x86_64-unknown-freebsd`     |       ✅       |             ✅           |
|  `aarch64-unknown-linux-gnu`   |       ✅       |             ✅           |
| `aarch64-unknown-linux-musl`   |       ✅       |             ✅           |
|    `aarch64-apple-darwin`      |       ✅       |             ❌           |
| `armv7-unknown-linux-gnueabi`  |       ✅       |             ✅           |
| `armv7-unknown-linux-musleabi` |       ✅       |             ✅           |

## Docker support
You can build a Docker image using the provided [`Dockerfile`](./Dockerfile).

```sh
docker build -t pwmp-server .
```

Use the included `docker-compose.yml` for a production-ready setup with PostgreSQL and several hardened options. **Do not forget to change the database credentials!** The binary is located at `/app/pwmp-server` in the container, and the configuration file path is set to `/app/data/config.yml`.

To run the necessary database migrations, you can use the following command:
```sh
docker compose exec pwmp-server /app/pwmp-server --config /app/data/config.yml database init
```

Running the server with a plain `docker run -it --rm pwmp-server:latest` will **not** work, as the server will try to create the configuration file and without a volume, the changes will be lost on container restart. You can use a bind mount to persist the configuration file.

You can use OpenSSL to generate random password string for the database:
```sh
openssl rand -hex 16
```

## Proxies
The server has been tested behind a reverse proxy using Nginx Proxy Manager stream, however, it caused some level of instability. Using reverse proxies is not recommended, as they may interfere with the custom socket optimizations.

## Over-the-Air updates
The PixelWeather network is designed to support over-the-air updates for devices. The architecture is fairly simple, with the database acting as a central repository for firmware files, and the server facilitating the distribution of these files to connected devices, when requested.

Notable details:
- Nodes with outdated firmware that is several versions behind, can only get the latest update.
  - Example: Assume a node with PWOS version 1.0.0, and the latest version is 1.2.0. The node will only receive the 1.2.0 update, and not anything in between.
- The server does not perform any validation on the firmware files, so it's the responsibility of the user to ensure that the correct files are uploaded to the database.
- The server does not perform any validation on the version numbers, so it's the responsibility of the user to ensure that the version numbers in `version_major`, `version_middle`, and `version_minor` are correct and match the version numbers in the firmware files.
- The `restrict_nodes` field can be used to restrict the update to specific nodes. It's type is a simple JSON array of node IDs.
  - A `NULL` value means that the update is available to all nodes.
  - An empty array means that the update is not available to any nodes. Useful for testing purposes.
  - An array with values (`[1, 2, 3]`) means that the update is only available to the nodes with the specified IDs.

Nodes must request firmware blobs in chunks, while the chunk size can be adjusted by the client even during the transfer. See [`Request::NextUpdateChunk`](https://github.com/PixelWeatherProject/pwmp-msg/blob/9d76debe97e316fc6dc76995db276a2ddf0e759d/src/request.rs#L59).

Every update attempt is logged in the `firmware_stats` table, which can be used to track the success rate of updates, and to identify any potential issues with specific firmware versions or devices.
The `success` field indicates whether the update was successful/unsuccessful, or hasn't been reported yet as either (`NULL`).