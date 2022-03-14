use std::char::DecodeUtf16Error;
use std::mem;

pub fn from_utf16(v: &[u8]) -> Result<String, DecodeUtf16Error> {
    const SIZE_OF_UTF16_CHAR: usize = mem::size_of::<u16>();
    let iter = (0..v.len() / SIZE_OF_UTF16_CHAR)
        .map(|i| u16::from_le_bytes([v[SIZE_OF_UTF16_CHAR * i], v[SIZE_OF_UTF16_CHAR * i + 1]]));
    std::char::decode_utf16(iter).collect::<Result<String, _>>()
}

#[test]
fn test_from_utf16() {
    let expected = vec!["Record          #", "Record", "Flowers "];
    let tests = [
        vec![
            82, 0, 101, 0, 99, 0, 111, 0, 114, 0, 100, 0, 32, 0, 32, 0, 32, 0, 32, 0, 32, 0, 32, 0,
            32, 0, 32, 0, 32, 0, 32, 0, 35, 0,
        ],
        vec![82, 0, 101, 0, 99, 0, 111, 0, 114, 0, 100, 0, 32],
        vec![70, 0, 108, 0, 111, 0, 119, 0, 101, 0, 114, 0, 115, 0, 32, 0],
    ];
    for et in tests.iter().zip(expected.iter()) {
        let (t, expected) = et;
        assert_eq!(expected, &&from_utf16(t).unwrap());
    }
}
