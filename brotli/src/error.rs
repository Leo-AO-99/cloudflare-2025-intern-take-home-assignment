use thiserror::Error;

#[derive(Error, Debug)]
pub enum BrotliError {
    #[error("Not enough bits to read")]
    NotEnoughBits,

    #[error("UpdatePosError")]
    IncreasePosError,

    #[error("DecreasePosError")]
    DecreasePosError,

    #[error("Huffman tree does not match the bit stream")]
    HuffmanTreeNotMatch,
}
