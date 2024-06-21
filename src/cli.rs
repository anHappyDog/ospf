use clap::{Arg, ArgMatches, Command};
use ospf_lib::router;
use rustyline::{
    history::{History, MemHistory},
    Completer, CompletionType, Config, Editor, Helper, Highlighter, Hinter, Validator,
};
use std::net;
use std::sync::{Arc, Mutex};
#[derive(Helper, Hinter, Validator, Highlighter, Completer)]
struct OspfHelper;

// impl Helper for OspfHelper {

// }

// impl Hinter for OspfHelper {
// }

// impl Validator for OspfHelper {}

// impl Highlighter for OspfHelper {}

// impl Completer for OspfHelper {
//     type Candidate = String;
// }

lazy_static::lazy_static! {

    static ref INTERFACE_UP_COMMAND : Command =  Command::new("up")
    .about("Interface up")
    .arg(Arg::new("interface").help("Interface name").required(true));
    static ref INTERFACE_DOWN_COMMAND : Command = Command::new("down")
    .about("Interface down")
    .arg(Arg::new("interface").help("Interface name").required(true));
    static ref INTERFACE_LIST_COMMAND : Command = Command::new("list")
    .about("List all interfaces");
    static ref INTERFACE_COMMAND : Command =  Command::new("interface")
    .about("Interface commands")
    .subcommand(INTERFACE_UP_COMMAND.clone())
    .subcommand(INTERFACE_DOWN_COMMAND.clone())
    .subcommand(INTERFACE_LIST_COMMAND.clone());
    static ref EXIT_COMMAND : Command = Command::new("exit")
    .about("Exit the ospf cli");
    static ref OSPF_COMMAND : Command =  Command::new("ospf")
    .version("1.0")
    .author("doggie")
    .about("OSPF CLI")
    .subcommand(INTERFACE_COMMAND.clone())
    .subcommand(EXIT_COMMAND.clone());

}

fn match_ospf_command(line: &str) {
    match OSPF_COMMAND
        .clone()
        .try_get_matches_from(line.split_whitespace())
    {
        Ok(matches) => {
            if let Some(sub_command_matches) = matches.subcommand_matches("interface") {
                match_interface_subcommand(sub_command_matches);
            }
            else if let Some(_) = matches.subcommand_matches("exit") {
                println!("Bye");
                std::process::exit(0);
            } else {
                OSPF_COMMAND
                    .clone()
                    .print_help()
                    .expect("print ospf command help failed");
            }
        }
        Err(err) => {
            err.print().expect("print err error");
        }
    }
}

fn match_interface_subcommand(args_match: &ArgMatches) {
    if let Some(sub_command_matches) = args_match.subcommand_matches("up") {
        let interface = sub_command_matches.get_one::<String>("interface").unwrap();
        println!("Interface up: {}", interface);
    }
    else if let Some(sub_command_matches) = args_match.subcommand_matches("down") {
        let interface = sub_command_matches.get_one::<String>("interface").unwrap();
        println!("Interface down: {}", interface);
    }
    else if let Some(_) = args_match.subcommand_matches("list") {
        println!("List all interfaces");
    } else {
        INTERFACE_COMMAND
            .clone()
            .print_help()
            .expect("print interface command help failed");
    }
}

pub(super) fn cli(router_id: net::Ipv4Addr) -> Result<(), Box<dyn std::error::Error>> {
    let cmdline_config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .build();
    let cmdline_helper = OspfHelper;
    let mut cmdline_editor = Editor::<OspfHelper, _>::with_config(cmdline_config)?;
    cmdline_editor.set_helper(Some(cmdline_helper));
    loop {
        let readline = cmdline_editor.readline(&format!("{}>>", router_id));
        if let Ok(line) = readline {
            cmdline_editor.add_history_entry(line.as_str())?;
            match_ospf_command(&line);
        } else {
            println!("Bye");
            break;
        }
    }

    Ok(())
}
