use crate::{GuessOutcome, Letter, LetterOutcome, Word};
use std::collections::HashSet;

pub trait Server {
    fn can_guess(&self) -> bool;
    fn submit(&mut self, guess: Word) -> Result<GuessOutcome, Error>;
}

pub struct InMemoryServer {
    answer: Word,
    in_answer: [bool; 26],
    guess_index: usize,
    guesses: [Option<Word>; 6],
    dictionary: HashSet<Word>,
}

impl InMemoryServer {
    pub fn new(answer: Word, dictionary: HashSet<Word>) -> Self {
        let mut in_answer = [false; 26];
        for c in answer.iter() {
            in_answer[c.index() as usize] = true;
        }
        Self {
            answer,
            in_answer,
            guess_index: 0,
            guesses: [None; 6],
            dictionary,
        }
    }

    fn answer_contains_letter(&self, l: &Letter) -> bool {
        self.in_answer[l.index() as usize]
    }
}

impl Server for InMemoryServer {
    fn can_guess(&self) -> bool {
        self.guess_index < 6
    }

    fn submit(&mut self, guess: Word) -> Result<GuessOutcome, Error> {
        if !self.can_guess() {
            return Err(Error::GameOver);
        }
        if self.guesses[..self.guess_index].contains(&Some(guess)) {
            return Err(Error::AlreadyGuessed);
        }
        if !self.dictionary.contains(&guess) {
            return Err(Error::InvalidWord);
        }
        self.guesses[self.guess_index] = Some(guess);
        self.guess_index += 1;

        let mut result = GuessOutcome::default();
        for (i, (x, y)) in guess.into_iter().zip(self.answer.into_iter()).enumerate() {
            if x == y {
                result[i] = LetterOutcome::Correct;
            } else if self.answer_contains_letter(&x) {
                result[i] = LetterOutcome::Present;
            } else {
                result[i] = LetterOutcome::Absent;
            }
        }

        Ok(result)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    GameOver,
    AlreadyGuessed,
    InvalidWord,
}

#[cfg(test)]
mod tests {
    use crate::{
        server::{self, InMemoryServer, Server},
        LetterOutcome, Word,
    };

    #[test]
    fn test_guess_submit() {
        let word = Word::try_from_str("trees").unwrap();
        let dictionary = vec!["river", "abbey", "crave", "kings", "great", "trees"]
            .into_iter()
            .map(|s| Word::try_from_str(s).unwrap())
            .collect();
        let mut server = InMemoryServer::new(word, dictionary);

        let guess = Word::try_from_str("river").unwrap();
        let result = server.submit(guess).unwrap();
        assert_eq!(
            result,
            [
                LetterOutcome::Present,
                LetterOutcome::Absent,
                LetterOutcome::Absent,
                LetterOutcome::Correct,
                LetterOutcome::Present,
            ]
        );

        assert_eq!(server.submit(guess), Err(server::Error::AlreadyGuessed),);

        let guess = Word::try_from_str("ghwsd").unwrap();
        assert_eq!(server.submit(guess), Err(server::Error::InvalidWord),);

        let guess = Word::try_from_str("abbey").unwrap();
        let result = server.submit(guess).unwrap();
        assert_eq!(
            result,
            [
                LetterOutcome::Absent,
                LetterOutcome::Absent,
                LetterOutcome::Absent,
                LetterOutcome::Correct,
                LetterOutcome::Absent,
            ]
        );

        let guess = Word::try_from_str("crave").unwrap();
        let result = server.submit(guess).unwrap();
        assert_eq!(
            result,
            [
                LetterOutcome::Absent,
                LetterOutcome::Correct,
                LetterOutcome::Absent,
                LetterOutcome::Absent,
                LetterOutcome::Present,
            ]
        );

        let guess = Word::try_from_str("kings").unwrap();
        let result = server.submit(guess).unwrap();
        assert_eq!(
            result,
            [
                LetterOutcome::Absent,
                LetterOutcome::Absent,
                LetterOutcome::Absent,
                LetterOutcome::Absent,
                LetterOutcome::Correct,
            ]
        );

        let guess = Word::try_from_str("great").unwrap();
        let result = server.submit(guess).unwrap();
        assert_eq!(
            result,
            [
                LetterOutcome::Absent,
                LetterOutcome::Correct,
                LetterOutcome::Correct,
                LetterOutcome::Absent,
                LetterOutcome::Present,
            ]
        );

        let result = server.submit(word).unwrap();
        assert_eq!(
            result,
            [
                LetterOutcome::Correct,
                LetterOutcome::Correct,
                LetterOutcome::Correct,
                LetterOutcome::Correct,
                LetterOutcome::Correct,
            ]
        );

        let result = server.submit(guess);
        assert_eq!(result, Err(server::Error::GameOver),);
    }
}
