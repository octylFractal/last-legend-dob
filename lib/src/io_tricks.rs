use std::io::Write;
use std::sync::{Arc, Mutex};

#[auto_enums::enum_derive(Read)]
pub enum ReadMixer<L, R> {
    Wrapped(L),
    Plain(R),
}

pub struct ArcWrite<W>(Arc<Mutex<W>>);

impl<W: Write> ArcWrite<W> {
    pub fn new(writer: Arc<Mutex<W>>) -> Self {
        Self(writer)
    }
}

impl<W: Write> Write for ArcWrite<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut lock = self.0.lock().expect("lock poisoned");
        lock.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut lock = self.0.lock().expect("lock poisoned");
        lock.flush()
    }
}
