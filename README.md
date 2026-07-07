# KMUTNB Physics Lab Plotter

A terminal UI graph plotter for physics lab data. It works like a small lab notebook plus graph-paper viewer inside the terminal: enter x-y values, edit the table, and see the plot, best-fit equation, slope, intercept, and R^2 update immediately.

The app stores data as plain CSV. No database is required.

## Install And Run

On NixOS or any system with Nix:

```sh
nix-shell
cargo run -- samples/ohms_law.csv
```

Without entering a shell:

```sh
nix-shell --run "cargo run -- samples/ohms_law.csv"
```

A `flake.nix` is also included for projects that prefer flakes. In a Git worktree, Nix flakes only see files tracked by Git, so use `nix-shell` while working with newly created untracked files.

With a normal Rust toolchain:

```sh
cargo run -- samples/ohms_law.csv
```

Start without a file to create a blank dataset:

```sh
cargo run
```

## Keybindings

| Key | Action |
| --- | --- |
| `h` / `j` / `k` / `l` | Move table cursor, or move graph crosshair when graph focus and crosshair are enabled |
| `i` | Edit current cell |
| `a` | Add row |
| `A` | Add column |
| `d` | Delete selected row |
| `r` | Rename selected column |
| `g` | Focus graph panel |
| `t` | Focus table panel |
| `s` | Save CSV |
| `o` | Open/load CSV |
| `f` | Toggle best-fit line |
| `G` | Toggle graph paper mode |
| `S` | Open manual scale dialog |
| `u` | Return to auto scale from the scale dialog |
| `c` | Toggle crosshair cursor |
| `?` | Help screen |
| `Esc` | Cancel current mode / return to normal mode |
| `q` | Quit |

## Example Workflow

1. Run `cargo run -- samples/ohms_law.csv`.
2. Use `j`/`k` to move through rows and `h`/`l` to move between columns.
3. Press `i`, type a value, then press `Enter`.
4. Press `a` to add another measurement row.
5. Press `G` for graph-paper mode.
6. Press `S` to set manual graph-paper scale, such as fixed min/max and major divisions.
7. Press `f` to toggle the best-fit line.
8. Press `g`, then `c`, then move the crosshair with `h`/`j`/`k`/`l` to read estimated graph coordinates.
9. Press `s` to save.

## CSV Format

The first row contains column names. The default columns are `x` and `y`; extra columns are preserved.

```csv
x,y
1,2.1
2,4.0
3,6.2
4,8.1
```

Invalid or blank values in `x` or `y` are kept in the table but excluded from plotting and regression. Invalid x-y cells are highlighted in the table.

## Graph Paper View

Graph paper mode prioritizes a clean grid for physics lab work:

- minor grid divisions
- stronger major grid divisions
- stronger x/y axes
- numeric labels at major divisions
- selected point marker
- optional best-fit line extended across the visible graph
- manual scale fields for x min, x max, y min, y max, x major division, and y major division

Unicode drawing characters are used when the terminal reports UTF-8 support. Otherwise the plot falls back to ASCII characters.

## Tests

```sh
nix-shell --run "cargo test"
```

The test suite covers CSV loading/saving, linear regression, invalid value handling, and row editing operations.
