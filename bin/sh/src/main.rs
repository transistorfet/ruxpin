#![no_std]
#![no_main]

use core::str;

extern crate ruxpin_app;

use ruxpin_api::{print, println, fork, exec, read, waitpid};
use ruxpin_types::FileDesc;


fn get_next_word<'a>(input: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    for i in 0..input.len() {
        if input[i] == ' ' as u8 || input[i] == '\n' as u8 || input[i] == '\r' as u8 {
            return (&input[..i], &input[i + 1..]);
        }
    }
    return (input, &input[input.len()..]);
}

fn parse_words<'a>(input: &'a [u8]) -> (usize, [&'a str; 20]) {
    let mut i = 0;
    let mut next = input;
    let mut words = [""; 20];

    loop {
        if next == b"" {
            break (i, words);
        }

        let (word, remain) = get_next_word(next);
        words[i] = str::from_utf8(word).unwrap();

        i += 1;
        next = remain;
    }
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
    let mut fullpath = [0; 256];
    loop {
        //let length = read_input(&mut data).unwrap();
        print!("\n% ");
        let length = read(FileDesc(0), &mut data).unwrap();
        let (argc, mut words) = parse_words(&data[..length]);

        if words[0] == "exit" {
            break;
        }

        if words[0] != "" {
            words[0] = substitute_path(&mut fullpath, "/bin/", words[0]);

            let pid = fork().unwrap();
            if pid == 0 {
                exec(words[0], &words[..argc], &[]);
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

