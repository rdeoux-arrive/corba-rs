use core::fmt::{Display, Formatter, Result};

#[derive(Default)]
pub struct Indent(usize);

impl Indent {
    pub const fn push(&mut self) {
        self.0 += 1;
    }

    pub const fn pop(&mut self) {
        self.0 -= 1;
    }
}

impl Display for Indent {
    fn fmt(&self, f: &mut Formatter) -> Result {
        for _ in 0..self.0 {
            write!(f, "  ")?;
        }
        Ok(())
    }
}
