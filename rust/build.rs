use std::fs::File;
use std::io::Write;

fn main() {
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
    println!("cargo:rustc-cfg=crt_static");
    // println!("cargo:rerun-if-changed=build.rs");
    map_texture();
    wall_texture();
}

fn map_texture() {
    let img = bmp::open("textures/map.bmp").unwrap();

    let mut hi = 0u128;
    let mut lo = 0u128;

    let mut win = vec![];

    for (x, y) in img.coordinates() {
        let pixel = img.get_pixel(x, y);
        let bit = pixel.r == 0 && pixel.g == 0 && pixel.b == 0;
        if y < 8 {
            hi <<= 1;
            hi |= bit as u128;
        } else {
            lo <<= 1;
            lo |= bit as u128;
        }
        if pixel.r == 128 && pixel.g == 128 && pixel.b == 128 {
            win.push([x, y]);
        }
    }

    for y in 0..16 {
        let mut row = String::with_capacity(16);
        for x in 0..16 {
            let bit = y * 16 + x;
            let bit = if y < 8 {
                (hi >> (127 - bit)) & 1
            } else {
                (lo >> (255 - bit)) & 1
            };
            if win.contains(&[x, y]) {
                row.push('@');
            } else {
                row.push(if bit == 1 { '█' } else { ' ' });
            }
        }
    }

    let mut f = File::create("src/map.rs").unwrap();
    writeln!(f, "const MAP: [u128; 2] = [").unwrap();
    writeln!(f, "    0x{hi:032x},").unwrap();
    writeln!(f, "    0x{lo:032x},").unwrap();
    writeln!(f, "];").unwrap();
    writeln!(f, "const WIN: &[[u32; 2]] = &{win:?};").unwrap();
}

fn wall_texture() {
    let img = bmp::open("textures/wall.bmp").unwrap();
    let mut content = [0u128; 16];
    for (x, y) in img.coordinates() {
        let pixel = img.get_pixel(x, y);
        let code = rgb2ansi256::rgb_to_ansi256(pixel.r, pixel.g, pixel.b);
        content[y as usize] |= (code as u128) << (8 * (15 - x));
    }
    let mut f = File::create("src/wall.rs").unwrap();
    writeln!(f, "const WALL: [u128; 16] = [").unwrap();
    for bytes in content {
        writeln!(f, "    0x{bytes:032x},").unwrap();
    }
    writeln!(f, "];").unwrap();
}
