# agrf

Turn a stream of numbers into a compact braille unicode graph, right in the
terminal. Reads `int`/`float` from stdin (or positional args), draws a
sparkline / bar chart using braille dots (U+2800) — 8 pixels per character,
so a lot of signal fits in very little width.

```
⠀⢀⣴⣾⣿⣿⣶⣤⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣴⣾⣿⣿⣶⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠
⣴⣿⣿⣿⣿⣿⣿⣿⣿⣦⡀⠀⠀⠀⠀⠀⠀⠀⣠⣾⣿⣿⣿⣿⣿⣿⣿⣿⣦⡀⠀⠀⠀⠀⠀⠀⠀⣠⣾⣿
⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣶⣄⣀⣀⣀⣤⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣦⣄⣀⣀⣀⣴⣾⣿⣿⣿
```
```sh
awk 'BEGIN{for(i=0;i<80;i++) printf "%.2f\n", 50+45*sin(i/6)}' | agrf -w 40 -H 3
```

## Install

### As a Nix package (flake)

The flake exposes a `default` package — no clone needed:

```sh
nix run   github:rmrfus/agrf -- 3 7 2 9   # run without installing
nix build github:rmrfus/agrf              # ./result/bin/agrf
nix profile install github:rmrfus/agrf    # install into your profile
```

Pull it into a NixOS / home-manager flake as an input:

```nix
inputs.agrf.url = "github:rmrfus/agrf";
# then, where inputs + pkgs are in scope:
environment.systemPackages = [ inputs.agrf.packages.${pkgs.system}.default ];
# home-manager: home.packages = [ inputs.agrf.packages.${pkgs.system}.default ];
```

### With Cargo

On any host with a Rust toolchain, no Nix required (builds locally):

```sh
cargo install --git https://github.com/rmrfus/agrf --locked
```

### From a local checkout

```sh
direnv allow            # cargo/rustc from the flake devShell (or: nix develop)
cargo build --release   # ./target/release/agrf
cargo install --path .  # or drop it onto your PATH
```

## Usage

```
agrf [OPTIONS] [VALUES]...
```

Values come from positional args if given, otherwise from stdin
(whitespace/newline separated).

| Flag              | Default | Meaning                                                       |
|-------------------|---------|---------------------------------------------------------------|
| `[VALUES]...`     | —       | numbers to plot; if omitted, read from stdin                  |
| `-w, --width <N>` | `20`    | width in characters (holds `2*N` values)                      |
| `-H, --height <N>`| `1`     | height in characters (4 px per char)                          |
| `-m, --max <F>`   | auto    | Y-axis top; values above are clamped. Default: window max     |
| `-p, --point`     | off     | draw only the marker at each value, no fill (default: bars)   |

Behavior worth knowing:

- **One value = one pixel column.** Two columns per braille char, so a `W`-wide
  graph shows the last `2*W` values.
- **Fills left to right.** More values than fit → the last `2*W` are kept;
  fewer → they sit on the left, blank on the right.
- **Y scale is 0-based.** Auto max is the largest value in the window (or `--max`).
  Clustered-but-large values (e.g. ping in the 10–30ms range) all look tall
  because the floor is 0 — pin `--max`/a lower bound yourself if you want spread.
- **Negatives clamp to 0**, **non-numeric tokens become gaps** (empty column),
  and any positive value shows at least one pixel so small bars don't vanish.

## Examples

Real-world: ping latency as a sparkline.

```
⣄⣦⣶⣴⣤⣷⣤⣶⣀⣦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
```
```sh
ping -c 20 1.1.1.1 | grep -oP 'time=\K[0-9.]+' | agrf -w 20 -H 2
```

Noise from `/dev/urandom` (uniform bytes 0–255) as bars.

```
⣀⣠⣄⣀⣀⣠⣸⣰⣶⣤⣠⣆⣄⣧⣄⣇⣀⣰⣆⣰
```
```sh
head -c 40 /dev/urandom | od -An -tu1 -v | tr -s ' ' '\n' | grep . | agrf -w 20
```

A random walk, two chars tall.

```
⣀⣀⣠⣄⣄⣤⣤⣴⣿⣶⣾⣿⣿⣿⣿⣾⣶⣿⣿⣷⣿⣷⣾⣿⣿⣿⣷⣿⣷⣾
⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿
```
```sh
awk 'BEGIN{x=50;for(i=0;i<60;i++){x+=int(rand()*21)-10;if(x<0)x=0;if(x>100)x=100;print x}}' | agrf -w 30 -H 2
```

Point mode (`-p`) traces the curve instead of filling under it.

```
⠀⢀⠔⠊⠉⠉⠑⠢⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⡠⠔⠊⠉⠉⠒⠤⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⡠
⠊⠁⠀⠀⠀⠀⠀⠀⠈⠑⢄⠀⠀⠀⠀⠀⠀⢀⡠⠊⠀⠀⠀⠀⠀⠀⠀⠈⠢⢄⠀⠀⠀⠀⠀⠀⢀⠤⠊⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠑⠢⠤⠤⠤⠒⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠑⠢⠤⠤⠤⠒⠁⠀⠀⠀
```
```sh
awk 'BEGIN{for(i=0;i<80;i++) printf "%.2f\n", 50+45*sin(i/6)}' | agrf -w 40 -H 3 -p
```

Positional args, with a non-numeric token dropped to a gap (the `x`).

```
⣰⣸⢠⣇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
```
```sh
agrf 3 7 2 9 x 5 8 1
```

## License

MIT — see [LICENSE](LICENSE).
