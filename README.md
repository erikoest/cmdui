# cmdui

Simple framework for building terminal command based user interface 
applications. The  interface uses rustyline for line editing, command
history and tab completion.

## Usage

Build the application into a struct which implements the CliApp trait. 
A Cli object is then created which operates on the application struct. The
callback method `execute_line` is called whenever a command line has been
entered and is to be executed by the application.

If tab completion is wanted, a keyword expander struct must be defined (it
must implement the KeywordExpander trait). It is them sent into the CliApp on
construction.

Se the included `demoapp` application for a complete example.

<pre>
    use cmdui::{CmdUI, CmdApp, KeywordExpander};

    struct DemoApp { }

    impl CmdApp for DemoApp {
       ...
    }

    struct DemoKeywordExpander { }

    impl KeywordExpander for DemoKeywordExpander {
       ...
    }

    let mut app = DemoApp::new();
    let kw_exp = DemoKeywordExpander::new();
    
    CmdUI::new(&mut app, Some(&kw_exp)).read_commands();
</pre>

### Helper functions

The CmdApp base struct contains some helper functions which are often needed
when building the command line app:

#### confirm_yes_no(&self) -> bool

Waits for the user to type a key (expecting 'y' or 'n', but tolerating
anything). Return true if 'y' type, otherwise false.

#### print_columns(&self, lines: [&string], max_line: uszie)

Prints words in multiple columns, with a pager functionality if the total
number exceeds the capacity of a page.

#### parse_int(intstr: &str) -> Result<usize, String>

Parses a string into an usize. Returns Ok(usize) or Err.

#### parse_bool(boolstr: &str) -> Result<bool, String>

Parses a string into a boolean. t/f and 0/1 are accepted as boolean values.
Returns a bool or error.

#### opt_part(args: &Vec<String>, pos: usize) -> Option<&str>

Takes a vector of arguments and a position indicator. Returns Some(&str)
if the argument at position n exists, otherwise None.

#### expects_num_arguments(args: &Vec<String>, n: usize) -> Result<(), String>

Takes a vector of arguments and a size indicator. Returns Ok if the argument
list is big enough, otherwise an Err.
