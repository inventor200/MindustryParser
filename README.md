# Mindustry Parser CLI Utility

A utility for reading and writing the `settings.bin` file for Mindustry.

## Inspiration

I play Mindustry across two computers, but each one has wildly-different hardware specs, and it was driving me crazy how syncing my game progress would wipe the settings from one computer or the other.

Now, with this utility, I can modify the settings file to tweak the config for the computer receiving the save data.

## How To Install

First, make sure you have rust and cargo installed.

```bash
git clone https://github.com/inventor200/MindustryParser.git
cd MindustryParser
cargo build --release
```

The executable will be found in `./target/release/mindustry_parser`.

Sorry, I don't have any pre-built binaries available for download at this time.

## How To Use

The following examples assume the settings file is located at `~/Mindustry/settings.bin`.

### Examination

```bash
mindustry_parser ~/Mindustry/settings.bin --show-all
```

List all the settings in the file, including names, values, and byte addresses.

### Precise Examination

```bash
mindustry_parser ~/Mindustry/settings.bin --read fullscreen --read linear
```

Lists any specific settings by name. In the above example, we are fetching the current values for `fullscreen` and `linear`.

You can also use `-r` for short.

### Modification

```bash
mindustry_parser ~/Mindustry/settings.bin --write fullscreen false
```

The above will modify the `fullscreen` value to be `false`.

You can also use `-w` for short.

### Predicted Modification

```bash
mindustry_parser ~/Mindustry/settings.bin --pretend -w fullscreen false
```

By adding the `--pretend` flag, the settings are only modified in RAM; the settings file on the disk remains unchanged.
