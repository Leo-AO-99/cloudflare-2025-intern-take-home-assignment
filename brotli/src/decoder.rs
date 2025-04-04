use crate::{
    bit_reader::BitReader, error::BrotliError, huffman_tree::HuffmanTree, ALPHABET_SIZE_LIMIT,
    CODE_LENGTH_CODES, CODE_LENGTH_CODE_ORDER, DEFAULT_CODE_LENGTH,
};

pub fn decode_code_length_codes(
    bit_reader: &mut BitReader,
) -> Result<[(u16, u16, u8); CODE_LENGTH_CODES], BrotliError> {
    let skip = bit_reader.read_bits(2)? as usize;
    let mut code_length_code_lengths: [u8; CODE_LENGTH_CODES] = [0; CODE_LENGTH_CODES];
    let mut space: i32 = 32;
    let mut num_codes = 0;

    for i in skip..CODE_LENGTH_CODES {
        let code_len_idx = CODE_LENGTH_CODE_ORDER[i];
        let code_length_code = bit_reader.peek_bits(4)?;

        let (code_length, pos_increment) = match code_length_code {
            0b0000 | 0b0100 | 0b1000 | 0b1100 => (0, 2), // 0b00
            0b0111 => (1, 4),                            // 0b0111
            0b0011 | 0b1011 => (2, 3),                   // 0b011
            0b0010 | 0b0110 | 0b1010 | 0b1110 => (3, 2), // 0b10
            0b0001 | 0b0101 | 0b1001 | 0b1101 => (4, 2), // 0b01
            0b1111 => (5, 4),                            // 0b111
            _ => panic!("Invalid code length code: {:b}", code_length_code), // should never be here
        };
        bit_reader.increase_pos(pos_increment)?;
        code_length_code_lengths[code_len_idx as usize] = code_length;
        if code_length != 0 {
            // 0 length should be omitted
            space -= (32 >> code_length) as i32;
            num_codes += 1;
            if space <= 0 {
                break;
            }
        }
    }
    if space != 0 && num_codes != 1 {
        panic!("Corrupted Huffman code histogram");
    }

    let mut code = 0;
    let mut next_code: [u8; 6] = [0; 6];
    let mut bl_count = [0; 6];
    let mut codes_info: [(u16, u16, u8); CODE_LENGTH_CODES] = [(0, 0, 0); CODE_LENGTH_CODES];

    for i in 0..CODE_LENGTH_CODES {
        let code_length_code = CODE_LENGTH_CODE_ORDER[i];
        let code_length = code_length_code_lengths[code_length_code as usize];
        bl_count[code_length as usize] += 1;
    }

    bl_count[0] = 0;
    for bits in 1..=5 {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }

    for symbol in 0..CODE_LENGTH_CODES {
        let code_length = code_length_code_lengths[symbol];
        if code_length == 0 {
            continue;
        }

        let symbol_code = &mut next_code[code_length as usize];

        codes_info[symbol] = (symbol as u16, *symbol_code as u16, code_length);

        *symbol_code += 1;
    }

    Ok(codes_info)

    // let tree = HuffmanTree::new_huffman_tree(&codes_info);
    // Ok(tree.decode_complex_prefix_code(&mut bit_reader))
}

pub fn decode_symbol_codes(
    bit_reader: &mut BitReader,
    code_length_info: &[(u16, u16, u8); CODE_LENGTH_CODES],
) -> Result<[(u16, u16, u8); ALPHABET_SIZE_LIMIT], BrotliError> {
    let tree = HuffmanTree::new_huffman_tree(code_length_info);

    struct ReapeatInfo {
        prev_code_len: u8,
        repeat_cmd: u16,
        repeat_count: u16,
    }
    struct State {
        symbol: usize,
        code_lengths: [u8; ALPHABET_SIZE_LIMIT],
        space: i32,
        repeat_info: ReapeatInfo,
    }

    let mut state = State {
        symbol: 0,
        code_lengths: [0; ALPHABET_SIZE_LIMIT],
        space: 32768,
        repeat_info: ReapeatInfo {
            prev_code_len: DEFAULT_CODE_LENGTH,
            repeat_cmd: 0,
            repeat_count: 0,
        },
    };

    let handle_repeat = |state: &mut State| {
        if state.repeat_info.repeat_cmd == 0 || state.repeat_info.repeat_count == 0 {
            return;
        }

        let repeat_code_len = if state.repeat_info.repeat_cmd == 17 {
            0
        } else {
            state.space -=
                (state.repeat_info.repeat_count << (15 - state.repeat_info.prev_code_len)) as i32;
            state.repeat_info.prev_code_len
        };

        // println!("handle_repeat: repeat_cmd: {}, repeat_count: {}, repeat_code_len: {}", state.repeat_info.repeat_cmd, state.repeat_info.repeat_count, repeat_code_len);

        for _ in 0..state.repeat_info.repeat_count {
            state.code_lengths[state.symbol] = repeat_code_len;
            state.symbol += 1;
        }

        // reset info
        // because 16 repeat last non-zero, 17 repeat zero, so no need to change repeat_info.prev_code_len
        state.repeat_info.repeat_cmd = 0;
        state.repeat_info.repeat_count = 0;
    };
    while !bit_reader.empty() {
        if state.space <= 0 || state.symbol >= ALPHABET_SIZE_LIMIT {
            break;
        }
        let code_len = tree.read_symbol(bit_reader)?;
        if code_len < 16 {
            // println!("{}", code_len);

            // if repeat_count is zero, nothing will happen
            // if repeat_cmd is 16 or 17, execute repeat
            handle_repeat(&mut state);
            state.code_lengths[state.symbol] = code_len as u8;
            state.symbol += 1;
            if code_len != 0 {
                state.repeat_info.prev_code_len = code_len as u8;
                state.space -= 32768 >> code_len;
            }
            // println!("code_len: {}", code_len);
        } else if code_len == 16 || code_len == 17 {
            if state.repeat_info.repeat_cmd != code_len {
                // if repeat cmd is zero, nothing will happen
                // this will be first repeat command
                handle_repeat(&mut state);
            }
            state.repeat_info.repeat_cmd = code_len;
            let step = code_len - 14;
            // let mut new_repeat = bit_reader.read_bits(1)?;
            // for _ in 0..step - 1 {
            //     new_repeat = (new_repeat << 1) | bit_reader.read_bits(1)?;
            // }
            let mut new_repeat = bit_reader.read_bits(step as u8)?;
            // println!("{} {}", code_len, new_repeat);
            new_repeat += 3;


            state.repeat_info.repeat_count = if state.repeat_info.repeat_count != 0 {
                // 16 need times 4, 17 need times 8
                ((state.repeat_info.repeat_count - 2) << step) + new_repeat as u16
            } else {
                new_repeat as u16
            };
        } else {
            panic!("invalid code length: {}", code_len);
        }
    }
    if state.space != 0 {
        panic!("left space is not 0, {}", state.space);
    }

    // codes_info: [(symbol, code, code_length), ...]
    let mut alphabet_codes_info: [(u16, u16, u8); ALPHABET_SIZE_LIMIT] =
        [(0, 0, 0); ALPHABET_SIZE_LIMIT];
    let mut bl_count: [u16; 16] = [0; 16];

    for i in 0..ALPHABET_SIZE_LIMIT {
        let alphabet_code_len = state.code_lengths[i];
        // if alphabet_code_len != 0 {
        //     println!("{} {}", i, alphabet_code_len);
        // }
        bl_count[alphabet_code_len as usize] += 1;
    }
    bl_count[0] = 0;
    let mut first_code: u16 = 0;
    let mut next_code: [u16; 16] = [0; 16];
    for bits in 1..=15 {
        first_code = (first_code + bl_count[bits - 1]) << 1;
        next_code[bits] = first_code;
    }

    for i in 0..ALPHABET_SIZE_LIMIT {
        let alphabet_code_len = state.code_lengths[i];
        if alphabet_code_len == 0 {
            continue;
        }
        let symbol_code = &mut next_code[alphabet_code_len as usize];
        // (symbol, code, code_length)
        alphabet_codes_info[i] = (i as u16, *symbol_code as u16, alphabet_code_len);
        *symbol_code += 1;
    }

    Ok(alphabet_codes_info)
}

pub fn decode_alphabet_code(
    bit_reader: &mut BitReader,
    alphabet_codes_info: &[(u16, u16, u8); ALPHABET_SIZE_LIMIT],
) -> Result<(), BrotliError> {
    let tree = HuffmanTree::new_huffman_tree(alphabet_codes_info);
    while !bit_reader.empty() {
        let symbol = match tree.read_symbol(bit_reader) {
            Ok(s) => s,
            Err(_) => break,
        };
        // println!("symbol: {}", symbol);
        print!("{}", symbol as u8 as char);
    }
    // println!("{}", bit_reader.remaining_bits());
    Ok(())
}

mod test {

    #[test]
    fn decode() -> Result<(), Box<dyn std::error::Error>> {
        use crate::bit_reader::BitReader;
        use crate::TEST_INPUT;

        use crate::decoder::decode_alphabet_code;
        use crate::decoder::decode_code_length_codes;
        use crate::decoder::decode_symbol_codes;

        let mut bit_reader = BitReader::new(&TEST_INPUT);
        let code_info = decode_code_length_codes(&mut bit_reader)?;
        let alphabet_codes_info = decode_symbol_codes(&mut bit_reader, &code_info)?;
        println!("{}", bit_reader.remaining_bits());
        decode_alphabet_code(&mut bit_reader, &alphabet_codes_info)?;
        Ok(())
    }

    #[test]
    fn canonical_prefix_code() {
        // https://datatracker.ietf.org/doc/html/rfc7932#section-3.2
        let mut code = 0;
        let bl_count: [u8; 5] = [0, 0, 1, 5, 2];
        let mut next_code: [u8; 5] = [0; 5];
        for bits in 1..=4 {
            // for example, bits = 4
            // bl_count[3] = 5 means there are 5 codes with 3 bits
            // code means the first code with 3 bits, which is 010
            // 2(010) + 5 = 7(111)
            // so 111 is the first code that 3 bits do not use, prefix 111 is ok
            // so 111 << 1 = 1110 can be used as the first code with 4 bits
            code = (code + bl_count[bits - 1]) << 1;
            next_code[bits] = code;
        }

        for bits in 1..=4 {
            println!("{}, {}", bits, next_code[bits]);
        }
    }
}
