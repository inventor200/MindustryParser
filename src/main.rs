use std::io;
use std::io::prelude::*;
use std::fs::{OpenOptions, File};
use std::collections::VecDeque;
use std::cell::Cell;
use std::collections::HashMap;
use std::fmt;
use std::path;

const EMPTY_BUF_MSG: &str = "Unexpectedly reached the end of file.";

#[derive(Debug)]
enum AddressedValue {
    Bool(bool),
    Int(u32),
    Long(u64),
    Float(f32),
    String(Option<Box<String>>),
    Binary(Option<Box<Vec<u8>>>)
}

impl fmt::Display for AddressedValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressedValue::Bool(val) => write!(f, "{}", val),
            AddressedValue::Int(val) => write!(f, "{}", val),
            AddressedValue::Long(val) => write!(f, "{}", val),
            AddressedValue::Float(val) => write!(f, "{}", val),
            AddressedValue::String(val) => write!(f, "\"{}\"", val.as_ref().unwrap()),
            AddressedValue::Binary(val) => write!(f, "{:X?}", val.as_ref().unwrap())
        }
    }
}

#[derive(Debug)]
struct SettingsItem {
    address: usize,
    value: AddressedValue
}

impl fmt::Display for SettingsItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@[addr:{:X?}]", self.value, self.address)
    }
}

#[inline]
fn buf_pop(buffer: &mut VecDeque<u8>, offset: &mut Cell<usize>) -> u8 {
    let byte = buffer.pop_front()
        .expect(EMPTY_BUF_MSG);

    let new_offset = offset.get() + 1;
    offset.set(new_offset);

    byte
}

#[inline]
fn read_bool(buffer: &mut VecDeque<u8>, offset: &mut Cell<usize>) -> SettingsItem {
    let start_offset = offset.get();
    let byte = buf_pop(buffer, offset);

    SettingsItem {
        address: start_offset,
        value: AddressedValue::Bool(match byte {
            0 => false,
            1 => true,
            _ => panic!("Heck! Malformed boolean was struck!")
        })
    }
}

#[inline]
fn read_u32(buffer: &mut VecDeque<u8>, offset: &mut Cell<usize>) -> SettingsItem {
    let start_offset = offset.get();

    SettingsItem {
        address: start_offset,
        value: AddressedValue::Int(u32::from_be_bytes([
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
        ]))
    }
}

#[inline]
fn read_u64(buffer: &mut VecDeque<u8>, offset: &mut Cell<usize>) -> SettingsItem {
    let start_offset = offset.get();

    SettingsItem {
        address: start_offset,
        value: AddressedValue::Long(u64::from_be_bytes([
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
        ]))
    }
}

#[inline]
fn read_f32(buffer: &mut VecDeque<u8>, offset: &mut Cell<usize>) -> SettingsItem {
    let start_offset = offset.get();

    SettingsItem {
        address: start_offset,
        value: AddressedValue::Float(f32::from_be_bytes([
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
            buf_pop(buffer, offset),
        ]))
    }
}

#[inline]
fn read_binary(buffer: &mut VecDeque<u8>, offset: &mut Cell<usize>) -> SettingsItem {
    let len = u32::from_be_bytes([
        buf_pop(buffer, offset),
        buf_pop(buffer, offset),
        buf_pop(buffer, offset),
        buf_pop(buffer, offset),
    ]) as usize;
    
    let start_offset = offset.get();

    let mut series: Vec<u8> = Vec::with_capacity(len);

    for _i in 0..len {
        series.push(buf_pop(buffer, offset))
    }

    SettingsItem {
        address: start_offset,
        value: AddressedValue::Binary(Some(Box::new(series)))
    }
}

#[inline]
fn read_key(buffer: &mut VecDeque<u8>, offset: &mut Cell<usize>) -> String {
    let len = u16::from_be_bytes([
        buf_pop(buffer, offset),
        buf_pop(buffer, offset),
    ]) as usize;

    let mut series: Vec<u8> = Vec::with_capacity(len);

    for _i in 0..len {
        series.push(buf_pop(buffer, offset))
    }

    String::from_utf8(series)
        .expect("Heck! Encountered malformed string!")
}

#[inline]
fn read_string(buffer: &mut VecDeque<u8>, offset: &mut Cell<usize>) -> SettingsItem {
    let len = u16::from_be_bytes([
        buf_pop(buffer, offset),
        buf_pop(buffer, offset),
    ]) as usize;
    
    let start_offset = offset.get();

    let mut series: Vec<u8> = Vec::with_capacity(len);

    for _i in 0..len {
        series.push(buf_pop(buffer, offset))
    }

    let series = String::from_utf8(series)
        .expect("Heck! Encountered malformed string!");

    SettingsItem {
        address: start_offset,
        value: AddressedValue::String(Some(Box::new(series)))
    }
}

#[inline]
fn parse_bool(value: String) -> bool {
    let lower = value.to_lowercase();

    match lower.as_str() {
        "0" | "false" | "f" | "nil" | "no" | "off" | "inactive" => false,
        "1" | "true" | "t" | "yes" | "on" | "active" => true,
        _ => panic!("Bad bool: value")
    }
}

enum Operation {
    Read,
    Write
}

fn main() -> io::Result<()> {
    let mut args: VecDeque<String> = std::env::args().collect();
    args.pop_front(); // Ditch the command name

    let file_path_string = args.pop_front();

    if file_path_string.is_none() {
        println!("SYNTAX:");
        println!("mindustry_parser path/to/settings.bin --read <key> ...");
        println!("  Print the value and byte address of <key>");
        println!("");
        println!("mindustry_parser path/to/settings.bin --write <key> <value> ...");
        println!("  Set <key> to <value>");
        println!("");
        println!("mindustry_parser path/to/settings.bin --show-all");
        println!("  Prints all keys, values, and addresses found in the file");
        println!("");
        println!("mindustry_parser path/to/settings.bin --pretend --write <key> <value>");
        println!("  The --pretend flag modifies the settings in memory only, and does not modify the file on disk");
        println!("");
        println!("The above argument groups can be used multiple times in a sequence, as desired.");
        println!("  -r => alias for --read");
        println!("  -w => alias for --write");
        println!("");
        println!("Valid boolean values for \"true\": 1 true t yes on active");
        println!("Valid boolean values for \"false\": 0 false f nil no off inactive");
        std::process::exit(0);
    }

    let file_path_string = file_path_string.unwrap();

    let file_path = path::Path::new(&file_path_string).canonicalize()?;

    let mut show_all = false;
    let mut pretend = false;
    let mut is_dirty = false;

    for arg in &args {
        let lower = arg.to_lowercase();
        match lower.as_str() {
            "--show-all" => { show_all = true; },
            "--pretend" => { pretend = true; },
            _ => { }
        }
    }
    
    let mut buffer: Vec<u8> = Vec::new();

    {
        let mut file = File::open(&file_path)?;
        file.read_to_end(&mut buffer)?;
    }

    let mut buffer: VecDeque<u8> = buffer.into_iter().collect();
    let mut offset: Cell<usize> = Cell::new(0);

    let entry_count = u32::from_be_bytes([
        buf_pop(&mut buffer, &mut offset),
        buf_pop(&mut buffer, &mut offset),
        buf_pop(&mut buffer, &mut offset),
        buf_pop(&mut buffer, &mut offset),
    ]) as usize;

    let mut items: HashMap<String, SettingsItem> = HashMap::new();

    for _i in 0..entry_count {
        let key = read_key(&mut buffer, &mut offset);
        let type_id = buf_pop(&mut buffer, &mut offset);
        let item = match type_id {
            0 => read_bool(&mut buffer, &mut offset),
            1 => read_u32(&mut buffer, &mut offset),
            2 => read_u64(&mut buffer, &mut offset),
            3 => read_f32(&mut buffer, &mut offset),
            4 => read_string(&mut buffer, &mut offset),
            5 => read_binary(&mut buffer, &mut offset),
            _ => panic!("Heck! Unknown type_id: {type_id}")
        };

        if show_all {
            println!("{key}={item}");
        }
        
        items.insert(key, item);
    }

    let mut op: Option<Operation> = None;
    let mut op_key: Option<String> = None;

    for arg in args {
        if op.is_none() {
            let lower = arg.to_lowercase();
            match lower.as_str() {
                "--read" | "-r" => { op = Some(Operation::Read); },
                "--write" | "-w" => { op = Some(Operation::Write); },
                "--show-all" | "--pretend" => { continue; },
                _ => panic!("Unkown operation: {arg}")
            };
        }
        else if op_key.is_none() {
            if !items.contains_key(&arg) {
                panic!("Key not found: {arg}");
            }

            op_key = Some(arg);

            match op {
                Some(Operation::Read) => {
                    let key = op_key.take().unwrap();
                    let found_item = items.get(&key).unwrap();
                    print!("{key}={found_item},");
                    op = None;
                    op_key = None;
                },
                _ => { }
            }
        }
        else {
            // It only gets this far during a write op
            let key = op_key.take().unwrap();
            let found_item = items.get_mut(&key).unwrap();
            let value = &mut found_item.value;
            match value {
                AddressedValue::Bool(_) => *value = AddressedValue::Bool(parse_bool(arg)),
                AddressedValue::Int(_) => *value = AddressedValue::Int(arg.parse::<u32>().expect("Bad positive integer: {arg}")),
                AddressedValue::Long(_) => *value = AddressedValue::Long(arg.parse::<u64>().expect("Bad positive integer: {arg}")),
                AddressedValue::Float(_) => *value = AddressedValue::Float(arg.parse::<f32>().expect("Bad floating point: {arg}")),
                AddressedValue::String(_) => *value = AddressedValue::String(Some(Box::new(arg))),
                AddressedValue::Binary(_) => panic!("Sorry; this software lacks an implementation for modifying byte lists.")
            }
            op = None;
            op_key = None;
            is_dirty = true;
        }
    }

    println!("");

    if is_dirty && !pretend {
        let mut out_buffer: Vec<u8> = Vec::new();

        // Write item count to front
        let item_count = items.len() as u32;
        for b in item_count.to_be_bytes() {
            out_buffer.push(b);
        }

        for (key, value) in items {
            // Key to bytes
            write_string_to_buffer(key, &mut out_buffer);

            // Value to bytes
            let SettingsItem { address: _, value: setting_value } = value;

            match setting_value {
                AddressedValue::Bool(value) => { out_buffer.push(0); out_buffer.push(match value { true => 1u8, _ => 0u8 }); },
                AddressedValue::Int(value) => { out_buffer.push(1); out_buffer.extend(value.to_be_bytes()); },
                AddressedValue::Long(value) => { out_buffer.push(2); out_buffer.extend(value.to_be_bytes()); },
                AddressedValue::Float(value) => { out_buffer.push(3); out_buffer.extend(value.to_be_bytes()); },
                AddressedValue::String(mut value) => {
                    out_buffer.push(4);
                    // Unbox the string
                    let unboxed = value.take().unwrap().leak();
                    // Clone it
                    let cloned = String::from(&mut *unboxed);
                    // Put the reference safely back into a box, to prevent memory leaks
                    let reboxed = unsafe { Box::from_raw(unboxed) };
                    // Delete the box
                    drop(reboxed);
                    // Write clone to buffer
                    write_string_to_buffer(cloned, &mut out_buffer);
                },
                AddressedValue::Binary(mut value) => {
                    out_buffer.push(5);
                    // Take the box
                    let mut taken = value.take();
                    let taken = taken.as_mut().unwrap();
                    let len = taken.len() as u32;

                    // Write length
                    for b in len.to_be_bytes() {
                        out_buffer.push(b);
                    }
                    
                    // Drain the box
                    let drain = taken.drain(..);
                    out_buffer.extend(drain);
                }
            }
        }

        let mut file = OpenOptions::new().write(true).truncate(true).open(&file_path)?;
        file.write(&out_buffer[..])?;

        println!("The file has been modified.");
    }

    Ok(())
}

#[inline]
fn write_string_to_buffer(value: String, out_buffer: &mut Vec<u8>) {
    let key_len = value.len() as u16;
    for b in key_len.to_be_bytes() {
        out_buffer.push(b);
    }

    let mut key_bytes = value.into_bytes();
    let key_drain = key_bytes.drain(..);
    out_buffer.extend(key_drain);
}
