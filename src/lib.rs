use std::fmt;

pub mod server;
pub mod solver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterOutcome {
    Correct, // That letter is in that position
    Present, // That letter is in the word, but not in that position
    Absent,  // That letter is not in the word
}

impl Default for LetterOutcome {
    fn default() -> Self {
        Self::Absent
    }
}

pub type GuessOutcome = [LetterOutcome; 5];

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Letter(u8);

impl Letter {
    pub const LETTERS: [Self; 26] = [
        Letter(b'a'),
        Letter(b'b'),
        Letter(b'c'),
        Letter(b'd'),
        Letter(b'e'),
        Letter(b'f'),
        Letter(b'g'),
        Letter(b'h'),
        Letter(b'i'),
        Letter(b'j'),
        Letter(b'k'),
        Letter(b'l'),
        Letter(b'm'),
        Letter(b'n'),
        Letter(b'o'),
        Letter(b'p'),
        Letter(b'q'),
        Letter(b'r'),
        Letter(b's'),
        Letter(b't'),
        Letter(b'u'),
        Letter(b'v'),
        Letter(b'w'),
        Letter(b'x'),
        Letter(b'y'),
        Letter(b'z'),
    ];

    pub const VOWELS: [Self; 5] = [
        Letter(b'a'),
        Letter(b'e'),
        Letter(b'i'),
        Letter(b'o'),
        Letter(b'u'),
    ];

    pub const fn new(c: u8) -> Option<Self> {
        if !c.is_ascii_alphabetic() {
            return None;
        }
        Some(Self(c.to_ascii_lowercase()))
    }

    pub const fn index(&self) -> u8 {
        self.0 - Self::LETTERS[0].0
    }
}

impl Default for Letter {
    fn default() -> Self {
        Self(b'a')
    }
}

impl fmt::Debug for Letter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = self.0 as char;
        f.debug_tuple("Letter").field(&c).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Word([Letter; 5]);

impl Word {
    pub fn try_from_str(s: &str) -> Option<Self> {
        if s.len() != 5 {
            return None;
        }

        let mut result = [Letter(0); 5];
        for (i, c) in s.bytes().enumerate() {
            result[i] = Letter::new(c)?;
        }
        Some(Self(result))
    }

    pub fn iter(&self) -> std::slice::Iter<Letter> {
        self.0.iter()
    }

    pub fn contains(&self, letter: &Letter) -> bool {
        self.iter().any(|l| l == letter)
    }

    pub fn count(&self, letter: &Letter) -> u8 {
        self.iter()
            .map(|l| if l == letter { 1u8 } else { 0u8 })
            .sum()
    }

    pub fn distinct_vowels(&self) -> u8 {
        let mut contains = [0u8; 5];
        for l in self.iter() {
            if let Some(i) = Letter::VOWELS.iter().position(|v| v == l) {
                contains[i] |= 1;
            }
        }
        contains.into_iter().sum()
    }
}

impl IntoIterator for Word {
    type Item = Letter;

    type IntoIter = std::array::IntoIter<Letter, 5>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub(crate) mod util {
    pub(crate) fn map_array<T, U, F, const N: usize>(xs: [T; N], f: F) -> [U; N]
    where
        T: Sized,
        U: Sized + Default + Copy,
        F: Fn(T) -> U,
    {
        let mut result = [Default::default(); N];
        for (i, t) in xs.into_iter().enumerate() {
            result[i] = f(t)
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::{util, Letter, Word};

    #[test]
    fn test_letters() {
        for c in 'a'..='z' {
            assert_eq!(
                format!("{:?}", Letter::new(c as u8).unwrap()),
                format!("Letter({:?})", c),
            );
        }

        for c in 'A'..='Z' {
            let l = c.to_ascii_lowercase();
            assert_eq!(
                format!("{:?}", Letter::new(c as u8).unwrap()),
                format!("Letter({:?})", l),
            );
        }

        for x in 0u8..=255u8 {
            if (b'A'..=b'z').contains(&x) {
                continue;
            }
            assert_eq!(Letter::new(x), None,);
        }
    }

    #[test]
    fn test_word_from_str() {
        assert_eq!(
            Word::try_from_str("River"),
            Some(Word(util::map_array(
                [b'r', b'i', b'v', b'e', b'r'],
                Letter
            ))),
        );

        // Longer than 5 bytes
        assert_eq!(Word::try_from_str("TooLong"), None,);

        // Spaces don't parse into letters
        assert_eq!(Word::try_from_str("AB CD"), None,);

        // Numbers don't parse into letters
        assert_eq!(Word::try_from_str("ABCD1"), None,);
    }
}
