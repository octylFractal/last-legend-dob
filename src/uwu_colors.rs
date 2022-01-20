use owo_colors::{OwoColorize, Style, Styled};
use supports_color::Stream::Stderr;

pub trait ErrStyle {
    fn errstyle(&self, style: Style) -> Styled<&Self>;
}

impl<D> ErrStyle for D {
    fn errstyle(&self, style: Style) -> Styled<&Self> {
        self.style(
            supports_color::on(Stderr)
                .filter(|f| f.has_basic)
                .map_or_else(Style::new, |_| style),
        )
    }
}
