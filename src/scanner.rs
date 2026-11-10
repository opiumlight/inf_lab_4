pub struct Scanner {
    chars: Vec<char>,
    pos: usize,
}


impl Scanner {
    pub fn new(chars: &str) -> Scanner {
        Scanner { chars: chars.chars().collect::<Vec<char>>(), pos: 0 }
    }
    pub fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    pub fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }
    pub fn expect(&mut self, want: char) -> Result<(), String> {
        let pos = self.pos;
        match self.peek() {
            Some(c) if c == want => {self.bump(); Ok(())},
            Some(c) => Err(format!("expected {}, got {} at pos {}", want, c, pos)),
            None => Err(format!("expected {}, got <EOF> at pos {}", want, pos)),
        }
    }
    pub fn starts_with(&self, s: &str) -> bool {
        self.chars[self.pos..].starts_with(s.chars().collect::<Vec<char>>().as_slice())
    }
    pub fn eat(&mut self, c: char) -> bool {
        if self.peek() == Some(c) {
            self.bump();
            return true;
        }
        false
    }
    pub fn eat_str(&mut self, s: &str) -> bool {
        if self.starts_with(s) {
            self.pos += s.chars().count();
            return true;
        }
        false
    }
    pub fn mark(&self) -> usize {
        self.pos
    }
    pub fn reset(&mut self, mark: usize) {
        self.pos = mark;
    }
    pub fn slice_from(&self, mark: usize) -> String {
        self.chars[mark..self.pos].iter().collect::<String>()
    }
    pub fn pos(&self) -> usize {
        self.pos
    }
}
