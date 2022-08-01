use gamepad_gui::ToolkitBuilder;
use regex::Regex;
use efivar::efi::VariableVendor;

#[derive(Debug)]
struct Entry {
    id: u16,
    boot_id: String,
    description: String,
}

impl Entry {
    fn new(var: &str, buf: &[u8]) -> Option<Entry> {
        let mut desc: Vec<u8> = vec![];
        let mut end_desc: usize = 0;

        // with crude conversion of u16 into u8
        for (i, byte) in buf.iter().skip(6).enumerate() {
            if i % 2 != 0 { continue; }
            if *byte == '\0' as u8 {
                end_desc = i+8; // ????
                break;
            }
            desc.push(byte.clone());
        }

        let mut pathlist: Vec<u8> = vec![];
        for (i, byte) in buf.iter().skip(end_desc).enumerate() {
            pathlist.push(byte.clone());
        }

        match pathlist[0] {
            0x05 => (), // BIOS Boot Specification Device Path
            0x04 => {    // Media Device Path
                let mut path: Vec<u8> = vec![];

                // again, char16 -> char8, also ????
                for (i, byte) in pathlist[46..].iter().enumerate() {
                    if i % 2 != 0 { continue; }
                    path.push(byte.clone());
                }
                let tmp = String::from_utf8_lossy(&path).to_lowercase();
                if tmp.starts_with(r"\efi\boot\bootx64.efi") || tmp.starts_with(r"\efi\boot\bootaa64.efi") {
                    // Ignore default boot mediums
                    return None;
                }

            }
            _ => return None,
        }


        let boot_id = var.to_string().split_off(4);
        let id = if let Ok(tmp) = u16::from_str_radix(&boot_id, 16) {
            tmp
        } else {
            return None
        };

        Some(Entry {
            boot_id, id,
            description: String::from_utf8_lossy(&desc).to_string(),
        })
    }
}

fn main() {
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
    let manager = efivar::system();
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

    for opt in options {
        println!("Choice: {}", opt.description);
    }
}
