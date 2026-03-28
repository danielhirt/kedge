# Installation

## Shell installer (Linux / macOS)

The fastest way to install kedge:

```bash
curl -fsSL https://raw.githubusercontent.com/danielhirt/kedge/main/install.sh | sh
```

This detects your platform and downloads the latest release binary to `/usr/local/bin`.

Override the install directory:

```bash
curl -fsSL https://raw.githubusercontent.com/danielhirt/kedge/main/install.sh | KEDGE_INSTALL_DIR=~/.local/bin sh
```

## Homebrew (macOS / Linux)

```bash
brew install danielhirt/tap/kedge
```

## Pre-built binaries

Download from [GitHub Releases](https://github.com/danielhirt/kedge/releases). Binaries are available for:

| Platform | Architecture |
|----------|-------------|
| Linux    | x86_64, aarch64 |
| macOS    | x86_64, aarch64 |
| Windows  | x86_64, aarch64 |

Each archive includes a `.sha256` checksum file. Verify after download:

```bash
sha256sum -c kedge-linux-x86_64.tar.gz.sha256
```

## From source

Requires Rust 1.70+ and `git` on PATH:

```bash
cargo install --path .
```

Or install directly from the repository:

```bash
cargo install --git https://github.com/danielhirt/kedge.git
```

## Docker

Run kedge without installing anything locally:

```bash
docker run --rm -v "$PWD:/repo" -w /repo danielhirt/kedge check
```

Build the image from source:

```bash
docker build -t kedge .
```

## CI runner setup

For air-gapped or pre-baked CI runners, copy the binary into your runner image:

```dockerfile
COPY kedge /usr/local/bin/kedge
```

On developer machines, `kedge install --link` symlinks steering files from the docs repo to your local agent directories. In CI, `kedge install --workspace` copies them into the workspace instead.

## Verify installation

```bash
kedge --help
```

You should see the list of available subcommands. Next, head to [Quick Start](quick-start.md) to set up your first repository.
