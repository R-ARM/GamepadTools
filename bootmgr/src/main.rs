use gamepad_gui::ToolkitBuilder;
use efivar::efi::{VariableName, VariableFlags};
use std::str::FromStr;
use std::io::BufRead;
use regex::Regex;

static DESCRIPTION_OFFSET_BYTES: usize = (32+16)/8;

#[derive(Debug)]
struct Entry {
    id: u16,
    description: String,
    path: Vec<String>,
}

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


impl Entry {
    fn new(var: &str, buf: &[u8]) -> Option<Entry> {
        let (description, end) = char16_to_string(&buf[DESCRIPTION_OFFSET_BYTES..]);
        let desc_end_offset = DESCRIPTION_OFFSET_BYTES + end;

        let mut path = Vec::<String>::new();
        let pathlist: &[u8] = &buf[desc_end_offset..];
        let cur_start: usize = 0;
        let mut cur: usize = cur_start;

        loop {
            let cur_type = pathlist[cur];
            let subtype = pathlist[cur+1];
            let len = pathlist[cur+2];

            //println!("type {:X?} subtype {:X?} len {:X?}", cur_type, subtype, len);
            match cur_type {
                0x03 => { // Messaging Device Type
                    match subtype {
                        01 => path.push("ATAPI".to_string()),
                        02 => path.push("SCSI".to_string()),
                        03 | 21 => path.push("Fibre Channel".to_string()),
                        04 => path.push("1394".to_string()),
                        05 | 15 | 16 => path.push("USB".to_string()),
                        06 => path.push("I2O".to_string()),
                        09 | 11 | 12 | 13 | 20 | 28 | 31
                            => path.push("Network".to_string()),
                        10 => path.push("Vendor specific".to_string()),
                        17 => (), // don't care
                        18 => path.push("SATA".to_string()),
                        19 => path.push("iSCSI".to_string()),
                        22 => path.push("SAS".to_string()),
                        23 => path.push("NVMe".to_string()),
                        24 => path.push("URI".to_string()),
                        25 => path.push("UFS".to_string()),
                        26 => path.push("SD Card".to_string()),
                        27 | 30 => path.push("Bluetooth".to_string()),
                        29 => path.push("eMMC".to_string()),
                        32 => path.push("NVDIMM".to_string()),
                        _ => unreachable!("Corrupted data or future spec"),
                    }
                }
                0x04 => { // Media Device Path
                    match subtype {
                        01 => path.push("Hard Drive".to_string()),
                        02 => path.push("CD-ROM".to_string()),
                        03 => todo!("Vendor-defined Media Device Path subtype isn't handled!"),
                        04 => {
                            let (tmp, _) = char16_to_string(&pathlist[cur+4..]);
                            path.push(tmp);
                        },
                        06 | 07 => path.push("UEFI PI".to_string()),
                        _ => {
                            todo!("{} Media Device Path subtype not supported", subtype);
                        }
                    }
                },
                0x05 => path.push("CSM".to_string()),
                0x7F => { // End of Hardware Device Path
                    let tmp = if subtype == 0xFF {
                        "End Entire Device Path"
                    } else {
                        "End This Instance of a Device Path and start a new Device Path"
                    };

                    println!("Wrapping up... found {} node", tmp);
                    break;
                }
                _ => {
                    todo!("{:X?} Device Path Node type not supported", cur_type);
                }
            }

            // advance our "pointer"
            cur += len as usize;

            if cur >= buf.len() {
                break;
            }
        }

        for i in &path {
            println!("{}", i);
        }


        let boot_id = var.to_string().split_off(4);
        let id = if let Ok(tmp) = u16::from_str_radix(&boot_id, 16) {
            tmp
        } else {
            return None
        };

        Some(Entry {
            path,
            id,
            description,
        })
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
    let mut buf = vec![0u8; 512];
    let mut options: Vec<Entry> = Vec::new();

    for var in manager.get_var_names().expect("asdf") {
        if boot_xxxx.is_match(var.variable()) {
            match manager.read(&var, &mut buf)  {
                Ok((size, ..)) => {
                    if let Some(entry) = Entry::new(&var.short_name(), &buf[..size]) {
                        options.push(entry);
                    }
                }
                Err(e) => eprintln!("{}", e),
            }
        }
    }

    options.sort_by_key(|s| s.id);

    for opt in &options {
        println!("Choose next boot: {}: {}, at: {:?}", opt.id, opt.description, opt.path);
    }

    let stdin = std::io::stdin();
    let input = stdin.lock().lines().next().unwrap().unwrap();
    let tmp: u16 = u16::from_str(&input).unwrap();
    if let Some(choice) = options.iter().find(|s| s.id == tmp) {
        println!("Setting {}", choice.description);

        let next = VariableName::new("BootNext");
        let attr = VariableFlags::NON_VOLATILE | VariableFlags::BOOTSERVICE_ACCESS | VariableFlags::RUNTIME_ACCESS;
        let val: [u8; 2] = choice.id.to_le_bytes();

        //manager.write(&next, attr, &val)?;
    } else {
        eprintln!("wrong choice");
    }
    Ok(())
}
