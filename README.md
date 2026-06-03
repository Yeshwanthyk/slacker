# slacker

Tiny Rust CLI for turning a link or clip into a Slack-ready custom emoji GIF.

## Usage

```bash
slacker '<input>' [flags]
```

`<input>` can be:

- a **GIPHY** link (`https://giphy.com/gifs/...`)
- a **Tenor** view page (`https://tenor.com/view/...-gif-123`)
- an **Imgur** link (`https://imgur.com/abc123`)
- any **direct media URL** (`.gif`, `.mp4`, `.webp`, `.webm`, `.mov`)
- a **local file** path
- `-` to read media from **stdin**

For remote clips the full-quality `.mp4` is preferred over an already-quantized
`.gif`, so the palette is built from the richest available frames.

```bash
slacker 'https://giphy.com/gifs/cigarette-HB4aJElNd7JMas9WSU' \
  --name hill_cigarette --out-dir ~/Downloads
```

### Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--name NAME` | derived from input | Emoji name (sanitized, capped at 64 chars). |
| `--out-dir DIR` | `/tmp` | Output directory. |
| `--fit crop\|pad` | `crop` | `crop` center-crops to a square; `pad` letterboxes the whole frame with a transparent border. |
| `--max-bytes N` | `120000` | Size budget. Slack's hard cap is 128 KiB. |
| `--max-frames N` | `50` | Frames kept from the source clip. |
| `--force` | off | Overwrite an existing output file. |
| `--upload` | off | Upload the result to Slack (see below). |
| `--team SUBDOMAIN` | `$SLACK_TEAM` | Workspace subdomain for `--upload`. |
| `--json` | off | Emit machine-readable output. |

Agent-friendly output:

```bash
slacker '<input>' --name hill_cigarette --json
# {"path":"...","bytes":108535,"name":"hill_cigarette","uploaded":false}
```

## Uploading to Slack

Slack has no official public emoji-add API, so `--upload` posts to the
workspace's internal `emoji.add` endpoint, matching how community emoji tooling
works. Credentials come from the environment:

- `SLACK_TOKEN` (required): an `xoxc`/`xoxs`/`xoxp` token with emoji rights.
- `SLACK_TEAM` (or `--team`): the workspace subdomain in `<team>.slack.com`.
- `SLACK_COOKIE` (optional): the `d` cookie value, required for `xoxc` tokens.

The token and cookie are passed through a private `curl --config` file (mode
`0600`), so they never appear in the process table.

```bash
SLACK_TOKEN=xoxc-… SLACK_TEAM=acme SLACK_COOKIE=… \
  slacker 'https://giphy.com/gifs/…' --name wave --upload
```

## Requirements

- `curl`
- `ffmpeg`

The binary has no Rust crate dependencies. Media work is delegated to FFmpeg.
