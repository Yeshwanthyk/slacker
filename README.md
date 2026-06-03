# slacker

Turn a link or video clip into a Slack-ready custom emoji GIF.

Slack caps custom emoji at 128 KiB. `slacker` downloads the source, crops it
square, and steps down through size/frame-rate/color profiles until the GIF
fits, all by driving `ffmpeg`. No Rust dependencies.

## Install

Needs `curl` and `ffmpeg` on your PATH at runtime.

**Prebuilt binary** (macOS and Linux, arm64/x64):

```bash
curl -fsSL https://raw.githubusercontent.com/Yeshwanthyk/slacker/main/install.sh | sh
```

Installs to `~/.local/bin`. Override with `SLACKER_INSTALL_DIR=...` or pin a
release with `SLACKER_VERSION=v0.1.0`. The script verifies the SHA-256 checksum.
Prefer to read before you pipe? Inspect [`install.sh`](install.sh) first, or
grab the archive yourself from the [releases page](https://github.com/Yeshwanthyk/slacker/releases).

**From source** (needs a Rust toolchain):

```bash
cargo install --git https://github.com/Yeshwanthyk/slacker --locked slacker
```

## Examples

```bash
# GIPHY link
slacker 'https://giphy.com/gifs/cigarette-HB4aJElNd7JMas9WSU' --name hill_cigarette

# Local file into a chosen directory
slacker ./reaction.mov --out-dir ~/Downloads --name shrug

# Pipe from stdin
curl -s https://example.com/clip.mp4 | slacker - --name wave
```

## Inputs

`slacker <input>` accepts:

- GIPHY links — `https://giphy.com/gifs/...`
- Tenor view pages — `https://tenor.com/view/...-gif-123`
- Imgur links — `https://imgur.com/abc123`
- direct media URLs — `.gif`, `.mp4`, `.webp`, `.webm`, `.mov`
- local file paths
- `-` for stdin

For remote clips the `.mp4` is preferred over the `.gif` when both exist, so the
palette is built from cleaner source frames.

## Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--name NAME` | derived from input | Emoji name (lowercased, non-word chars become `_`, capped at 64). |
| `--out-dir DIR` | `/tmp` | Output directory. |
| `--fit crop\|pad` | `crop` | `crop` center-crops to a square; `pad` shrinks the whole frame and adds a transparent border. |
| `--max-bytes N` | `120000` | Size budget. Slack's hard limit is 128 KiB. |
| `--max-frames N` | `50` | Frames kept from the source. |
| `--force` | off | Overwrite an existing output file. |
| `--json` | off | Print `{"path":...,"bytes":...,"name":...}` instead of just the path. |

## How it works

The output is built by trying a fixed ladder of profiles — descending
dimensions (128 → 80 px), frame rates, and color counts — and keeping the first
result at or under `--max-bytes`. Long clips are trimmed to `--max-frames`.

## License

MIT OR Apache-2.0.
