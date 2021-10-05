
use simple_error::SimpleError;

fn seven_bit_decompress_get_size(
	compressed_data: &[u8]
) -> usize {
	if compressed_data.is_empty() || compressed_data[0] >> 3 > 2 /* NOT 7BITASCII and NOT 7BITUNICODE */ {
	  return 0;
	}
	let cbit_final = (compressed_data[0] & 0x7) + 1;
	let cbit_total = ((compressed_data.len() - 2) * 8) + cbit_final as usize;
	cbit_total / 7
}

fn seven_bit_decompress_buf(
    compressed_data: &[u8]
) -> Result<Vec<u8>, SimpleError> {
	if compressed_data.is_empty() || compressed_data[0] == 0x18 {
		return Err(SimpleError::new("compressed data is too short"));
	}

    let decompressed_size = seven_bit_decompress_get_size(compressed_data);
	if decompressed_size == 0 {
		return Err(SimpleError::new("compressed data size is 0"));
	}

	let mut uncompressed_data = Vec::<u8>::with_capacity(decompressed_size as usize);
    unsafe { uncompressed_data.set_len(uncompressed_data.capacity()); }

	let mut compressed_index = 1usize;
	let mut compressed_bit = 0u8;
	for i in &mut uncompressed_data {
		let byte;
		if compressed_bit <= 1 {
			byte = (compressed_data[compressed_index] >> compressed_bit) & 0x7f;
		} else {
			let compressed_word = u16::from_ne_bytes([compressed_data[compressed_index],
				compressed_data[compressed_index+1]]) as u32;
			byte = ((compressed_word >> compressed_bit) & 0x7f) as u8;
		}
		*i = byte;
		compressed_bit += 7;
		if compressed_bit >= 8 {
			compressed_bit %= 8;
			compressed_index += 1;
		}
	}

	Ok(uncompressed_data)
}

#[test]
fn test_7bit_decompression() {
	let mut test_compression_7bit_compressed_data : Vec<u8> = vec![
		0xe, 0xd2, 0xa2, 0x0e, 0x04, 0x42, 0xbd, 0x82, 0xf2, 0x31, 0x3a, 0x5d, 0x36, 0xb7, 0xc3, 0x70,
		0x78, 0xd9, 0xfd, 0xb2, 0x96, 0xe5, 0xf7, 0xb4, 0x9a, 0x5c, 0x96, 0x93, 0xcb, 0xa0, 0x34, 0xbd,
		0xdc, 0x9e, 0xbf, 0xac, 0x65, 0xb9, 0xfe, 0xed, 0x26, 0x97, 0xdd, 0xa0, 0x34, 0xbd, 0xdc, 0x9e,
		0xa7, 0x00
	];

	let test_compression_7bit_uncompressed_data : Vec<u8> = vec![
		0x52, 0x45, 0x3a, 0x20, 0x20, 0x28, 0x2f, 0x41, 0x72, 0x63, 0x68, 0x69, 0x65, 0x66, 0x6d, 0x61,
		0x70, 0x70, 0x65, 0x6e, 0x2f, 0x56, 0x65, 0x72, 0x77, 0x69, 0x6a, 0x64, 0x65, 0x72, 0x64, 0x65,
		0x20, 0x69, 0x74, 0x65, 0x6d, 0x73, 0x2f, 0x56, 0x65, 0x72, 0x7a, 0x6f, 0x6e, 0x64, 0x65, 0x6e,
		0x20, 0x69, 0x74, 0x65, 0x6d, 0x73, 0x29
	];

	let uncompressed_data_size = seven_bit_decompress_get_size(&test_compression_7bit_compressed_data);
	assert_eq!(uncompressed_data_size, 55);

	let uncompressed_data = seven_bit_decompress_buf(&test_compression_7bit_compressed_data).expect("7-bit decompression failed");
	assert_eq!(&uncompressed_data, &test_compression_7bit_uncompressed_data);

	let empty : Vec<u8> = vec![];
	assert_eq!(seven_bit_decompress_get_size(&empty), 0);
	let empty_res = seven_bit_decompress_buf(&empty);
	assert!(empty_res.is_err(), "{}", true);

	// test 7bit UNICODE decompression
	test_compression_7bit_compressed_data[0] = 0x16;
	let uncompressed_data_size_u = decompress_size(&test_compression_7bit_compressed_data);
	assert_eq!(uncompressed_data_size_u, 110);

	let uncompressed_data_u = decompress_buf(&test_compression_7bit_compressed_data, uncompressed_data_size_u).expect("decompression failed");
	for i in 0..uncompressed_data.len() {
		assert_eq!(uncompressed_data[i], uncompressed_data_u[i*2]);
		assert_eq!(uncompressed_data_u[i*2+1], 0);
	}
}

pub fn decompress_size(
	compressed_data : &[u8]
) -> usize {
    if compressed_data.is_empty() {
		return 0;
	}
	let identifier = compressed_data[0] >> 3;
	match identifier {
		1 => { // 7bit ASCII
			seven_bit_decompress_get_size(compressed_data)
		},
		2 => { // 7bit UNICODE
			seven_bit_decompress_get_size(compressed_data) * 2
		},
		3 => { // LZXPRESS
			if compressed_data.len() < 3 {
				return 0;
			}
			u16::from_ne_bytes([compressed_data[1], compressed_data[2]]) as usize
		},
		_ => {
			0
		}
	}
}

pub fn decompress_buf(
    compressed_data: &[u8],
    decompressed_size: usize
) -> Result<Vec<u8>, SimpleError> {
	if compressed_data.is_empty() {
		return Err(SimpleError::new("compressed data is too short"));
	}
	let identifier = compressed_data[0] >> 3;
	match identifier {
		1 => { // 7bit ASCII
			seven_bit_decompress_buf(compressed_data)
		},
		2 => { // 7bit UNICODE
			let decompressed_buf = seven_bit_decompress_buf(compressed_data)?;
			let mut buf = Vec::<u8>::with_capacity(decompressed_buf.len() * 2);
    		unsafe { buf.set_len(buf.capacity()); }
			let mut i = 0;
			for c in decompressed_buf {
				buf[i] = c;
				buf[i+1] = 0;
				i += 2;
			}
			Ok(buf)
		},
		3 => { // LZXPRESS
			if compressed_data.len() < 3 {
				return Err(SimpleError::new("compressed data is too short"));
			}
			match lz77_decompress(&compressed_data[3..], decompressed_size) {
				Ok(unc) => {
					Ok(unc)
				},
				Err(e) => {
					let s = format!("{:?}", e);
					println!("{}", s);
					Err(SimpleError::new(s))
				}
			}
		},
		_ => {
			return Err(SimpleError::new(format!("bad identifier: {}", identifier)));
		}
	}
}

#[cfg(target_os = "windows")]
extern "C" {
    fn decompress(
        data: *const u8, data_size: u32, out_buffer: *mut u8, out_buffer_size: u32, decompressed: *mut u32) -> u32;
}

#[allow(dead_code)]
#[cfg(target_os = "windows")]
pub fn ms_impl_decompress_size(
    v: &[u8]
) -> usize {
    const JET_wrnBufferTruncated: u32 = 1006;

    let mut decompressed: u32 = 0;
    let res = unsafe { decompress(v.as_ptr(), v.len() as u32, std::ptr::null_mut(), 0, &mut decompressed) };

    if res == JET_wrnBufferTruncated && decompressed as usize > v.len() {
        return decompressed as usize;
    }
    0
}

#[allow(dead_code)]
#[cfg(target_os = "windows")]
pub fn ms_impl_decompress_buf(
    v: &[u8],
    decompressed_size: usize
) -> Result<Vec<u8>, SimpleError> {
    const JET_errSuccess: u32 = 0;
    let mut buf = Vec::<u8>::with_capacity(decompressed_size);
    unsafe { buf.set_len(buf.capacity()); }
    let mut decompressed : u32 = 0;
    let res = unsafe { decompress(v.as_ptr(), v.len() as u32, buf.as_mut_ptr(), buf.len() as u32, &mut decompressed) };
    debug_assert!(decompressed_size == decompressed as usize && decompressed as usize == buf.len());
    if res != JET_errSuccess {
        return Err(SimpleError::new(format!("Decompress failed. Err {}", res)));
    }
    Ok(buf)
}

#[cfg(target_os = "windows")]
#[test]
fn test_lzxpress_decompression() {
	let comp_data : Vec<u8> = vec![
		0x18, 0x2C, 0x01, 0xff, 0xff, 0xff, 0x1f, 0x61, 0x62, 0x63, 0x17, 0x00, 0x0f, 0xff, 0x26, 0x01];
	let ms1 = ms_impl_decompress_size(&comp_data);
	let ms1_dec = ms_impl_decompress_buf(&comp_data, ms1).expect("ms_impl decomp failed");
	let unc = lz77_decompress(&comp_data[3..], ms1).expect("lz77 decomp failed");
	assert_eq!(ms1_dec, unc);
}

// https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-xca/a8b7cb0a-92a6-4187-a23b-5e14273b96f8
// 2.4.4
fn lz77_decompress(
    in_buf: &[u8],
	decompress_size: usize
) -> Result<Vec<u8>, SimpleError>
{
	let mut out_i:      usize = 0;
    let mut out_pos:    usize = 0;
    let mut in_pos:     usize = 0;
    let mut last_len:   usize = 0;
    let mut flags:      u32   = 0;
    let mut flag_count: u32   = 0;

    let mut out_buf = Vec::<u8>::with_capacity(decompress_size);
	unsafe { out_buf.set_len(out_buf.capacity()); }

    while in_pos < in_buf.len() {
        if flag_count == 0 {
            if (in_pos + 3) >= in_buf.len() {
                return Err(SimpleError::new("index out of bounds"));
            }

			flags = u32::from_le_bytes([in_buf[in_pos], in_buf[in_pos+1], in_buf[in_pos+2], in_buf[in_pos+3]]);
            in_pos += 4;
            flag_count = 32;
        }

        flag_count -= 1;

        if (flags & (1 << flag_count)) == 0 {
            if in_pos >= in_buf.len() {
                return Err(SimpleError::new("index out of bounds"));
            }
            out_buf[out_i] = in_buf[in_pos];
			out_i += 1;

            in_pos += 1;
            out_pos += 1;
        } else {
			if in_pos == in_buf.len() {
				break;
			} else if (in_pos + 1) > in_buf.len() {
                return Err(SimpleError::new("index out of bounds"));
            }

			let mut length = u16::from_le_bytes([in_buf[in_pos], in_buf[in_pos+1]]) as usize;
            in_pos += 2;

			let offset = (length / 8) + 1;
            length %= 8;

            if length == 7 {
                if last_len == 0 {
                    if in_pos >= in_buf.len() {
                        return Err(SimpleError::new("index out of bounds"));
                    }

                    length = (in_buf[in_pos] % 16).into();
                    last_len = in_pos;
                    in_pos += 1;
                } else {
                    if last_len >= in_buf.len() {
                        return Err(SimpleError::new("index out of bounds"));
                    }

                    length = (in_buf[last_len] / 16).into();
                    last_len = 0;
                }

                if length == 15 {
                    if in_pos >= in_buf.len() {
                        return Err(SimpleError::new("index out of bounds"));
                    }

                    length = in_buf[in_pos].into();
                    in_pos += 1;

                    if length == 255 {
                        if (in_pos + 1) >= in_buf.len() {
                            return Err(SimpleError::new("index out of bounds"));
                        }

						length = u16::from_le_bytes([in_buf[in_pos], in_buf[in_pos+1]]) as usize;
                        in_pos += 2;

                        if length == 0 {
							length = u32::from_le_bytes([in_buf[in_pos], in_buf[in_pos+1], in_buf[in_pos+2],
								in_buf[in_pos+3]]) as usize;
                            in_pos += 4;
                        }

                        if length < 15 + 7 {
                            return Err(SimpleError::new("corrupted data"));
                        }
                        length -= 15 + 7;
                    }
                    length += 15;
                }
                length += 7;
            }
            length += 3;

            for _ in 0..length {
                if offset > out_pos {
                    return Err(SimpleError::new("corrupted data"));
                }

                out_buf[out_i] = out_buf[out_pos - offset];
				out_i += 1;
                out_pos += 1;
            }
        }
    }

    Ok(out_buf)
}
