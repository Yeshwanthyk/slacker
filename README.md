# slacker

Tiny Rust CLI for turning a GIPHY link into a Slack-ready custom emoji GIF.

## Usage

```bash
slacker make 'https://giphy.com/gifs/cigarette-cigarettes-culture-of-the-hill-HB4aJElNd7JMas9WSU' \
  --name hill_cigarette
```

Agent-friendly output:

```bash
slacker make '<giphy-url>' --name hill_cigarette --json
```

## Requirements

- `curl`
- `ffmpeg`
- `ffprobe`

The binary has no Rust crate dependencies. Media work is delegated to FFmpeg.
