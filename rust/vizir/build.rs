use std::fs::File;
use std::io::Write;

fn main() {
    init_sixel();
    died();
    flag();
}

fn init_sixel() {
    let mut header = String::new();
    header.push_str("\\x1bPq");
    for r in 0..6 {
        for g in 0..6 {
            for b in 0..6 {
                let code = 16 + r * 36 + g * 6 + b;
                let r = 100 * if r != 0 { r * 8 + 11 } else { 0 } / 51;
                let g = 100 * if g != 0 { g * 8 + 11 } else { 0 } / 51;
                let b = 100 * if b != 0 { b * 8 + 11 } else { 0 } / 51;
                header.push_str(&format!("#{code};2;{r};{g};{b}"));
            }
        }
    }
    for gray in 0..24 {
        let l = gray * 10 + 8;
        let code = 232 + gray;
        header.push_str(&format!("#{code};2;{l};{l};{l}"));
    }
    let mut f = File::create("src/sixel.rs").unwrap();
    writeln!(f, "const HEADER: &str = \"{header}\";").unwrap();
    writeln!(f, "const FOOTER: &str = \"\\x1b\\\\\";").unwrap();
}

fn died() {
    let f = include_str!("textures/died.ascii");
    let lines: Vec<&str> = f.lines().collect();
    let mut f = File::create("src/died.rs").unwrap();
    writeln!(f, "const DIED: [&str; {}] = [", lines.len()).unwrap();
    let mut width = 0;
    for line in lines {
        writeln!(f, "    r\"{line}\",").unwrap();
        width = width.max(line.len());
    }
    writeln!(f, "];").unwrap();
    writeln!(f, "const DIED_WIDTH: usize = {width};").unwrap();
}

fn flag() {
    let flag = b"expX{FLAG}";
    let mut f = File::create("src/flag.rs").unwrap();
    writeln!(
        f,
        "const FLAG: [u8; {}] = {:?};",
        flag.len(),
        flag.map(|b| b ^ 0x42)
    )
    .unwrap();
}
