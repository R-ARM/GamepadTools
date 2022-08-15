#![feature(ptr_metadata)]
use gamepad_gui::ToolkitBuilder;
use std::ffi::c_void;
use efivar::efi::{VariableName, VariableFlags};

use uefi::proto::device_path::DevicePath;
use uefi::proto::device_path::DevicePathInstance;

use uefi::proto::device_path::DevicePathNode;
use uefi::proto::device_path::DevicePathHeader;
use std::str::FromStr;
use std::io::BufRead;
use regex::Regex;
use core::{mem, ptr};

fn char16_to_string(buf: &[u8]) -> (String, usize) {
    let mut iter = buf.iter();
    let mut out: Vec<u16> = Vec::new();
    let mut i: usize = 0;

    loop {
        i += 2;
        if let (Some(lower), Some(upper)) = (iter.next(), iter.next()) {
            let tmp = (*upper as u16) << 8 | *lower as u16;
            if tmp == '\0' as u16 {
                break;
            } else {
                out.push(tmp);
            }
        } else {
            break;
        }
    }

    (std::char::decode_utf16(out)
        .map(|r| r.unwrap_or(' '))
        .map(|r| if r.is_ascii() {r} else {' '})
        .collect::<String>(), i)
}

#[derive(Debug)]
struct Entry {
    id: u16,
    id_string: String,
    description: String,
    path: Vec<String>,
}

impl Entry {
    fn new(var: &str, buf: &[u8]) -> Self {
        let (description, end) = char16_to_string(&buf[(32+16)/8..]);
        let desc_end_offset = (32+16)/8 + end;
        let device_path: &DevicePath = unsafe {
            std::mem::transmute(&buf[desc_end_offset..])
        };

        let mut out_path: Vec<String> = Vec::new();
        for node in device_path.node_iter() {
            if let Some(file) = node.as_file_path_media_device_path() {
                let path = file.path_name().to_cstring16().unwrap();
                out_path.push(path.to_string());
            } else {
                out_path.push(format!("{:?}", node.device_type()));
            }
        }

        let boot_id = var.to_string().split_off(4);
        let id = if let Ok(tmp) = u16::from_str_radix(&boot_id, 16) {
            tmp
        } else {
            0
        };

        Entry {
            id,
            id_string: boot_id,
            description,
            path: out_path,
        }
    }
}

impl std::fmt::Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let mut path = String::new();
        for tmp in &self.path {
            path.push_str(&tmp);
            path.push_str(" ");
        }
        path.pop(); // remove trailing space
        write!(f, "Boot{}: {}, at: '{}'", self.id_string, self.description, path)
    }
}

fn main() -> Result<(), efivar::Error> {
    /*let mut tk = ToolkitBuilder::new("Testing")
        .tab("whatever")
        .button("idk")
        .button("stuff")
        .tab("another tab")
        .button("i am a button")
        .tab("tab from vec<str>")
        .buttons_vec(names_str)
        .build();

    while tk.tick() {
        //println!("{:#?}", tk);
    }*/

    let boot_xxxx = Regex::new(r"^Boot\d\d\d\d$").unwrap();
    let mut manager = efivar::system();
    let mut buf: [u8; 256] = [0u8; 256];
    //let mut options: Vec<DevicePath> = Vec::new();

    for var in manager.get_var_names().expect("asdf") {
        if boot_xxxx.is_match(var.variable()) {
            match manager.read(&var, &mut buf)  {
                Ok((size, ..)) => {
                    let tmp = Entry::new(var.variable(), &buf);
                    println!("{}", tmp);
                    
                }
                Err(e) => eprintln!("{}", e),
            }
        }
    }
    Ok(())
}
