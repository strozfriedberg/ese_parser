use crate::parser::ese_db::*;
use simple_error::SimpleError;
use std::array::TryFromSliceError;
use std::char::DecodeUtf16Error;
use std::convert::TryInto;
use std::mem;

pub fn calc_crc32(buffer: &[u8]) -> u32 {
    let buf32 = unsafe {
        std::slice::from_raw_parts(buffer.as_ptr() as *const u32, (buffer.len() / 4) as usize)
    };
    buf32
        .iter()
        .skip(1)
        .fold(ESEDB_FILE_SIGNATURE, |crc, &val| crc ^ val)
}

fn get_u32_byte_slice(pb: &[u8]) -> Result<[u32; 8], SimpleError> {
    const size_of_u32: usize = mem::size_of::<u32>();
    let mut ret: [u32; 8] = [0; 8];

    for index in 0..8 {
        let val = u32::from_le_bytes(
            pb[index * size_of_u32..(index * size_of_u32) + size_of_u32]
                .try_into()
                .map_err(|e: TryFromSliceError| {
                    SimpleError::new(format!(
                        "can't get_u32_byte_slice for calc_new_crc. error: {}",
                        e
                    ))
                })?,
        );
        ret[index] = val;
    }
    Ok(ret)
}

// translated from
// https://github.com/microsoft/Extensible-Storage-Engine/blob/933dc839b5a97b9a5b3e04824bdd456daf75a57d/dev/ese/src/_esefile/xsum.cxx#L887
// ChecksumNewFormat
pub fn calc_new_crc(pb: &[u8], pgno: u32, skip_header: bool) -> Result<u64, SimpleError> {
    let cb = pb.len() as u32;
    let cdw = (cb / 4) as usize;

    let mut p: u32 = 0;
    let mut p0: u32 = 0;
    let mut p1: u32 = 0;
    let mut p2: u32 = 0;
    let mut p3: u32 = 0;
    {
        let mut idxp: u32 = 0xff800000;
        let mut i = 0;
        let mut pT0: u32 = 0;
        let mut pT1: u32 = 0;

        let size_of_u32 = mem::size_of::<u32>();
        loop {
            let _pT = get_u32_byte_slice(&pb[i * size_of_u32..])?;
            if i > 0 || !skip_header {
                pT0 = _pT[0];
                pT1 = _pT[1];
            }
            let pT2 = _pT[2];
            let pT3 = _pT[3];

            p0 ^= pT0;
            p1 ^= pT1;
            p2 ^= pT2;
            p3 ^= pT3;

            p ^= idxp & lParityMask(pT0 ^ pT1 ^ pT2 ^ pT3);

            idxp = idxp.wrapping_add(0xff800080);

            let pT4 = _pT[4];
            let pT5 = _pT[5];
            let pT6 = _pT[6];
            let pT7 = _pT[7];

            p0 ^= pT4;
            p1 ^= pT5;
            p2 ^= pT6;
            p3 ^= pT7;

            p ^= idxp & lParityMask(pT4 ^ pT5 ^ pT6 ^ pT7);

            idxp = idxp.wrapping_add(0xff800080);

            i += 8;
            if i >= cdw {
                break;
            }
        }
    }

    p |= 0x00400000 & lParityMask(p0 ^ p1);
    p |= 0x00000040 & lParityMask(p2 ^ p3);

    let r0 = p0 ^ p2;
    let r1 = p1 ^ p3;

    p |= 0x00200000 & lParityMask(r0);
    p |= 0x00000020 & lParityMask(r1);

    let r2 = r0 ^ r1;
    let mut r: u32 = 0;
    let mut idxr: u32 = 0xffff0000;
    for i in 0u32..32u32 {
        let mask: u32 = if (r2 & (1u32 << i)) > 0 {
            0xFFFFFFFF
        } else {
            0
        };
        r ^= mask & idxr;
        idxr = idxr.wrapping_add(0xffff0001);
    }

    let mask: u32 = (cb << 19).wrapping_sub(1);

    let eccChecksum = p & 0xffe0ffe0 & mask | r & 0x001f001f;
    let xorChecksum = r2;
    Ok(MakeChecksumFromECCXORAndPgno(
        eccChecksum,
        xorChecksum,
        pgno,
    ))
}

fn MakeChecksumFromECCXORAndPgno(eccChecksum: u32, xorChecksum: u32, pgno: u32) -> u64 {
    let low = (xorChecksum ^ pgno) as u64;
    let high = (eccChecksum as u64) << 32;
    high | low
}

fn lParityMask(dw: u32) -> u32 {
    static parityLookupTable: &'static [i32] = &[
        0, -1, -1, 0, -1, 0, 0, -1, -1, 0, 0, -1, 0, -1, -1, 0, // 0x0x
        -1, 0, 0, -1, 0, -1, -1, 0, 0, -1, -1, 0, -1, 0, 0, -1, // 0x1x
        -1, 0, 0, -1, 0, -1, -1, 0, 0, -1, -1, 0, -1, 0, 0, -1, // 0x2x
        0, -1, -1, 0, -1, 0, 0, -1, -1, 0, 0, -1, 0, -1, -1, 0, // 0x3x
        -1, 0, 0, -1, 0, -1, -1, 0, 0, -1, -1, 0, -1, 0, 0, -1, // 0x4x
        0, -1, -1, 0, -1, 0, 0, -1, -1, 0, 0, -1, 0, -1, -1, 0, // 0x5x
        0, -1, -1, 0, -1, 0, 0, -1, -1, 0, 0, -1, 0, -1, -1, 0, // 0x6x
        -1, 0, 0, -1, 0, -1, -1, 0, 0, -1, -1, 0, -1, 0, 0, -1, // 0x7x
        -1, 0, 0, -1, 0, -1, -1, 0, 0, -1, -1, 0, -1, 0, 0, -1, // 0x8x
        0, -1, -1, 0, -1, 0, 0, -1, -1, 0, 0, -1, 0, -1, -1, 0, // 0x9x
        0, -1, -1, 0, -1, 0, 0, -1, -1, 0, 0, -1, 0, -1, -1, 0, // 0xax
        -1, 0, 0, -1, 0, -1, -1, 0, 0, -1, -1, 0, -1, 0, 0, -1, // 0xbx
        0, -1, -1, 0, -1, 0, 0, -1, -1, 0, 0, -1, 0, -1, -1, 0, // 0xcx
        -1, 0, 0, -1, 0, -1, -1, 0, 0, -1, -1, 0, -1, 0, 0, -1, // 0xdx
        -1, 0, 0, -1, 0, -1, -1, 0, 0, -1, -1, 0, -1, 0, 0, -1, // 0xex
        0, -1, -1, 0, -1, 0, 0, -1, -1, 0, 0, -1, 0, -1, -1, 0, // 0xfx
    ];
    let dw1 = dw >> 16;
    let dw2 = dw ^ dw1;
    let dw3 = dw2 >> 8;
    let dw4 = dw2 ^ dw3;
    if parityLookupTable[(dw4 & 0xff) as usize] == -1 {
        0xFFFFFFFF
    } else {
        0
    }
}

pub fn from_utf16(v: &[u8]) -> Result<String, DecodeUtf16Error> {
    const SIZE_OF_UTF16_CHAR: usize = mem::size_of::<u16>();
    let iter = (0..v.len() / SIZE_OF_UTF16_CHAR)
        .map(|i| u16::from_le_bytes([v[SIZE_OF_UTF16_CHAR * i], v[SIZE_OF_UTF16_CHAR * i + 1]]));
    std::char::decode_utf16(iter).collect::<Result<String, _>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_from_utf16() {
        let expected = vec!["Record          #", "Record", "Flowers "];
        let tests = [
            vec![
                82, 0, 101, 0, 99, 0, 111, 0, 114, 0, 100, 0, 32, 0, 32, 0, 32, 0, 32, 0, 32, 0,
                32, 0, 32, 0, 32, 0, 32, 0, 32, 0, 35, 0,
            ],
            vec![82, 0, 101, 0, 99, 0, 111, 0, 114, 0, 100, 0, 32],
            vec![70, 0, 108, 0, 111, 0, 119, 0, 101, 0, 114, 0, 115, 0, 32, 0],
        ];
        for et in tests.iter().zip(expected.iter()) {
            let (t, expected) = et;
            assert_eq!(expected, &&from_utf16(t).unwrap());
        }
    }

    #[test]
    fn test_calc_new_crc_good() {
        let input = fs::read("testdata/checksum_buffer_12050322830504531039.bin").unwrap();
        assert_eq!(
            12050322830504531039,
            calc_new_crc(&input[..], 1793, true).unwrap()
        );
    }

    #[test]
    fn test_calc_new_crc_good_empty() {
        assert_eq!(Ok(0), calc_new_crc(&[0; 32], 0, false));
    }
}
