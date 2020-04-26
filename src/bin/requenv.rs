// SPDX-FileCopyrightText: Thomas Herzog
//
// SPDX-License-Identifier: OSL-3.0

use std::os::unix::process::CommandExt as _;
use std::process::Command;

use reqenv::command_from_args;

fn main() {
	let args: Vec<_> = std::env::args_os().skip(1).collect();
	
	let separator_idx_res: Option<usize> = args
		.iter()
		.enumerate()
		.find_map(|(i, s)| if s == "--" { Some(i) } else { None } );
	
	let separator_idx = separator_idx_res.unwrap_or(0);
	
	let has_dashes = separator_idx_res.is_some();
	let command_skip = if has_dashes { 1 } else { 0 };
	
	let (options, command) = args.split_at(separator_idx);
	
	let command: Vec<String> = command
		.into_iter()
		.skip(command_skip)
		.map(|s| s.clone().into_string().unwrap())
		.collect();
	
	if command.is_empty() {
		eprintln!("ERROR: Must provide a command");
		std::process::exit(1);
	}
	
	let num_cpus = num_cpus::get();
	
	match command_from_args(num_cpus as u8, options.to_vec(), &command) {
		Ok(command) => {
			let error = Command::new(command.program)
				.args(command.arguments)
				.exec();
			
			eprintln!("Unable to spawn program: {}", error);
			std::process::exit(1);
		},
		Err(err) => {
			eprintln!("Error: {}", err);
			std::process::exit(1);
		},
	}
}
