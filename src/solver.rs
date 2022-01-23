use crate::server::{self, Server};
use crate::{GuessOutcome, LetterOutcome, Word};
use rand::seq::IteratorRandom;
use std::collections::HashSet;

pub struct Solver {
    guess_index: usize,
    guess_outcomes: [Option<GuessOutcome>; 6],
    letters_state: [LetterState; 26],
    dictionary: HashSet<Word>,
}

impl Solver {
    pub fn new(dictionary: HashSet<Word>) -> Self {
        Self {
            guess_index: 0,
            guess_outcomes: [None; 6],
            letters_state: [LetterState::Unknown; 26],
            dictionary,
        }
    }

    pub fn guess(&mut self, server: &mut Server) -> Result<(Word, GuessOutcome), Error> {
        // Select a random word still in the dictionary
        let mut rng = rand::thread_rng();
        let guess = *self
            .dictionary
            .iter()
            .choose(&mut rng)
            .ok_or(Error::Stumped)?;
        self.dictionary.remove(&guess);

        let outcome = server.submit(guess)?;
        self.guess_outcomes[self.guess_index] = Some(outcome);
        self.guess_index += 1;

        // Update knowledge about the letters
        for (i, (x, y)) in guess.iter().zip(outcome.iter()).enumerate() {
            let j = x.index();
            match y {
                LetterOutcome::Absent => self.letters_state[j as usize] = LetterState::Absent,
                LetterOutcome::Present => match self.letters_state[j as usize] {
                    LetterState::Unknown | LetterState::Absent => {
                        let mut ps = [PositionState::Maybe; 5];
                        ps[i] = PositionState::No;
                        self.letters_state[j as usize] = LetterState::Positions(ps);
                    }
                    LetterState::Positions(mut ps) => {
                        ps[i] = PositionState::No;
                    }
                },
                LetterOutcome::Correct => match self.letters_state[j as usize] {
                    LetterState::Unknown | LetterState::Absent => {
                        let mut ps = [PositionState::Maybe; 5];
                        ps[i] = PositionState::Yes;
                        self.letters_state[j as usize] = LetterState::Positions(ps);
                    }
                    LetterState::Positions(mut ps) => {
                        ps[i] = PositionState::Yes;
                    }
                },
            }
        }

        // Filter dictionary based on information
        let state = &self.letters_state;
        self.dictionary.retain(|w| satisfies(w, state));

        Ok((guess, outcome))
    }
}

fn satisfies(word: &Word, state: &[LetterState; 26]) -> bool {
    for (i, l) in word.iter().enumerate() {
        let j = l.index();
        match state[j as usize] {
            LetterState::Absent => return false,
            LetterState::Unknown => (), // not sure
            LetterState::Positions(ps) => {
                match ps[i] {
                    PositionState::Yes => (),   // definitely right
                    PositionState::Maybe => (), // not sure
                    PositionState::No => return false,
                }
            }
        }
    }
    true
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LetterState {
    Unknown,
    Positions([PositionState; 5]),
    Absent,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PositionState {
    Yes,
    Maybe,
    No,
}

pub enum Error {
    Stumped,
    Server(server::Error),
}

impl From<server::Error> for Error {
    fn from(e: server::Error) -> Self {
        Self::Server(e)
    }
}

#[cfg(test)]
mod tests {
    // TODO
}
