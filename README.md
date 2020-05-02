<!--
SPDX-FileCopyrightText: Thomas Herzog

SPDX-License-Identifier: CC0-1.0
-->

# `reqenv`

`reqenv` is a tool to check/enforce environmental requirements. It allows to
throttle CPU speed, restrict execution of program to a limited amount of CPU
cores and enforce hard-limits on memory usage.

It uses `systemd-run` and Linux cgroups to work, so it only works on systems
that have those available.

## Installation

This project is written in Rust, so a working Rust installation is needed.

After cloning the repository, running the following `cargo` command will install
the program in the local `cargo` binary location, which should be in the
`$PATH`.

```sh
cargo install --path .
```

## Usage

By default, `systemd-run` will ask for authentication every time it is invoked.
To disable this the `--install-polkit-rule` option can be supplied to set up the
system so that authentication is not neede.

```sh
# needs root access to install the rule
sudo reqenv --install-polkit-rule
```

The command to be run **must** be separated from the `reqenv` call by `--`.

```sh
reqenv -- some-command with args
```

The options to supply are:
- `--cpu-speed <percent>`:
  CPU utilisation in percent. To run with 50% the speed, use `50`.
  Maximum value is `100`, minimum value is `0`.
- `--cpus <N>`:
  Number of CPU cores to use. To force a command to run on a single CPU core the
  value `1` can be used. Maximum value is the number of CPU cores on the system,
  minimum value is `1`.
- `--memory <amount (K/M/G/T)>`:
  Amount of virtual memory to allow the command to use. The postfix allows to
  specify the amount in Kilo-, Mega-, Giga- or Terabytes. No postfix means the
  amount is in bytes.

### Examples

```sh
# make sure that `my-command` finishes withing 10 seconds when running on a
# single core with 20% execution speed.
reqenv --cpus 1 --cpu-speed 20 -- timeout 10 my-command

# limit memory available to `ffmpeg`
reqenv --memory 500M -- ffmpeg ...
```

## License

All code is licensed under the [OSL-3.0](LICENSES/OSL-3.0.txt).

All files that are not properly copyrightable are in the public domain, using
the [CC0 license](LICENSES/CC0-1.0.txt).

This project aims to be [REUSE compliant](https://reuse.software/).