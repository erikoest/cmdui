use rustyline::hint::Hinter;
use rustyline::Helper;
use rustyline::{CompletionType, Context, Editor, Config};
use rustyline::completion::{Completer, Pair};
use rustyline::validate::{Validator, ValidationResult, ValidationContext};
use rustyline::highlight::{Highlighter};
use rustyline::error::ReadlineError;
extern crate term_size;

use std::collections::HashMap;
use std::ops::{Range, RangeFrom};
use console::{Term, Key};
use std::cmp::min;
use std::io;
use std::io::stdin;
use std::io::Write;
use std::fs;

pub trait KeywordExpander {
    fn command_list<'a>(&self) -> &'a [&'a str];

    fn expand_keyword(&self, cp: &CommandPart, parts: &Vec<String>)
                      -> Vec<String>;

    fn expand_filename(&self, path: &str) -> Vec<String> {
        let mut ret = vec!();
        let (dir, dpart, fpart);

        if let Some(pos) = path.rfind('/') {
            dir = &path[0..pos + 1];
            dpart = &path[0..pos + 1];
            fpart = &path[pos + 1..];
        }
        else if path == "." {
            dir = "./";
            dpart = ".";
            fpart = "";
        }
        else {
            dir = "./";
            dpart = "";
            fpart = path;
        }

        if let Ok(d) = fs::read_dir(dir) {
            for entry_result in d {
                let entry = entry_result.unwrap();
                if let Some(fstr) = entry.file_name().to_str() {
                    if fstr.starts_with(fpart) {
                        let mut fstring = fstr.to_string();
                        if entry.path().is_dir() {
                            fstring.push('/');
                        }

                        ret.push(format!("{}{}", dpart, fstring));
                    }
                }
            }
        }

        return ret;
    }
}

pub trait CmdApp {
    // Mandatory methods
    fn command_list<'a>(&self) -> &'a [&'a str];

    fn execute_line(&mut self, cmd: &str, args: &Vec<String>)
                    -> Result<(), String>;

    // Optional callbacks
    fn startup(&mut self) { }

    fn exit(&mut self) { }

    // Helper methods
    fn confirm_yes_no(&self) -> bool {
        let mut buf = String::new();
        stdin().read_line(&mut buf).unwrap();
        return buf.trim().to_lowercase() == "y" || buf.trim().len() == 0;
    }

    // Wait-for-keypress, for the pager function.
    fn wait_for_key(&self) -> Key {
        let term = Term::stdout();
        return term.read_key().unwrap();
    }

    // Pager function. Lists words in columns, one page at a time.
    fn print_columns(&self, lines: &[String], max_line: usize) {
        let (term_w, term_h) = term_size::dimensions().unwrap_or((80, 25));
        let min_padding = 2;
        let cols = term_w/(max_line + min_padding);
        let cwidth = term_w/cols;
        let end_row = (lines.len() + cols - 1) / cols;
        let page_size = term_h - 1;
        let is_paged = end_row > page_size;
        let mut position = 0;

        'outer: loop {
            let lstart = position*cols;
            let lend = min((position + page_size)*cols, lines.len());
            let mut i = 0;

            if is_paged {
                print!("\r");
            }

            for l in &lines[lstart..lend] {
                i = (i + 1)%cols;

                if i == 0 {
                    println!("{}", l);
                }
                else {
                    print!("{: <1$}", l, cwidth);
                }
            }

            if i != 0 {
                println!();
            }

            if !is_paged {
                break;
            }

            print!("--More--");
            io::stdout().flush().unwrap();

            loop {
                match self.wait_for_key() {
                    Key::Home => {
                        if position > 0 {
                            position = 0;
                            break;
                        }
                    },
                    Key::End => {
                        if position + page_size < end_row {
                            position = end_row - page_size;
                            break;
                        }
                    },
                    Key::PageUp | Key::Char('b') => {
                        if position > page_size {
                            position -= page_size;
                            break;
                        }
                        else if position > 0 {
                            position = 0;
                            break;
                        }
                    },
                    Key::Char(' ') | Key::PageDown => {
                        if position + page_size*2 < end_row {
                            position += page_size;
                            break;
                        }
                        else if position + page_size < end_row {
                            position = end_row - page_size;
                            break;
                        }
                    },
                    Key::ArrowUp => {
                        if position > 0 {
                            position -= 1;
                            break;
                        }
                    },
                    Key::Enter | Key::ArrowDown => {
                        if position + page_size < end_row {
                            position += 1;
                            break;
                        }
                    },
                    Key::Char('q') | Key::CtrlC | Key::Escape => {
                        break 'outer;
                    },
                    _ => { },
                }
            }
        }

        if is_paged {
            // Remove --More-- prompt
            print!("\r");
            print!("        ");
            print!("\r");
            io::stdout().flush().unwrap();
        }
    }
}

impl dyn CmdApp {
    pub fn parse_int(intstr: &str) -> Result<usize, String> {
        if let Ok(length) = intstr.parse() {
            return Ok(length);
        }
        else {
            return Err(format!("Expected integer, got '{}'", intstr));
        }
    }

    pub fn parse_bool(boolstr: &str) -> Result<bool, String> {
        return match boolstr {
            "on" | "true" | "1" => {
                Ok(true)
            },
            "off" | "false" | "0" => {
                Ok(false)
            },
            _ => {
                Err(format!("Expected boolean, got '{}'", boolstr))
            },
        }
    }

    pub fn opt_part(args: &Vec<String>, pos: usize) -> Option<&str> {
        if args.len() > pos {
            Some(&args[pos])
        }
        else {
            None
        }
    }

    pub fn expects_num_arguments(args: &Vec<String>, n: usize)
                             -> Result<(), String> {
        if args.len() < n {
            return Err(format!("Expected {} arguments", n));
        }
        else {
            return Ok(());
        }
    }
}

struct CommandLine {
    line: String
}

impl CommandLine {
    fn new(line: String) -> Self {
        Self {
            line: line
        }
    }

    fn as_str(&self) -> &str {
        return &self.line;
    }

    fn parts(&self) -> CommandLineIterator {
        return CommandLineIterator::new(self);
    }
}

pub struct CommandPart<'a> {
    slice: &'a str,
    is_quoted: bool,
    is_error: bool,
}

impl<'a> CommandPart<'a> {
    fn new(slice: &'a str) -> Self {
        let is_quoted = slice.find(' ').is_some();
        Self {
            slice: slice,
            is_quoted: is_quoted,
            is_error: false,
        }
    }

    fn error(slice: &'a str) -> Self {
        Self {
            slice: slice,
            is_quoted: false,
            is_error: true,
        }
    }

    pub fn as_str(&self) -> &str {
        return self.slice;
    }
    
    fn starts_with(&self, other: &CommandPart) -> bool {
        return self.slice.starts_with(other.slice);
    }

    fn to_string(&self) -> String {
        if self.is_error {
            return format!("Bad_command: {}", self.slice);
        }
        else if self.is_quoted {
            return format!("'{}'", self.slice);
        }
        else {
            return self.slice.to_string();
        }
    }
}

impl<'a> PartialEq for CommandPart<'a> {
    fn eq(&self, other: &CommandPart) -> bool {
        return self.slice == other.slice;
    }
}

pub struct CommandLineIterator<'a> {
    line: &'a CommandLine,
    position: usize,
}

impl<'a> CommandLineIterator<'a> {
    fn new(line: &'a CommandLine) -> Self {
        Self {
            line: line,
            position: 0,
        }
    }

    fn slice(&self, r: Range<usize>) -> &'a str {
        return &self.line.as_str()[r];
    }

    fn slice_from(&self, r: RangeFrom<usize>) -> &'a str {
        return &self.line.as_str()[r];
    }

    fn find_from(&self, r: RangeFrom<usize>, c: char) -> Option<usize> {
        let start = r.start;
        if let Some(nextpos) = &self.slice_from(r).find(c) {
            return Some(start + nextpos);
        }
        else {
            return None;
        }
    }

    fn find_from_to(&self, r: Range<usize>, c: char) -> Option<usize> {
        let start = r.start;
        if let Some(nextpos) = &self.slice(r).find(c) {
            return Some(start + nextpos);
        }
        else {
            return None;
        }
    }

    fn len(&self) -> usize {
        return self.line.as_str().len();
    }

    fn char_is(&self, pos: usize, c: char) -> bool {
        return self.line.as_str()[pos..].starts_with(c);
    }
}

impl<'a> Iterator for CommandLineIterator<'a> {
    type Item = CommandPart<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.position;

        if pos > self.len() {
            return None;
        }

        // If we are exactly at the end of line, a space was added after
        // the last part. Add an empty final part to signify this.
        if pos == self.len() {
            self.position += 1;
            return Some(CommandPart::new(""));
        }

        if self.char_is(pos, '\'') {
            // Quoted part
            if let Some(nextpos) = &self.find_from(pos + 1.., '\'') {
                // Located second quote
                if nextpos + 1 != self.len() {
                    // Not end of line
                    if self.char_is(nextpos + 1, ' ') {
                        // Space after quote
                        self.position = nextpos + 2;
                    }
                    else {
                        // No space found after quote. Treat as error
                        self.position = nextpos + 1;
                        return Some(CommandPart::error(
                            &self.slice(pos..*nextpos)));
                    }
                }
                else {
                    // End of line
                    self.position = nextpos + 2;
                }

                return Some(CommandPart::new(&self.slice(pos + 1..*nextpos)));
            }
            else {
                // No second quote found. Treat the rest of the string as part.
                self.position = self.len() + 1;
                return Some(CommandPart::new(&self.slice_from(pos + 1..)));
            }
        }
        else {
            // Unquoted part
            if let Some(nextpos) = &self.find_from(pos.., ' ') {
                // Not end of line
                // Check that part doesn't contain a quote
                self.position = nextpos + 1;
                if let Some(_) = self.find_from_to(pos..*nextpos, '\'') {
                    return Some(CommandPart::error(&self.slice(pos..*nextpos)));
                }
                else {
                    return Some(CommandPart::new(&self.slice(pos..*nextpos)));
                }
            }
            else {
                // End of line.
                self.position = self.len() + 1;
                // Check that part doesn't contain a quote
                if let Some(_) = self.find_from(pos.., '\'') {
                    return Some(CommandPart::error(&self.slice_from(pos..)));
                }
                else {
                    return Some(CommandPart::new(&self.slice_from(pos..)));
                }
            }
        }
    }
}

#[derive(Helper)]
struct CommandHelper<'a> {
    completer: CommandCompleter<'a>,
}

struct CommandCompleter<'a> {
    kw_exp: &'a dyn KeywordExpander,
}

impl<'a> CommandCompleter<'a> {
    fn new(kw_exp: &'a dyn KeywordExpander) -> Self {
        Self {
            kw_exp: kw_exp,
        }
    }

    fn complete(&self, line: &str, _pos: usize, _ctx: &Context)
        -> rustyline::Result<(usize, Vec<Pair>)>
    {
        let mut pairs = HashMap::new();

        let line_cl = CommandLine::new(line.to_string());
        let lwords: Vec<CommandPart> = line_cl.parts().collect();

        // Return empty completion list if line has errors
        for w in &lwords {
            if w.is_error {
                return Ok((0, vec!()));
            }
        }

        // Loop over all commands
        'commands: for cmd in self.kw_exp.command_list() {
            let mut prefix = "".to_string();

            let cmd_cl = CommandLine::new(cmd.to_string());
            let cmd_vec: Vec<CommandPart> = cmd_cl.parts().collect();
            let cmd_vec_len = cmd_vec.len();
            let mut parts = vec!();

            // Loop over command parts
            'parts: for (i, cp) in cmd_vec.into_iter().enumerate() {
                if i == lwords.len() {
                    continue 'commands;
                }

                let lpart = &lwords[i];

                parts.push(lpart.to_string());
                let keys = self.kw_exp.expand_keyword(&cp, &parts);

                let mut got_matches = false;

                for k in keys.iter().map(|k| CommandPart::new(&k)) {
                    if i == lwords.len() - 1 {
                        // Unfinished (last) part. Accept partial match.
                        if !k.starts_with(&lpart) {
                            continue;
                        }
                    }
                    else {
                        // Not last part. Require complete match for keywords
                        // No check for variable parameters
                        if !cp.slice.starts_with('<') {
                            if *lpart != k {
                                continue;
                            }
                        }

                        // The line matches the complete part. Add it to
                        // prefix, skip to next command part, continue
                        // matching.
                        prefix.push_str(&lpart.to_string());
                        prefix.push_str(" ");

                        continue 'parts;
                    }

                    // All line parts match the corresponding part in a
                    // command. Create a replacement pair.
                    let mut replacement = prefix.clone();
                    replacement.push_str(&k.to_string());

                    if cmd_vec_len > i + 1 {
                        replacement.push(' ');
                    }

                    let display = k.to_string();

                    pairs.insert(display.clone(), Pair {
                        display: display,
                        replacement: replacement,
                    });
                    got_matches = true;
                }

                if !got_matches {
                    continue 'commands;
                }
            }
        }

        let mut pairvec: Vec<Pair> = pairs.into_values().collect();
        pairvec.sort_by(|a, b| a.display.cmp(&b.display));

        Ok((0, pairvec))
    }
}

impl<'a> Completer for CommandHelper<'a> {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, ctx: &Context)
                -> rustyline::Result<(usize, Vec<Pair>)>
    {
        self.completer.complete(line, pos, ctx)
    }
}

impl<'a> Hinter for CommandHelper<'a> {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context)
            -> Option<String>
    {
        None
    }
}

impl<'a> Validator for CommandHelper<'a> {
    fn validate(&self, _ctx: &mut ValidationContext)
                -> rustyline::Result<ValidationResult>
    {
        Ok(ValidationResult::Valid(None))
    }
}

impl<'a> Highlighter for CommandHelper<'a> {}

pub struct CmdUI<'a> {
    app: &'a mut dyn CmdApp,
    opt_kw_exp: Option<&'a dyn KeywordExpander>,
}

impl<'a> CmdUI<'a> {
    pub fn new(
        app: &'a mut dyn CmdApp,
        opt_kw_exp: Option<&'a dyn KeywordExpander>,
    ) -> Self
    {
        Self {
            app: app,
            opt_kw_exp: opt_kw_exp,
        }
    }

    pub fn read_commands(&mut self) {
        self.app.startup();

        let config = Config::builder()
            .completion_type(CompletionType::List)
            .build();

        let mut editor = Editor::with_config(config).unwrap();

        loop {
            if let Some(kw_exp) = self.opt_kw_exp {
                let helper = CommandHelper {
                    completer: CommandCompleter::new(kw_exp),
                };
                editor.set_helper(Some(helper));
            }

            let mut args: Vec<String>;
            let readline = editor.readline("> ");

            match readline {
                Ok(line) => {
                    let _ = editor.add_history_entry(&line);
                    args = CommandLine::new(line)
                        .parts()
                        .map(|p| p.to_string())
                        .collect();
                },
                Err(ReadlineError::Interrupted) => {
                    continue;
                },
                Err(ReadlineError::Eof) => {
                    break;
                },
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                },
            }

            // Move the left hand static command keywords out of the args
            // list, and concatenate them into a command string.
            let mut cmd = "".to_string();
            let mut cmdlist: Vec<&str> = self.app.command_list().to_vec();

            loop {
                if args.len() == 0 {
                    break;
                }

                if args[0].starts_with('<') && args[0].ends_with('>') {
                    // Next param is a '<keyword>' replacement word, literate.
                    // Don't include it into the command.
                    break;
                }

                // Skip empty args
                if args[0].is_empty() {
                    args.remove(0);
                    continue;
                }

                let p = if cmd.len() == 0 {
                    args[0].clone()
                }
                else {
                    format!("{} {}", cmd, args[0])
                };

                cmdlist = cmdlist.into_iter()
                    .filter(|c| c.starts_with(&p))
                    .collect();

                if cmdlist.len() > 0 {
                    cmd = p;
                    args.remove(0);
                }
                else {
                    break;
                }
            }

            if cmd == "" {
                if args.len() > 0 {
                    println!("Bad command.");
                }
                continue;
            }

            if let Err(e) = self.app.execute_line(&cmd, &args) {
                println!("{}", e);
            }
        }

        self.app.exit();
    }
}
