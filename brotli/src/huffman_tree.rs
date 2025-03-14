use crate::{bit_reader::BitReader, error::BrotliError};

#[derive(Clone)]
enum HuffmanNode {
    Empty,
    Leaf(u16),
    Internal {
        left: Box<HuffmanNode>,
        right: Box<HuffmanNode>,
    },
}

impl HuffmanNode {
    /// create a new internal node
    fn new_internal() -> Self {
        Self::Internal {
            left: Box::new(Self::Empty),
            right: Box::new(Self::Empty),
        }
    }
}

pub struct HuffmanTree {
    root: Box<HuffmanNode>,
}

impl HuffmanTree {
    /// arguments:
    /// 
    /// codes_info: [(symbol, code, code_length), ...]
    /// 
    /// return: HuffmanTree
    pub fn new_huffman_tree(codes_info: &[(u16, u16, u8)]) -> HuffmanTree {
        let mut root = HuffmanNode::new_internal();

        for (symbol, code, code_length) in codes_info {
            if *code_length == 0 {
                continue;
            }
            let mut cur_node = &mut root;

            for shift in (0..*code_length).rev() {
                let bit = (code >> shift) & 1;
                match cur_node {
                    HuffmanNode::Internal { left, right } => {
                        if bit == 0 {
                            if let HuffmanNode::Empty = **left {
                                **left = HuffmanNode::new_internal();
                            }
                            cur_node = left;
                        } else {
                            if let HuffmanNode::Empty = **right {
                                **right = HuffmanNode::new_internal();
                            }
                            cur_node = right;
                        }
                    }
                    _ => panic!("unexcepted node in building huffman tree"),
                }
            }

            *cur_node = HuffmanNode::Leaf(*symbol);
        }
        HuffmanTree {
            root: Box::new(root),
        }
    }

    pub fn read_symbol(&self, bit_reader: &mut BitReader) -> Result<u16, BrotliError> {
        let mut cur_node = &self.root;
        let mut bits_count = 0;

        while !bit_reader.empty() {
            match **cur_node {
                HuffmanNode::Empty => return Err(BrotliError::HuffmanTreeNotMatch),
                HuffmanNode::Leaf(symbol) => {
                    return Ok(symbol);
                }
                HuffmanNode::Internal {
                    ref left,
                    ref right,
                } => {
                    let bit = bit_reader.read_bits(1)?;

                    cur_node = if bit == 0 {
                        left
                    } else {
                        right
                    };
                    bits_count += 1;
                }
            }
        }
        bit_reader.decrease_pos(bits_count)?;
        Err(BrotliError::HuffmanTreeNotMatch)
    }
}
