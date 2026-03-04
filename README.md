# Texpro

> A simple CLI text-processing utility.

## Overview
Texpro is a regex-based text processor for the CLI, with several subcommands that can analyze text for useful information, in addition to simple, automatic edition features for making some changes on the fly.

## Features
- **Pattern Search:** This is the most fundamental feature. It lets you find a specific regex pattern inside one text file.
- **File Comparison:** Allows for comparing two given text files and find how different they are in terms of percentage — useful for checking whether there are changes or updates in different copies of a file.
- **Heuristic Fail-Safe:** Inspired by tools like the `file` CLI utility, it implements its own internal algorithm for identifying text files, skipping those that aren't plain text.

## Build
To build the tool from source, just clone the repo:

```
git clone https://github.com/durakitus/texpro.git
cd texpro
cargo build --release
```

## Usage
This tool is based on subcommands. Some of the most notable are:
- `texpro search <file> <pattern>` — the command that lets you search for a pattern inside a file.
- `texpro directory <path_to_directory> <pattern>` — this one searches for the pattern across all text files it can find inside a directory/folder. Good for finding which file talks about what you're looking for.
- `texpro compare <first_file> <second_file>` — this can compare two files and show the percentage of difference between them.

Run `texpro help` — or `cargo run -- help` inside the project folder if you haven't installed it to your `PATH` — for more usage details, if you decide to build it. Additionally, run `texpro help <command>` or `cargo run -- help <command>` to get help for a specific subcommand you want.
