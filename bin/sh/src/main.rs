#![no_std]
#![no_main]

extern crate alloc;
extern crate ruxpin_app;

use core::str;

use alloc::vec;
use alloc::vec::Vec;

use ruxpin_api::{STDIN_FILENO, STDOUT_FILENO, print, println, open, dup2, fork, exec, read, waitpid};
use ruxpin_types::{OpenFlags, FileAccess};


struct Command<'a> {
    words: Vec<&'a str>,
    input: Option<&'a str>,
    output: Option<&'a str>,
    append: bool,
}

fn get_next_word<'a>(input: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    for i in 0..input.len() {
        if input[i] == ' ' as u8 || input[i] == '\n' as u8 || input[i] == '\r' as u8 {
            return (&input[..i], &input[i + 1..]);
        }
    }
    return (input, &input[input.len()..]);
}

fn parse_words<'a>(input: &'a [u8]) -> Vec<&'a str> {
    let mut next = input;
    let mut words = vec![];

    loop {
        if next == b"" {
            break words;
        }

        let (word, remain) = get_next_word(next);
        words.push(str::from_utf8(word).unwrap());

        next = remain;
    }
}

fn parse_command<'a>(input: &'a [u8]) -> Vec<Command> {
    let mut commands = vec![];

    for (i, ch) in str::from_utf8(input).unwrap().chars().enumerate() {
        match ch {
            '<' => {
                commands.push(Command {
                    words: parse_words(&input[..i]),
                    input: Some(str::from_utf8(&input[i..]).unwrap().trim()),
                    output: None,
                    append: false,
                });

                return commands;
            },
            '>' => {
                commands.push(Command {
                    words: parse_words(&input[..i]),
                    input: None,
                    output: Some(str::from_utf8(&input[i + 1..]).unwrap().trim()),
                    append: false,
                });

                return commands;
            },
            _ => { },
        }
    }

    commands.push(Command {
        words: parse_words(input),
        input: None,
        output: None,
        append: false,
    });

    commands
}

fn substitute_path<'a>(fullpath: &'a mut [u8], path: &str, command: &str) -> &'a str {
    let path_len = path.as_bytes().len();
    let command_len = command.as_bytes().len();

    (&mut fullpath[..path_len]).copy_from_slice(path.as_bytes());
    (&mut fullpath[path_len..path_len + command_len]).copy_from_slice(command.as_bytes());
    str::from_utf8(&fullpath[..path_len + command_len]).unwrap()
}


#[no_mangle]
pub fn main() {
    println!("\nStarting shell...");

    let mut data = [0; 256];
    loop {
        print!("\n% ");
        let length = read(STDIN_FILENO, &mut data).unwrap();
        let commands = parse_command(&data[..length]);

        if commands.len() == 0 || commands[0].words.len() == 0 || commands[0].words[0] == "" {
            continue;
        }

        if commands[0].words[0] == "exit" {
            break;
        }

        for mut command in commands {
            let mut fullpath = [0; 256];
            command.words[0] = substitute_path(&mut fullpath, "/bin/", command.words[0]);

            let pid = fork().unwrap();
            if pid == 0 {
                // TODO open the in/out files and use dup2() syscall
                if let Some(name) = command.input {
                    let fd = open(name, OpenFlags::ReadOnly, FileAccess::DefaultFile).unwrap();
                    dup2(fd, STDIN_FILENO).unwrap();
                }

                if let Some(name) = command.output {
                    let mut flags = OpenFlags::WriteOnly;
                    if command.append {
                        flags = flags.plus(OpenFlags::Append);
                    } else {
                        flags = flags.plus(OpenFlags::Create);
                    }
                    let fd = open(name, flags, FileAccess::DefaultFile).unwrap();
                    dup2(fd, STDOUT_FILENO).unwrap();
                }

                exec(command.words[0], &command.words[..], &[]);
            } else {
                let mut status = 0;
                let result = waitpid(pid, &mut status, 0);
                match result {
                    Ok(pid) => { println!("pid {} exited with {}", pid, status); },
                    Err(err) => { println!("Error while waiting for process: {:?}", err); },
                }
            }
        }
    }

    println!("shell exited");
}

