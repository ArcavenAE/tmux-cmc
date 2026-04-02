/// A completed response from tmux to a sent command.
#[derive(Debug, Clone)]
pub struct Response {
    pub serial: u64,
    pub flags: u32,
    /// Lines between `%begin` and `%end`. Empty for commands with no output.
    pub output: Vec<String>,
    /// True if tmux returned `%error` instead of `%end`.
    pub is_error: bool,
}

impl Response {
    /// The first output line, if any.
    pub fn first_line(&self) -> Option<&str> {
        self.output.first().map(String::as_str)
    }

    /// All output joined by newlines.
    pub fn text(&self) -> String {
        self.output.join("\n")
    }
}
