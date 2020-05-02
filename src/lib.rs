// SPDX-FileCopyrightText: Thomas Herzog
//
// SPDX-License-Identifier: OSL-3.0

use contracts::*;

#[derive(Copy, Clone, Debug)]
pub enum SizePostfix {
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
pub struct Options {
	pub cpus: Option<u8>,
	pub cpu_speed: Option<u8>,
	pub memory: Option<(u32, SizePostfix)>,
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

#[pre(num_cpus > 0)]
#[pre(opts.invariant())]
#[pre(!command.is_empty(), "must provide a command to execute")]
#[post(ret.arguments.ends_with(command), "command must be preserved")]
#[post(opts.cpus.is_some() ==> ret.arguments.contains(&"taskset".into()))]
#[post(opts.cpus.or(opts.cpu_speed).is_some()
	==> ret.arguments.iter().find(|e| e.starts_with("CPUQuota")).is_some()
)]
#[post(opts.memory.is_some()
	==> ret.arguments.iter().find(|e| e.starts_with("MemoryMax")).is_some()
)]
pub fn build_command(
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
