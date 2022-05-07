#[auto_enums::enum_derive(Read)]
pub enum ReadMixer<L, R> {
    Wrapped(L),
    Plain(R),
}
