# iOS / iPadOS hardware string to latest OS version mappings

*CONTRIBUTIONS WELCOME*

An incomplete mapping of hardware string to OS version mappings
can be found in the [mappings.json](./mappings.json) file.

I have mainly focused on iOS / iPadOS versions >= 15 because that's
what I care about.

If any of your favorite mappings are missing, feel free to submit a PR.

Sources:
* https://en.wikipedia.org/wiki/List_of_iPhone_models
* https://en.wikipedia.org/wiki/List_of_iPad_models

## How to contribute

You must have [Rust](https://rustup.rs/) installed.

Update the file [source.yaml](./source.yaml), then use the following command to
update the [mappings.json](./mappings.json) file:

```shell
cargo run --quiet > mappings.json
```
