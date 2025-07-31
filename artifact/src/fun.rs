include!("signal.rs");

pub fn r#fn(t: usize) -> bool {
    let idx = t % LENGTH;

    ((SIGNAL[idx >> 7] >> (127 - (idx & 127))) & 1) != 0
}
