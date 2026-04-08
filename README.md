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

You can use OpenSSL to generate random password string for the database:
```sh
openssl rand -hex 16
``` 

## Proxies
The server has been tested behind a reverse proxy using Nginx Proxy Manager stream, however, it caused some level of instability. Using reverse proxies is not recommended, as they may interfere with the custom socket optimizations.