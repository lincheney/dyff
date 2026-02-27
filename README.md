# dyff

Diffs that look nice to me.

Get diffs that look like:

<img width="393" height="156" alt="Screenshot_2026-02-27_17-39-43" src="https://github.com/user-attachments/assets/46a674f6-0263-496c-839e-ba88ea32efca" />

rather than:

<img width="286" height="218" alt="Screenshot_2026-02-27_17-39-59" src="https://github.com/user-attachments/assets/8ad5ea39-3089-49ee-8df2-d24ef6e5195e" />

> Because of the way this works, what is displayed is often not a minimal diff.

## Installation

You will need to compile this yourself.
You can install using:
```bash
cargo install --git https://github.com/lincheney/dyff
```

or clone this repo and run `cargo build --release`

## Usage

`dyff` can be used similarly to normal `diff`, by running `dyff FILE1 FILE2`.

`dyff` also acts as a filter; you pipe diffs into stdin and it prints formatted output: e.g. `git diff | dyff`

### Using with git

`dyff` can work with git fine most of the time, but needs to have inlining turned off for interactive use (e.g. `git add -p`).
```
[interactive]
	diffFilter = dyff --color=always --exact || true
```
