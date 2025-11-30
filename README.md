# Capture as Image

This crate provides a screenshot command line utility for Windows.

## Build

Build excutable binary.

```sh
cargo build --examples --release
```

Output binary to *target\release\examples\capture-as-image.exe*.

## Usage

```text
Usage: capture-as-image.exe [OPTIONS] --output <FILE>

Options:
  -o, --output <FILE>   Specify output filename
  -f, --fullscreen      Specify if full screen capture taking
  -w, --window <TITLE>  Specify target window title
  -d, --desktop         Specify if desktop window taking
  -l, --list            List desktop window name
  -h, --help            Print help
  -V, --version         Print version
```

## Logging

Configure `RUST_LOG` environment variable.

```sh
set RUST_LOG=trace
capture-as-image.exe ...
```

## Notes

- Need to display window to the front of the screen if using Windows Terminal,
  because of clipping target area from full screen.
  So, must be specify `--fullscreen` or `--desktop` option.
  The workaround is to use the Windows Console Host (conhost.exe).
