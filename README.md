# volkanicmc-construct

Automatically deploy any Minecraft server by description.

## Notice
**VolkanicMC Construct is still early-alpha software. Do not use in production.**

## Usage
Command-line usage can be provided with the following command:
```sh
volkanicmc-construct help
```

## Getting started

### Create a base template
```sh
volkanicmc-construct template create > my-template.json
```

This command prints a basic but functional template. We then redirect that output into a file.

### Build the template
```sh
volkanicmc-construct build my-template.json
```

Append `--force` if you would like to allow overwriting the previous build.

### Run the build
```sh
volkanicmc-construct run
```

This works fine for quick local testing, but shouldn't be considered secure. Ideally, you should generate a shell script for executing the server.

```sh
volkanicmc-construct exec-script > start.sh && chmod +x start.sh
./start.sh
```

Currently, only `sh` shell scripts can be generated.