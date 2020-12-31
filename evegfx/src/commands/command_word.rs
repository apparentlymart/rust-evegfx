pub(crate) struct CommandWord(u32);

impl CommandWord {
    pub const fn to_raw(&self) -> u32 {
        self.0
    }
}

impl From<u32> for CommandWord {
    #[inline]
    fn from(v: u32) -> Self {
        CommandWord(v)
    }
}

impl From<i32> for CommandWord {
    #[inline]
    fn from(v: i32) -> Self {
        CommandWord(v as u32)
    }
}

impl From<(u16, u16)> for CommandWord {
    #[inline]
    fn from(v: (u16, u16)) -> Self {
        CommandWord((v.0 as u32) | (v.1 as u32) << 16)
    }
}

impl From<(i16, i16)> for CommandWord {
    #[inline]
    fn from(v: (i16, i16)) -> Self {
        let a = v.0 as i32;
        let b = v.1 as i32;
        CommandWord((a as u32) | (b as u32) << 16)
    }
}

impl From<(u8, u8, u8, u8)> for CommandWord {
    #[inline]
    fn from(v: (u8, u8, u8, u8)) -> Self {
        CommandWord((v.0 as u32) | (v.1 as u32) << 8 | (v.2 as u32) << 16 | (v.3 as u32) << 24)
    }
}

pub(crate) fn command_words_for_bytes<'a>(bytes: &'a [u8]) -> ByteToCommandIter<'a> {
    ByteToCommandIter { remain: bytes }
}

pub(crate) struct ByteToCommandIter<'a> {
    remain: &'a [u8],
}

impl<'a> Iterator for ByteToCommandIter<'a> {
    type Item = CommandWord;

    fn next(&mut self) -> core::option::Option<Self::Item> {
        const SIZE: usize = core::mem::size_of::<u32>();

        let mut raw: u32 = 0;
        let mut len = self.remain.len();
        if len == 0 {
            return None;
        }
        if len > SIZE {
            len = SIZE;
        }
        for i in 0..len {
            raw = raw | ((self.remain[i] as u32) << (i * 8));
        }
        self.remain = &self.remain[len..];
        Some(CommandWord(raw))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<'a> ExactSizeIterator for ByteToCommandIter<'a> {
    fn len(&self) -> usize {
        const SIZE: usize = core::mem::size_of::<u32>();

        // This is ceil(len / 4), accounting for us rounding up to include
        // alignment bytes.
        (self.remain.len() + (SIZE - 1)) / SIZE
    }
}

impl<'a> core::iter::FusedIterator for ByteToCommandIter<'a> {}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;

    #[test]
    fn test_byte_to_command_iter_exact() {
        use std::vec::Vec;
        let bytes: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut word_vec: Vec<u32> = Vec::new();
        for w in command_words_for_bytes(&bytes[..]) {
            word_vec.push(w.to_raw());
        }
        debug_assert_eq!(word_vec.as_slice(), [0x04030201, 0x08070605]);
    }

    #[test]
    fn test_byte_to_command_iter_pad() {
        use std::vec::Vec;
        let bytes: [u8; 6] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let mut word_vec: Vec<u32> = Vec::new();
        for w in command_words_for_bytes(&bytes[..]) {
            word_vec.push(w.to_raw());
        }
        debug_assert_eq!(word_vec.as_slice(), [0x04030201, 0x00000605]);
    }

    #[test]
    fn test_byte_to_command_iter_empty() {
        use std::vec::Vec;
        let bytes: [u8; 0] = [];
        let mut word_vec: Vec<u32> = Vec::new();
        for w in command_words_for_bytes(&bytes[..]) {
            word_vec.push(w.to_raw());
        }
        debug_assert_eq!(word_vec.as_slice(), []);
    }
}
