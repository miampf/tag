use std::{path::Path, process::Command};

use colored::Colorize;

/// `execute_command_on_file` executes a command on a given #FILE#.
pub fn execute_command_on_file(path: &Path, command: &str) -> String {
    let command = command.replace("#FILE#", path.to_str().unwrap());

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").arg(command.clone()).output()
    } else {
        Command::new("bash").arg("-c").arg(command.clone()).output()
    };

    if let Err(e) = &output {
        eprintln!(
            "{} Wasn't able to execute command {}: {}",
            "[ERROR]".red().bold(),
            command.blue().underline(),
            e.to_string().red()
        );
    }

    let output = output.unwrap();
    let output_string = std::str::from_utf8(output.stdout.as_slice());

    if let Err(e) = &output_string {
        eprintln!(
            "{} Failed to get output from command {}: {}",
            "[ERROR]".red().bold(),
            command.blue().underline(),
            e.to_string().red()
        );
    }

    output_string.unwrap().to_string()
}

/// `execute_filter_command_on_file` executes a command on a given #FILE# and returns
/// true if the command ran successfully.
pub fn execute_filter_command_on_file(path: &Path, command: &str) -> bool {
    let command = command.replace("#FILE#", path.to_str().unwrap());

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").arg(command.clone()).output()
    } else {
        Command::new("bash").arg("-c").arg(command.clone()).output()
    };

    if let Err(e) = &output {
        eprintln!(
            "{} Wasn't able to execute command {}: {}",
            "[ERROR]".red().bold(),
            command.blue().underline(),
            e.to_string().red()
        );
    }

    output.unwrap().status.success()
}
