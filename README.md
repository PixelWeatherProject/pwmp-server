# PWMP Server
This crate contains the PixelWeather Messaging Protocol Server and CLI.

#### CI Stats
[![Run static analysis on commit](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/verify_commits.yml/badge.svg)](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/verify_commits.yml) | [![Release tagged versions](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/release.yml/badge.svg)](https://github.com/PixelWeatherProject/pwmp-server/actions/workflows/release.yml)

## Server configuration
The server will read the configuration file from `~/.config/pwmp-server/config.yml`. If it doesn't exist, it'll be created. Optionally, you can provide a `--config` flag, with an alternative path.

### Defaults
```yml
# Server binding
host: "0.0.0.0"
port: 55300

# Database connection settings.
# PostgreSQL is the only supported database.
database:
  host: "123.456.789.012"
  port: 5432
  user: "root"
  password: "root"
  name: "pixelweather"
  ssl: false
  timezone: null

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
  time_frame: 1 # second(s)
  # Maximum amount of requests a client can make within the specified time frame above.
  max_requests: 4
  # Maximum amount of connections that may be accepted within the specified time frame above.
  max_connections: 4
```

## Database server
Only PostgreSQL is supported. It's recommended to use the latest version.

Compatibility has been verified with the following versions:
- 16.x
- 17.4

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

Service management is only supported on **Linux** systems with **Systemd**. There is a boilerplate implementation for **OpenRC**, but it's not supported yet.

TODO:
- [x] Add support for Systemd services
- [ ] Add support for OpenRC services
- [ ] Add support for macOS Homebrew services

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

## Compiling caveats and portability
In general, *x86* and *aarch64* are well-supported with *Linux*. macOS *should* theoretically work. Windows is **not** supported and there are no plans for it.
Cross-compilation to macOS requires additional setup, due to licensing restrictions.

#### Platform support table
**Note**: This incomplete and may change in the future!
|          **Target**           | **Supported** | **`cargo-cross` support** |
| :---------------------------: | :-----------: | :-----------------------: |
|  `x86_64-unknown-linux-gnu`   |       ✅       |             ✅             |
|  `x86_64-unknown-linux-musl`  |       ✅       |             ✅             |
|   `x86_64-unknown-freebsd`    |       ❌       |            N/A            |
|      `x86_64-pc-solaris`      |       ❌       |            N/A            |
|  `aarch64-unknown-linux-gnu`  |       ✅       |             ✅             |
| `aarch64-unknown-linux-musl`  |       ✅       |             ✅             |
|    `aarch64-apple-darwin`     |       ✅       |             ❌             |
| `armv7-unknown-linux-gnueabi` |       ✅       |             ✅             |

## Docker support
You can build a Docker image using the provided [`Dockerfile`](./Dockerfile).

```sh
docker build -t pwmp-server .
```

You can test the container using the following command:
```sh
# Note that the config already exists on the host!
docker run --rm -it -v ~/.pwmp-server:/config:ro pwmp-server
```