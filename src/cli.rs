use clap::{Arg, ArgMatches, Command};
use rustyline::{
    Completer, CompletionType, Config, Editor, Helper, Highlighter, Hinter, Validator,
};

use crate::{interface, ROUTER_ID};

lazy_static::lazy_static! {

    static ref INTERFACE_UP_COMMAND : Command =  Command::new("up")
    .about("Interface up")
    .arg(Arg::new("interface").help("Interface name").required(true));
    static ref INTERFACE_DOWN_COMMAND : Command = Command::new("down")
    .about("Interface down")
    .arg(Arg::new("interface").help("Interface name").required(true));
    static ref INTERFACE_LIST_COMMAND : Command = Command::new("list")
    .about("List all interfaces");
    static ref INTERFACE_DISPLAY_COMMAND : Command = Command::new("display")
    .about("Display interface")
    .arg(Arg::new("interface").help("Interface name").required(true));
    static ref AREA_CREATE_COMMAND : Command = Command::new("create")
    .about("Create area").arg(Arg::new("area").help("Area id").required(true));
    static ref AREA_LIST_COMMAND : Command = Command::new("list")
    .about("List all areas");
    static ref AREA_DISPLAY_COMMAND : Command = Command::new("display")
    .about("Display area").arg(Arg::new("area").help("Area id").required(true));
    static ref AREA_COMMAND : Command = Command::new("area")
    .about("Area commands")
    .subcommand(AREA_CREATE_COMMAND.clone())
    .subcommand(AREA_LIST_COMMAND.clone())
    .subcommand(AREA_DISPLAY_COMMAND.clone());
    static ref INTERFACE_COMMAND : Command =  Command::new("interface")
    .about("Interface commands")
    .subcommand(INTERFACE_UP_COMMAND.clone())
    .subcommand(INTERFACE_DOWN_COMMAND.clone())
    .subcommand(INTERFACE_LIST_COMMAND.clone())
    .subcommand(INTERFACE_DISPLAY_COMMAND.clone());
    static ref EXIT_COMMAND : Command = Command::new("exit")
    .about("Exit the ospf cli");
    static ref OSPF_COMMAND : Command =  Command::new("ospf")
    .version("1.0")
    .author("doggie")
    .about("OSPF CLI")
    .subcommand(INTERFACE_COMMAND.clone())
    .subcommand(EXIT_COMMAND.clone())
    .subcommand(AREA_COMMAND.clone());

}

#[derive(Helper, Hinter, Validator, Highlighter, Completer)]
struct OspfHelper;

async fn match_ospf_command(line: &str) {
    match OSPF_COMMAND
        .clone()
        .try_get_matches_from(line.split_whitespace())
    {
        Ok(matches) => {
            if let Some(sub_command_matches) = matches.subcommand_matches("interface") {
                match_interface_subcommand(sub_command_matches).await;
            } else if let Some(_) = matches.subcommand_matches("exit") {
                println!("Bye");
                std::process::exit(0);
            } else if let Some(sub_command_matches) = matches.subcommand_matches("area") {
                match_area_command(sub_command_matches).await;
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

async fn match_area_command(args_match: &ArgMatches) {
    if let Some(sub_command_matches) = args_match.sub_command_matches("create") {
        let area_id = sub_command_matches.get_one::<String>("area").unwrap();
        area::create(area_id.parse().unwrap()).await;
    } else if let Some(_) = args_match.subcommand_matches("list") {
        area::list().await;
    } else if let Some(sub_command_matches) = args_match.subcommand_matches("display") {
        let area_id = sub_command_matches.get_one::<String>("area").unwrap();
        area::Area::display(area_id.parse().unwrap()).await;
    } else {
        AREA_COMMAND
            .clone()
            .print_help()
            .expect("print area command help failed");
    }
}

async fn match_interface_subcommand(args_match: &ArgMatches) {
    if let Some(sub_command_matches) = args_match.subcommand_matches("up") {
        let interface_name = sub_command_matches.get_one::<String>("interface").unwrap();

        interface::event::send_by_name(
            interface_name.clone(),
            interface::event::Event::InterfaceUp(interface_name.clone()),
        )
        .await;
    } else if let Some(sub_command_matches) = args_match.subcommand_matches("down") {
        let interface_name = sub_command_matches.get_one::<String>("interface").unwrap();
        interface::event::send_by_name(
            interface_name.clone(),
            interface::event::Event::InterfaceDown(interface_name.clone()),
        )
        .await;
    } else if let Some(_) = args_match.subcommand_matches("list") {
        interface::list().await;
    } else if let Some(sub_command_matches) = args_match.subcommand_matches("display") {
        let interface_name = sub_command_matches.get_one::<String>("interface").unwrap();
        interface::Interface::display(interface_name.clone()).await;
    } else {
        INTERFACE_COMMAND
            .clone()
            .print_help()
            .expect("print interface command help failed");
    }
}

pub async fn cli() -> Result<(), Box<dyn std::error::Error>> {
    let cmdline_config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .build();
    let cmdline_helper = OspfHelper;
    let mut cmdline_editor = Editor::<OspfHelper, _>::with_config(cmdline_config)?;
    cmdline_editor.set_helper(Some(cmdline_helper));
    loop {
        let readline = cmdline_editor.readline(&format!("{}>>", ROUTER_ID.clone()));
        if let Ok(line) = readline {
            cmdline_editor.add_history_entry(line.as_str())?;
            match_ospf_command(&line).await;
        } else {
            println!("Bye");
            std::process::exit(0);
        }
    }
}
