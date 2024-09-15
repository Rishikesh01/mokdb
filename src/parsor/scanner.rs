#[derive(Debug)]
pub struct Scanner {
    source: String,
    start_line: i64,
    currnet_line: i64,
}

impl Scanner {
    fn scanner(mut self, source: String) {
        self.source = source;
    }
}

