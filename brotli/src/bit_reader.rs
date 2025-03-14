use crate::error::BrotliError;

pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    length: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            length: data.len() as usize * 8,
        }
    }

    pub fn get_pos(&self) -> usize {
        self.pos
    }

    pub fn peek_bits(&mut self, bits: u8) -> Result<u64, BrotliError> {
        self.top_bits(bits, false)
    }

    pub fn read_bits(&mut self, bits: u8) -> Result<u64, BrotliError> {
        self.top_bits(bits, true)
    }

    pub fn increase_pos(&mut self, increment: usize) -> Result<(), BrotliError> {
        if self.pos + increment > self.length {
            return Err(BrotliError::IncreasePosError);
        }
        self.pos += increment;
        Ok(())
    }

    pub fn decrease_pos(&mut self, decrement: usize) -> Result<(), BrotliError> {
        if self.pos < decrement {
            return Err(BrotliError::DecreasePosError);
        }
        self.pos -= decrement;
        Ok(())
    }

    fn top_bits(&mut self, bits: u8, pop: bool) -> Result<u64, BrotliError> {
        let start = self.pos;
        let end = start + bits as usize;
        if end > self.length {
            return Err(BrotliError::NotEnoughBits);
        }

        let mut ret: u64 = 0;
        for i in start..end {
            let byte = self.data[i / 8];
            let bit_idx = i % 8;
            ret |= ((byte >> bit_idx) as u64 & 1) << (i - start);
        }
        if pop {
            self.pos = end;
        }
        Ok(ret)
    }

    pub fn empty(&self) -> bool {
        self.pos == self.length
    }

    pub fn remaining_bits(&self) -> usize {
        self.length - self.pos
    }
}

mod test {
    #[test]
    fn bit_reader_test() {
        use super::*;
        use crate::EXAMPLE_HEX;
        let mut reader = BitReader::new(&EXAMPLE_HEX);
        let hskip = reader.read_bits(2).unwrap();
        assert_eq!(hskip, 0b00);

        for _ in 0..15 {
            let symbol_code = reader.read_bits(2).unwrap();
            assert_eq!(symbol_code, 0b01);
        }

        for _ in 0..2 {
            let symbol_code = reader.read_bits(4).unwrap();
            assert_eq!(symbol_code, 0b1111);
        }
    }
}
