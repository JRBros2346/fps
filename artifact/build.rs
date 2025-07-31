use std::{fs::File, io::Write as _};

use morse_codec::encoder::Encoder;

fn main() {
    let flag = "expx(broken_artifact)";
    let mut encoder = Encoder::<128>::new().with_message(flag, true).build();
    encoder.encode_message_all();
    let encoded_charrays = encoder.get_encoded_message_as_morse_charrays();
    let out = encoded_charrays
        .map(|charray| {
            charray
                .unwrap()
                .into_iter()
                .flatten()
                .map(|ch| match ch {
                    b'.' => "1",
                    b'-' => "111",
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>()
                .join("0")
        })
        .collect::<Vec<_>>()
        .join("000")
        .as_bytes()
        .chunks(128)
        .map(|chunk| String::from_utf8_lossy(chunk).to_string())
        .collect::<Vec<_>>();
    println!("cargo:warning={out:?}");
    println!(
        "cargo:warning={}",
        out.iter().map(|s| s.len()).sum::<usize>()
    );
    let mut f = File::create("src/signal.rs").unwrap();
    writeln!(f, "const SIGNAL: [u128; {}] = [", out.len()).unwrap();
    for chunk in &out {
        writeln!(f, "    0b{chunk:0<128},").unwrap();
    }
    writeln!(f, "];").unwrap();
    writeln!(
        f,
        "const LENGTH: usize = {};",
        out.iter().map(|s| s.len()).sum::<usize>()
    )
    .unwrap();
}
