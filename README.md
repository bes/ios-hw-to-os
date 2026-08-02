# iOS Hardware to Operating System mapping as a small JSON file

*Mappings from iOS / iPadOS hardware string to latest supported iOS / iPadOS versions*

**CONTRIBUTIONS WELCOME**

An incomplete mapping of hardware string to OS version mappings
can be found in the [mappings.json](./mappings.json) file.

I have mainly focused on iOS / iPadOS versions >= 15 because that's
what I care about.

If any of your favorite mappings are missing, feel free to submit a PR.

The main data source is [AppleDB](https://appledb.dev), via the [API](https://github.com/littlebyteorg/appledb/blob/main/API.md).

## How to use the mapping

Either just copy [mappings.json](./mappings.json) or use the Swift Package Manager
[package ios-hw-to-os-spm](https://github.com/bes/ios-hw-to-os-spm).
The page for the Swift package contains documentation for the Swift API and
usage instructions.

## Contributions

Please contribute to this repository to update the YAML and JSON mappings,
and to [ios-hw-to-os-spm](https://github.com/bes/ios-hw-to-os-spm) repository to
improve the Swift API.

### How to contribute

You must have [Rust](https://rustup.rs/) installed.

Pull the latest hardware and OS support data from AppleDB into
[source.yaml](./source.yaml):

```shell
cargo run --quiet -- update-source
```

Then regenerate [mappings.json](./mappings.json) from it:

```shell
cargo run --quiet -- write-mappings
```
