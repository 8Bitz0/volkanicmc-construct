![GitHub License](https://img.shields.io/github/license/8Bitz0/volkanicmc-construct)
![GitHub Issues or Pull Requests](https://img.shields.io/github/issues/8Bitz0/volkanicmc-construct)
![GitHub commit activity](https://img.shields.io/github/commit-activity/t/8Bitz0/volkanicmc-construct)

# volkanicmc-construct

Automatically deploy any Minecraft server by description.

## Notice
**VolkanicMC Construct is still alpha software and not meant for use in production environments.**

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

Append `--force` or `-f` to allow overwriting the previous build.

### Run the build
```sh
vkconstruct run
```

This works fine for local testing, but isn't considered secure. Ideally, you'd generate a shell script for starting the server.

```sh
vkconstruct exec-script bash > start.sh && chmod +x start.sh
./start.sh
```

Or for batch scripts:

```
vkconstruct exec-script batch
```
