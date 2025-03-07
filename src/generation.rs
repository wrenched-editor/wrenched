/// Opaque representation of a generation.
///
/// Obtained from [`PlainEditor::generation`].
// Overflow handling: the generations are only compared,
// so wrapping is fine. This could only fail if exactly
// `u32::MAX` generations happen between drawing
// operations. This is implausible and so can be ignored.
#[derive(PartialEq, Eq, Default, Clone, Copy, Debug)]
pub struct Generation(u32);

impl Generation {
    /// Make it not what it currently is.
    pub fn nudge(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
