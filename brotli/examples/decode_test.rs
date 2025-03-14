use brotli::{
    bit_reader::BitReader,
    decoder::{decode_alphabet_code, decode_code_length_codes, decode_symbol_codes},
    error::BrotliError,
    TEST_INPUT,
};

/// (symbol, code, code_length)
fn show_symbol_code_detail(code_info: &[(u16, u16, u8)]) {
    for (symbol, code, code_length) in code_info {
        if *code_length == 0 {
            continue;
        }
        println!("symbol: {}, code: {:0width$b}", symbol, code, width = *code_length as usize);
    }

}

fn main() -> Result<(), BrotliError> {
    let mut bit_reader = BitReader::new(&TEST_INPUT);
    let code_info = decode_code_length_codes(&mut bit_reader)?;
    println!("========================");
    show_symbol_code_detail(&code_info);
    let alphabet_codes_info = decode_symbol_codes(&mut bit_reader, &code_info)?;
    println!("========================");
    show_symbol_code_detail(&alphabet_codes_info);
    println!("still {} bits to be decoded", bit_reader.remaining_bits());
    decode_alphabet_code(&mut bit_reader, &alphabet_codes_info)?;
    Ok(())
}
