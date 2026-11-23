//! Decode fixed hexadecimal guests at compile time, without a host assembler.

const fn whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

const fn nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("invalid guest hex digit"),
    }
}

pub const fn byte_len(hex: &str) -> usize {
    let bytes = hex.as_bytes();
    let mut digits = 0;
    let mut i = 0;
    while i < bytes.len() {
        if !whitespace(bytes[i]) {
            let _ = nibble(bytes[i]);
            digits += 1;
        }
        i += 1;
    }
    assert!(digits % 2 == 0, "unpaired guest hex digit");
    digits / 2
}

pub const fn decode<const N: usize>(hex: &str) -> [u8; N] {
    assert!(byte_len(hex) == N, "guest hex length mismatch");
    let mut result = [0; N];
    let bytes = hex.as_bytes();
    let mut digit = 0;
    let mut i = 0;
    while i < bytes.len() {
        if !whitespace(bytes[i]) {
            result[digit / 2] = (result[digit / 2] << 4) | nibble(bytes[i]);
            digit += 1;
        }
        i += 1;
    }
    result
}

macro_rules! guest_bytes {
    ($path:literal) => {{
        const HEX: &str = include_str!($path);
        const BYTES: &[u8] =
            &crate::guest_bytes::decode::<{ crate::guest_bytes::byte_len(HEX) }>(HEX);
        BYTES
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_mixed_case_whitespace_and_empty_input() {
        const BYTES: [u8; 4] = decode("00 aB\tCD\r\neF");
        assert_eq!(BYTES, [0, 0xab, 0xcd, 0xef]);
        assert_eq!(decode::<0>(" \n"), []);
    }

    #[test]
    #[should_panic(expected = "invalid guest hex digit")]
    fn rejects_invalid_digits() {
        decode::<1>("0g");
    }

    #[test]
    #[should_panic(expected = "unpaired guest hex digit")]
    fn rejects_unpaired_digits() {
        byte_len("0");
    }
}
