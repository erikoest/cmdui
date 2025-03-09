extern crate cmdui;

use cmdui::{CmdUI, CmdApp, CommandPart, KeywordExpander};

const COMMAND_LIST: &'static [&'static str] = &[
    "set attr1 <bool>",
    "set attr2 <int>",
    "read <filename>",
    "store <filename>",
    "add <key> <word>",
    "run",
    "help",
];

struct DemoKeywordExpander {
}

impl DemoKeywordExpander {
    fn new() -> Self {
        Self {}
    }

    fn expand_keys(&self, _: &str) -> Vec<String> {
        return vec!["akey".to_string(), "bkey".to_string(),
                    "ckey".to_string()];
    }

    fn expand_words(&self, _: &str) -> Vec<String> {
        return vec!["apple".to_string(), "orange".to_string(),
                    "banana".to_string()];
    }
}

impl KeywordExpander for DemoKeywordExpander {
    fn command_list<'a>(&self) -> &'a [&'a str] {
        return COMMAND_LIST;
    }

    fn expand_keyword(&self, cp: &CommandPart, parts: &Vec<String>)
                      -> Vec<String> {
        let lpart = &parts[parts.len() - 1];

        match cp.as_str() {
            "<filename>"  => { self.expand_filename(lpart) },
            "<key>"       => { self.expand_keys(lpart) },
            "<word>"      => { self.expand_words(lpart) },
            "<bool>"      => { vec!["false".to_string(), "true".to_string()] },
            s             => { vec![s.to_string()] },
        }
    }
}

struct DemoApp {
}

impl DemoApp {
    fn new() -> Self {
        Self {}
    }

    fn set_bool_param(&mut self, key: &str, val: bool) {
        println!("Setting parameter {} to {}", key, val);
    }

    fn set_int_param(&mut self, key: &str, val: usize) {
        println!("Setting parameter {} to {}", key, val);
    }

    fn read(&mut self, _: Option<&str>) {
        println!("Reading something");
    }

    fn store(&mut self, _: Option<&str>) {
        println!("Storinging something");
    }

    fn add_keyword(&mut self, _: &str, _: &str) {
        println!("Adding keyword");
    }

    fn run(&mut self) {
        println!("Running something");
    }
    
    fn help(&self) {
        println!("{}", COMMAND_LIST.into_iter()
                 .map(|c| c.replace("<bool>", "on/off"))
                 .collect::<Vec<String>>()
                 .join("\n")
        );
    }
}

impl CmdApp for DemoApp {
    fn command_list<'a>(&self) -> &'a [&'a str] {
        return COMMAND_LIST;
    }

    fn execute_line(&mut self, cmd: &str, args: &Vec<String>)
                    -> Result<(), String> {
        match cmd {
            "set attr1" => {
                <dyn CmdApp>::expects_num_arguments(args, 1)?;
                self.set_bool_param("attr1", <dyn CmdApp>::parse_bool(&args[0])?);
            },
            "set attr2" => {
                <dyn CmdApp>::expects_num_arguments(args, 1)?;
                self.set_int_param("attr2", <dyn CmdApp>::parse_int(&args[0])?);
            },
            "read" => {
                self.read(<dyn CmdApp>::opt_part(args, 0));
            },
            "store" => {
                self.store(<dyn CmdApp>::opt_part(args, 0));
            },
            "add" => {
                <dyn CmdApp>::expects_num_arguments(args, 2)?;
                self.add_keyword(&args[0], &args[1]);
            },
            "run" => {
                self.run();
            },
            "help" => {
                self.help();
            },
            "" => { },
            _ => {
                return Err("Bad command".to_string());
            },
        }

        Ok(())
    }

    fn startup(&mut self) {
        println!("Starting up...");
    }

    fn exit(&mut self) {
        println!("Quitting...");
    }
}

fn main() {
    let mut app = DemoApp::new();
    let kw_exp = DemoKeywordExpander::new();

    CmdUI::new(&mut app, Some(&kw_exp)).read_commands();
}
