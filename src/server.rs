use crate::{GuessOutcome, LetterOutcome, Word};
use std::collections::HashSet;
use std::fmt;

pub trait Server {
    fn can_guess(&self) -> bool;
    fn submit(&mut self, guess: Word) -> Result<GuessOutcome, Error>;
}

pub struct InMemoryServer {
    answer: Word,
    count_in_answer: [u8; 26],
    guess_index: usize,
    guesses: [Option<Word>; 6],
    dictionary: HashSet<Word>,
}

impl fmt::Debug for InMemoryServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Dictionary intentionally left off because it is never modified
        f.debug_struct("InMemoryServer")
            .field("answer", &self.answer)
            .field("count_in_answer", &self.count_in_answer)
            .field("guess_index", &self.guess_index)
            .field("guesses", &self.guesses)
            .finish()
    }
}

impl InMemoryServer {
    pub fn new(answer: Word, dictionary: HashSet<Word>) -> Self {
        let mut count_in_answer = [0; 26];
        for c in answer.iter() {
            count_in_answer[c.index() as usize] += 1;
        }
        Self {
            answer,
            count_in_answer,
            guess_index: 0,
            guesses: [None; 6],
            dictionary,
        }
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
        let mut correct_count_in_guess = [0u8; 26];
        // In the first pass, find all the correct letters
        for (i, (x, y)) in guess.iter().zip(self.answer.iter()).enumerate() {
            if x == y {
                result[i] = LetterOutcome::Correct;
                correct_count_in_guess[x.index() as usize] += 1;
            }
        }
        // In the second pass, set present or absent only based
        // on the non-correct positions
        for (i, x) in guess.into_iter().enumerate() {
            if result[i] == LetterOutcome::Correct {
                continue;
            }
            let j = x.index() as usize;
            if self.count_in_answer[j] - correct_count_in_guess[j] == 0 {
                result[i] = LetterOutcome::Absent;
            } else {
                result[i] = LetterOutcome::Present;
                correct_count_in_guess[j] += 1;
            }
        }

        Ok(result)
    }
}

pub struct InteractiveServer;

impl Server for InteractiveServer {
    fn can_guess(&self) -> bool {
        true
    }

    fn submit(&mut self, guess: Word) -> Result<GuessOutcome, Error> {
        println!("Guess: {:?}", guess);

        let mut input = String::with_capacity(5);
        let mut outcome = [LetterOutcome::Absent; 5];
        loop {
            input.clear();
            if let Err(_) = std::io::stdin().read_line(&mut input) {
                println!("Some error occurred, try again.");
            }
            let trimmed = input.trim();

            let mut parse_err = false;
            for (i, b) in trimmed.bytes().enumerate() {
                if i == 5 {
                    break;
                }
                match b {
                    b'*' => outcome[i] = LetterOutcome::Correct,
                    b'+' => outcome[i] = LetterOutcome::Present,
                    b'-' => outcome[i] = LetterOutcome::Absent,
                    b'!' => return Err(Error::GameOver),
                    _ => {
                        println!(
                            "Unrecognized character, use only *=correct +=present -=absent !=game_over"
                        );
                        parse_err = true;
                        break;
                    }
                }
            }

            if trimmed.len() < 5 {
                println!("Input too short, try again.");
                parse_err = true;
            } else if trimmed.len() > 5 {
                println!("Input too long, try again.");
                parse_err = true;
            }

            if !parse_err {
                break;
            }
        }
        Ok(outcome)
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
        GuessOutcome, LetterOutcome, Word,
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
                LetterOutcome::Absent,
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

    #[test]
    fn test_duplicate_letters_in_guess() {
        fn to_str(xs: &[u8]) -> &str {
            std::str::from_utf8(xs).unwrap()
        }
        let word = Word::try_from_str("whack").unwrap();
        let dictionary = vec!["whack", "audio", "snake", "track", "clack"]
            .into_iter()
            .map(|s| Word::try_from_str(s).unwrap())
            .collect();
        let mut server = InMemoryServer::new(word, dictionary);

        let guess = Word::try_from_str("audio").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "+----",);

        let guess = Word::try_from_str("snake").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "--*+-",);

        let guess = Word::try_from_str("track").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "--***",);

        // Note the first 'c' is considered absent because the second
        // 'c' is already in the correct position and there is only one
        // 'c' in the word.
        let guess = Word::try_from_str("clack").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "--***",);

        let guess = Word::try_from_str("whack").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "*****",);

        let word = Word::try_from_str("whack").unwrap();
        let dictionary = vec!["whack", "cacao"]
            .into_iter()
            .map(|s| Word::try_from_str(s).unwrap())
            .collect();
        let mut server = InMemoryServer::new(word, dictionary);

        // The first 'c' is considered present because there is 1 'c' in the answer,
        // but the second 'c' is considered absent because there are not two.
        // Similarly for the 'a's.
        let guess = Word::try_from_str("cacao").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "++---",);
    }

    #[test]
    fn test_duplicate_letters_in_answer() {
        fn to_str(xs: &[u8]) -> &str {
            std::str::from_utf8(xs).unwrap()
        }
        let word = Word::try_from_str("dwell").unwrap();
        let dictionary = vec!["dwell", "audio", "dense", "dryer"]
            .into_iter()
            .map(|s| Word::try_from_str(s).unwrap())
            .collect();
        let mut server = InMemoryServer::new(word, dictionary);

        let guess = Word::try_from_str("audio").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "--+--",);

        let guess = Word::try_from_str("dense").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "*+---",);

        let guess = Word::try_from_str("dryer").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "*--+-",);

        let guess = Word::try_from_str("dwell").unwrap();
        let outcome = server.submit(guess).unwrap();
        assert_eq!(to_str(&guess_outcome_to_ascii(outcome)), "*****",);
    }

    fn guess_outcome_to_ascii(g: GuessOutcome) -> [u8; 5] {
        crate::util::map_array(g, |l| match l {
            LetterOutcome::Absent => b'-',
            LetterOutcome::Present => b'+',
            LetterOutcome::Correct => b'*',
        })
    }
}
