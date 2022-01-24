use std::io::Read;

pub struct XorRead<R, F> {
    inner: R,
    xor_lookup: F,
    index: usize,
}

impl<R: Read, F: Fn(usize) -> u8> XorRead<R, F> {
    pub fn new(reader: R, xor_lookup: F) -> Self {
        Self {
            inner: reader,
            xor_lookup,
            index: 0,
        }
    }
}

impl<R: Read, F: Fn(usize) -> u8> Read for XorRead<R, F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_amt = self.inner.read(buf)?;
        for b in &mut buf[..read_amt] {
            *b ^= (self.xor_lookup)(self.index);
            self.index += 1;
        }
        Ok(read_amt)
    }
}
