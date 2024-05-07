![GitHub License](https://img.shields.io/github/license/8Bitz0/volkanicmc-construct)
![GitHub Issues or Pull Requests](https://img.shields.io/github/issues/8Bitz0/volkanicmc-construct)
![GitHub commit activity](https://img.shields.io/github/commit-activity/t/8Bitz0/volkanicmc-construct)
![Github Created At](https://img.shields.io/github/created-at/8Bitz0/volkanicmc-construct)

# volkanicmc-construct

Automatically deploy any Minecraft server by description.

## Notice
**VolkanicMC Construct is still early-alpha software. Do not use in production.**

## Usage
Command-line usage can be provided with the following command:
```sh
vkconstruct help
```

## Getting started

### Create a base template
```sh
vkconstruct template create > my-template.json
```

This command prints a basic but functional template. We then redirect that output into a file.

### Build the template
```sh
vkconstruct build my-template.json
```

Append `--force` if you would like to allow overwriting the previous build.

### Run the build
```sh
vkconstruct run
```

This works fine for quick local testing, but shouldn't be considered secure. Ideally, you should generate a shell script for executing the server.

```sh
vkconstruct exec-script bash > start.sh && chmod +x start.sh
./start.sh
```

Currently, only `sh` shell scripts can be generated.
