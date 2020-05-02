// SPDX-FileCopyrightText: Thomas Herzog
//
// SPDX-License-Identifier: OSL-3.0

use std::convert::TryFrom as _;
use std::os::unix::process::CommandExt as _;
use std::process::Command;

use clap::{value_t, App, Arg};

use reqenv::{build_command, Options, SizePostfix};

fn main() {
    let num_cpus = u8::try_from(num_cpus::get()).unwrap_or(u8::max_value());

    let matches = App::new("reqenv")
        .version("0.1.0")
        .author("Thomas Herzog")
        .arg(
            Arg::with_name("install-polkit-rule")
                .long("install-polkit-rule")
                .help("Install a polkit rule to avoid authentication (requires root)")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("cpus")
                .long("cpus")
                .value_name("N")
                .help("Number of CPU cores to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cpu-speed")
                .long("cpu-speed")
                .value_name("percent")
                .help("CPU utilisation in percent (integer)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("memory")
                .long("memory")
                .value_name("amount (K/M/G/T)")
                .help("Amount of virtual memory to provide")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("command")
                .multiple(true)
                .help("Command to run")
                .last(true),
        )
        .get_matches();

    fn usage_exit(param: &str, matches: clap::ArgMatches<'_>) -> ! {
        let usage = matches.usage();
        eprint!("Invalid parameter `{}`. ", param);
        eprintln!("Use `--help` to get an overview of the options");
        eprintln!("{}", usage);
        std::process::exit(1)
    };

    if matches.is_present("install-polkit-rule") {
        install_polkit_rule();
    }

    let cpus_raw = matches.value_of("cpus");
    let cpu_speed = value_t!(matches, "cpu-speed", u8).ok().map(|s| s.min(100));
    let memory_raw = matches.value_of("memory");
    let command = matches.values_of_lossy("command");

    let cpus = if let Some(raw) = cpus_raw {
        if let Ok(res) = raw.parse::<u8>() {
            Some(res.max(1).min(num_cpus))
        } else {
            usage_exit("--cpus", matches)
        }
    } else {
        None
    };

    let memory = if let Some(raw) = memory_raw {
        if let Ok(res) = parse_memory(raw) {
            Some(res)
        } else {
            usage_exit("--memory", matches)
        }
    } else {
        None
    };

    let command = if let Some(raw) = command {
        raw
    } else {
        usage_exit("<<command>>", matches)
    };

    let options = Options {
        cpus,
        cpu_speed,
        memory,
    };

    let command = build_command(num_cpus, options, &command);

    // the `.exec()` replaces the process memory with the new command's one
    // so this should not return, and if it does there was an error.
    let error = Command::new(command.program).args(command.arguments).exec();

    eprintln!("Unable to spawn program: {}", error);
    std::process::exit(1);
}

const POLKIT_RULE: &str = include_str!("../../assets/99-reqenv.rules");

fn install_polkit_rule() -> ! {
    const PATH: &str = "/etc/polkit-1/rules.d/99-reqenv.rules";

    std::fs::write(PATH, POLKIT_RULE).expect("Unable to write file. Are you root/sudo?");
    std::process::exit(0)
}

fn parse_memory(s: &str) -> Result<(u32, SizePostfix), std::num::ParseIntError> {
    if s.ends_with('K') {
        let s = s.trim_end_matches('K');
        let num = s.parse()?;
        Ok((num, SizePostfix::Kilo))
    } else if s.ends_with('M') {
        let s = s.trim_end_matches('M');
        let num = s.parse()?;
        Ok((num, SizePostfix::Mega))
    } else if s.ends_with('G') {
        let s = s.trim_end_matches('G');
        let num = s.parse()?;
        Ok((num, SizePostfix::Giga))
    } else if s.ends_with('T') {
        let s = s.trim_end_matches('T');
        let num = s.parse()?;
        Ok((num, SizePostfix::Tera))
    } else {
        let num = s.parse()?;
        Ok((num, SizePostfix::None))
    }
}
