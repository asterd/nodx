// RFC 1951 DEFLATE decoder. Minimal, safe, no allocations beyond the output buffer
// and a per-block code-length table. Bounded by `max_output` to prevent zip bombs.

pub fn inflate(input: &[u8], max_output: usize) -> Result<Vec<u8>, &'static str> {
    let mut reader = BitReader::new(input);
    let mut out = Vec::new();
    loop {
        let bfinal = reader.read_bits(1)? == 1;
        let btype = reader.read_bits(2)?;
        match btype {
            0 => inflate_stored(&mut reader, &mut out, max_output)?,
            1 => inflate_fixed(&mut reader, &mut out, max_output)?,
            2 => inflate_dynamic(&mut reader, &mut out, max_output)?,
            _ => return Err("invalid DEFLATE block type"),
        }
        if bfinal {
            return Ok(out);
        }
    }
}

fn inflate_stored(
    reader: &mut BitReader,
    out: &mut Vec<u8>,
    max_output: usize,
) -> Result<(), &'static str> {
    reader.align_to_byte();
    let len = reader.read_u16_le()? as usize;
    let nlen = reader.read_u16_le()?;
    if (len as u16) ^ nlen != 0xffff {
        return Err("stored block LEN/NLEN mismatch");
    }
    if out.len().checked_add(len).ok_or("output overflow")? > max_output {
        return Err("output exceeds limit");
    }
    for _ in 0..len {
        out.push(reader.read_byte()?);
    }
    Ok(())
}

fn inflate_fixed(
    reader: &mut BitReader,
    out: &mut Vec<u8>,
    max_output: usize,
) -> Result<(), &'static str> {
    let lit = fixed_literal_table();
    let dist = fixed_distance_table();
    inflate_huffman_block(reader, out, max_output, &lit, &dist)
}

fn inflate_dynamic(
    reader: &mut BitReader,
    out: &mut Vec<u8>,
    max_output: usize,
) -> Result<(), &'static str> {
    let hlit = reader.read_bits(5)? as usize + 257;
    let hdist = reader.read_bits(5)? as usize + 1;
    let hclen = reader.read_bits(4)? as usize + 4;
    if hlit > 286 || hdist > 30 || hclen > 19 {
        return Err("dynamic block header out of range");
    }
    let order = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut code_len_lens = [0u8; 19];
    for i in 0..hclen {
        code_len_lens[order[i]] = reader.read_bits(3)? as u8;
    }
    let cl_table = build_huffman(&code_len_lens, 19)?;
    let total = hlit + hdist;
    let mut lengths = vec![0u8; total];
    let mut i = 0;
    while i < total {
        let symbol = decode_symbol(reader, &cl_table)?;
        match symbol {
            0..=15 => {
                lengths[i] = symbol as u8;
                i += 1;
            }
            16 => {
                if i == 0 {
                    return Err("invalid code length repeat at start");
                }
                let repeat = reader.read_bits(2)? as usize + 3;
                if i + repeat > total {
                    return Err("code length repeat overflow");
                }
                let prev = lengths[i - 1];
                for _ in 0..repeat {
                    lengths[i] = prev;
                    i += 1;
                }
            }
            17 => {
                let repeat = reader.read_bits(3)? as usize + 3;
                if i + repeat > total {
                    return Err("code length zeros overflow");
                }
                for _ in 0..repeat {
                    lengths[i] = 0;
                    i += 1;
                }
            }
            18 => {
                let repeat = reader.read_bits(7)? as usize + 11;
                if i + repeat > total {
                    return Err("code length zeros overflow");
                }
                for _ in 0..repeat {
                    lengths[i] = 0;
                    i += 1;
                }
            }
            _ => return Err("invalid code length symbol"),
        }
    }
    let lit_table = build_huffman(&lengths[..hlit], 286)?;
    let dist_table = build_huffman(&lengths[hlit..], 30)?;
    inflate_huffman_block(reader, out, max_output, &lit_table, &dist_table)
}

fn inflate_huffman_block(
    reader: &mut BitReader,
    out: &mut Vec<u8>,
    max_output: usize,
    lit: &HuffmanTable,
    dist: &HuffmanTable,
) -> Result<(), &'static str> {
    const LENGTH_BASE: [u16; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LENGTH_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DISTANCE_BASE: [u16; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DISTANCE_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];
    loop {
        let symbol = decode_symbol(reader, lit)?;
        if symbol < 256 {
            if out.len() == max_output {
                return Err("output exceeds limit");
            }
            out.push(symbol as u8);
        } else if symbol == 256 {
            return Ok(());
        } else if symbol <= 285 {
            let idx = symbol as usize - 257;
            let length = LENGTH_BASE[idx] as usize + reader.read_bits(LENGTH_EXTRA[idx])? as usize;
            let dist_symbol = decode_symbol(reader, dist)?;
            if dist_symbol >= 30 {
                return Err("invalid distance symbol");
            }
            let didx = dist_symbol as usize;
            let distance = DISTANCE_BASE[didx] as usize
                + reader.read_bits(DISTANCE_EXTRA[didx])? as usize;
            if distance == 0 || distance > out.len() {
                return Err("invalid back reference");
            }
            if out.len().checked_add(length).ok_or("output overflow")? > max_output {
                return Err("output exceeds limit");
            }
            let start = out.len() - distance;
            for i in 0..length {
                let b = out[start + i];
                out.push(b);
            }
        } else {
            return Err("invalid literal/length symbol");
        }
    }
}

fn fixed_literal_table() -> HuffmanTable {
    let mut lengths = [0u8; 288];
    for item in lengths.iter_mut().take(143 + 1) {
        *item = 8;
    }
    for item in lengths.iter_mut().take(255 + 1).skip(144) {
        *item = 9;
    }
    for item in lengths.iter_mut().take(279 + 1).skip(256) {
        *item = 7;
    }
    for item in lengths.iter_mut().skip(280) {
        *item = 8;
    }
    build_huffman(&lengths, 288).expect("fixed literal table")
}

fn fixed_distance_table() -> HuffmanTable {
    let lengths = [5u8; 30];
    build_huffman(&lengths, 30).expect("fixed distance table")
}

struct HuffmanTable {
    codes: Vec<(u16, u8, u16)>, // (code, length, symbol)
    max_len: u8,
}

fn build_huffman(lengths: &[u8], expected: usize) -> Result<HuffmanTable, &'static str> {
    if lengths.len() > expected {
        return Err("huffman table too long");
    }
    let max_len = *lengths.iter().max().unwrap_or(&0);
    if max_len == 0 {
        return Ok(HuffmanTable {
            codes: Vec::new(),
            max_len: 0,
        });
    }
    if max_len > 15 {
        return Err("huffman code length > 15");
    }
    let mut bl_count = [0u32; 16];
    for &l in lengths {
        if l > 0 {
            bl_count[l as usize] += 1;
        }
    }
    let mut next_code = [0u32; 16];
    let mut code = 0u32;
    for bits in 1..=15 {
        code = (code + bl_count[bits - 1]) << 1;
        next_code[bits] = code;
    }
    let mut codes = Vec::new();
    for (symbol, &len) in lengths.iter().enumerate() {
        if len > 0 {
            let c = next_code[len as usize];
            next_code[len as usize] += 1;
            codes.push((c as u16, len, symbol as u16));
        }
    }
    codes.sort_by_key(|&(c, l, _)| (l, c));
    Ok(HuffmanTable { codes, max_len })
}

fn decode_symbol(reader: &mut BitReader, table: &HuffmanTable) -> Result<u16, &'static str> {
    if table.codes.is_empty() {
        return Err("empty huffman table");
    }
    let mut code: u32 = 0;
    for bits in 1..=table.max_len {
        let bit = reader.read_bits(1)?;
        code = (code << 1) | bit;
        for &(c, l, s) in &table.codes {
            if l == bits && c as u32 == code {
                return Ok(s);
            }
        }
    }
    Err("no huffman match")
}

struct BitReader<'a> {
    bytes: &'a [u8],
    pos: usize,
    buffer: u64,
    buffered: u8,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            buffer: 0,
            buffered: 0,
        }
    }

    fn read_bits(&mut self, count: u8) -> Result<u32, &'static str> {
        if count == 0 {
            return Ok(0);
        }
        while self.buffered < count {
            if self.pos >= self.bytes.len() {
                return Err("unexpected end of DEFLATE stream");
            }
            self.buffer |= (self.bytes[self.pos] as u64) << self.buffered;
            self.pos += 1;
            self.buffered += 8;
        }
        let mask = (1u64 << count) - 1;
        let value = (self.buffer & mask) as u32;
        self.buffer >>= count;
        self.buffered -= count;
        Ok(value)
    }

    fn read_byte(&mut self) -> Result<u8, &'static str> {
        if !self.buffered.is_multiple_of(8) {
            self.buffer >>= self.buffered % 8;
            self.buffered -= self.buffered % 8;
        }
        if self.buffered >= 8 {
            let v = (self.buffer & 0xff) as u8;
            self.buffer >>= 8;
            self.buffered -= 8;
            return Ok(v);
        }
        if self.pos >= self.bytes.len() {
            return Err("unexpected end of DEFLATE stream");
        }
        let v = self.bytes[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_u16_le(&mut self) -> Result<u16, &'static str> {
        let lo = self.read_byte()? as u16;
        let hi = self.read_byte()? as u16;
        Ok(lo | (hi << 8))
    }

    fn align_to_byte(&mut self) {
        let drop = self.buffered % 8;
        self.buffer >>= drop;
        self.buffered -= drop;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_block_roundtrip() {
        // empty final stored block: bfinal=1, btype=00, padding, len=0, nlen=0xffff
        let data = [0x01u8, 0x00, 0x00, 0xff, 0xff];
        assert_eq!(inflate(&data, 1024).unwrap(), Vec::<u8>::new());
    }
}
