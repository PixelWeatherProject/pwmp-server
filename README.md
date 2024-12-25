# PWMP Server
This crate contains the PixelWeather Messaging Protocol Server and CLI.

# Server configuration
The server will read the configuration file from `~/.config/pwmp-server/config.yml`. If it doesn't exist, it'll be created. Optionally, you can provide a `--config` flag, with an alternative path.

## Defaults
```yml
# Server binding
host: "0.0.0.0"
port: 55300

# Database connection settings.
# PostgreSQL is the only supported database.
db_host: "123.456.789.012"
db_port: 5432
db_user: "root"
db_password: "root"
db_name: "pixelweather"

# Maximum number of devices that can be connected at the same time.
# This limit cannot be disabled.
max_devices: 10

# Sets how many settings can be requested using the `Message::GetSettings` message.
# This limit cannot be disabled.
max_settings: 10
```

# Using as a service
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