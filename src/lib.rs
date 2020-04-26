// SPDX-FileCopyrightText: Thomas Herzog
//
// SPDX-License-Identifier: OSL-3.0

use contracts::*;

#[derive(Copy, Clone, Debug)]
enum SizePostfix {
	None,
	Kilo,
	Mega,
	Giga,
	Tera,
}

impl std::string::ToString for SizePostfix {
	fn to_string(&self) -> String {
		let lit = match self {
			SizePostfix::None => "",
			SizePostfix::Kilo => "K",
			SizePostfix::Mega => "M",
			SizePostfix::Giga => "G",
			SizePostfix::Tera => "T",
		};
		lit.to_string()
	}
}

#[derive(Copy, Clone, Debug)]
struct Options {
	cpus: Option<u8>,
	cpu_speed: Option<u8>,
	memory: Option<(u32, SizePostfix)>,
}

impl Options {
	fn invariant(&self) -> bool {
		assert!(
			self.cpus.unwrap_or(1) > 0,
			"CPU count must be non-zero",
		);
		assert!(
			self.cpu_speed.unwrap_or(100) <= 100,
			"CPU speed must not exceed 100%",
		);
		true
	}
}

/// The command that should be executed by the system to apply the restrictions
///
/// See the [`Command`][cmd] type from the standard library.
///
/// [cmd]: https://doc.rust-lang.org/std/process/struct.Command.html
#[derive(Debug, Clone)]
pub struct Command {
	/// The program to execute.
	pub program: String,
	/// The arguments of the program.
	pub arguments: Vec<String>,
}

// This contract can't be enabled because it generates invalid code with the
// `impl Trait` return type used here.
//
// #[pre(!command.is_empty(), "must provide a command to execute")]
/// Build a [`Command`](struct.Command.html) from command line arguments.
///
/// - `num_cpus` are the number of CPUs available on the system.
///   This is needed to appropriately scale the cpu quota appropriately.
/// - `args` are the options of the current program that affect the restrictions
///   on the program that should be executed.
/// - `command` are the program-path and arguments of the program that should be
///   executed.
///
/// Errors can occur when invalid `args` are supplied.
pub fn command_from_args(
	num_cpus: u8,
	args: Vec<std::ffi::OsString>,
	command: &[String],
) -> Result<Command, impl std::fmt::Display + std::fmt::Debug> {
	
	macro_rules! opt {
		($e:expr) => {
			match $e {
				Result::Ok(val) => val,
				Result::Err(err) => return Err(err)
			}
		};
	};
	
	let mut args = pico_args::Arguments::from_vec(args);
	let cpus: Option<u8> = opt!(args.opt_value_from_str("--cpus"));
	let cpu_speed: Option<u8> = opt!(args.opt_value_from_str("--cpu-speed"));
	let memory = opt!(args.opt_value_from_fn("--memory", parse_memory));
	
	opt!(args.finish());
	
	let opts = Options {
		cpus: cpus.map(|c| c.max(1).min(num_cpus)),
		cpu_speed: cpu_speed.map(|s| s.min(100)),
		memory,
	};
	
	Ok(build_command(num_cpus, opts, &command))
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

#[pre(num_cpus > 0)]
#[pre(opts.invariant())]
#[pre(!command.is_empty(), "must provide a command to execute")]
#[post(ret.arguments.ends_with(command), "command must be preserved")]
// #[post(opts.cpus.is_some() ==> ret.arguments.contains("taskset"))]
// #[post(opts.cpus.or(opts.cpu_speed).is_some()
// 	==> ret.arguments.first(|e| e.starts_with("CPUQuota")).is_some()
// )]
// #[post(opts.memory.is_some()
// 	==> ret.arguments.first(|e| e.starts_with("MemoryMax")).is_some()
// )]
fn build_command(
	num_cpus: u8,
	opts: Options,
	command: &[String],
) -> Command {
	let program = "systemd-run".to_string();
	let mut args = vec!["--quiet".to_string(), "--scope".to_string()];
	
	if let Some((size, postfix)) = opts.memory {
		args.push("-p".to_string());
		args.push(format!("MemoryMax={}{}", size, postfix.to_string()));
	}
	
	let speed = match (opts.cpus, opts.cpu_speed) {
		(None, None) => None,
		(None, Some(speed)) => {
			let cpus = u16::from(num_cpus);
			let total_speed: u16 = cpus * u16::from(speed);
			Some(total_speed)
		},
		(Some(cpus), None) => {
			let cpus = u16::from(cpus.min(num_cpus));
			let speed: u16 = 100;
			let total_speed = cpus * speed;
			Some(total_speed)
		},
		(Some(cpus), Some(speed)) => {
			let cpus = u16::from(cpus.min(num_cpus));
			let speed = u16::from(speed);
			let total_speed = cpus * speed;
			Some(total_speed)
		},
	};
	
	if let Some(speed) = speed {
		args.push("-p".to_string());
		args.push(format!("CPUQuota={}%", speed));
	}
	
	if let Some(cpus) = opts.cpus {
		let cpus = cpus.min(num_cpus);
		
		let cpu_list: String = select_cpus(num_cpus, cpus)
			.map(|n| n.to_string())
			.collect::<Vec<_>>()
			.join(",");
		
		args.push("taskset".to_string());
		args.push("-c".to_string());
		args.push(cpu_list);
	}
	
	args.extend_from_slice(command);
	
	Command { program, arguments: args }
}


#[pre(num_cpus > 0, used > 0)]
#[pre(num_cpus >= used, "must have enough cores available")]
#[post(ret.end == num_cpus)]
#[post((ret.end - ret.start) == used, "will return a range with enough cores")]
fn select_cpus(num_cpus: u8, used: u8) -> core::ops::Range<u8> {
	let first = num_cpus - used;
	first..num_cpus
}
