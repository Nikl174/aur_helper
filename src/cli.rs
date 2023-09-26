use std::env;

use clap::Arg;

pub struct Cli {
    default_dir: String,
}
impl Cli {
    pub fn new() -> Self {
        let mut home_dir = match env::var_os("HOME") {
            Some(path) => path,
            None => {
                println!("WARNING: couldn't find HOME environmental variable, defaulting to current dir!");
                match env::current_dir() {
                    Ok(path) => path.into_os_string(),
                    Err(err) => {
                        println!(
                            "Couldn't get the current path because: \n {}\n Exiting!",
                            err
                        );
                        panic!();
                    }
                }
            }
        };
        home_dir.push("/AUR");
        Self {
            default_dir: home_dir
                .to_str()
                .expect("can't convert str of default dir to string")
                .to_string(),
        }
    }
    // build the CLI with clap, -> pacman as inspiration
    // may panic, when no AUR_PATH is given
    pub fn get_cli_command(self) -> clap::Command {
        // arguments
        let remove_arg = Arg::new("remove")
            .short('r')
            .action(clap::ArgAction::SetTrue)
            .help("removes the not installed directorys from the aur dir");

        let build_arg = Arg::new("build")
            .short('b')
            .action(clap::ArgAction::SetTrue)
            .help("builds the updated packages");

        let install_arg = Arg::new("install")
            .short('i')
            .action(clap::ArgAction::SetTrue)
            .help("generates the pacman command and installs the build packages, CALLS SUDO!");

        let aur_path_arg = Arg::new("AUR_PATH")
            .value_name("AUR_PATH")
            .default_value(self.default_dir)
            .value_parser(clap::builder::PathBufValueParser::new())
            .value_hint(clap::ValueHint::DirPath)
            .help("The path to the aur-directories");

        let search_arg = Arg::new("search")
            .short('s')
            .action(clap::ArgAction::SetTrue)
            .help("extended search for package name and description");
        let search_name_arg = Arg::new("search_name")
            .required(true)
            .value_hint(clap::ValueHint::Other)
            .action(clap::ArgAction::Set)
            .value_parser(clap::builder::StringValueParser::new())
            .help("a package name to search for")
            .num_args(1);

        // end arguments
        //
        //
        // subcommands
        let check = clap::Command::new("check")
            .short_flag('C')
            .about("checks, which packages are actually installed")
            .arg(remove_arg.clone());
        let install = clap::Command::new("install").short_flag('I').about(
            "generates the pacman command and installs the LAST BUILD packages, CALLS SUDO!",
        );
        let update = clap::Command::new("update")
            .short_flag('U')
            .about("updates the git repos in the directory")
            .arg(build_arg.clone())
            .arg(install_arg.clone());
        let build = clap::Command::new("build")
            .short_flag('B')
            .about("builds the packages recursively")
            .arg(install_arg.clone());
        // TODO: optional: download after search and select afterward
        let search = clap::Command::new("search")
            .short_flag('S')
            .about("searches for packages by a given name and shows informations about the package")
            .arg(search_name_arg)
            .arg(search_arg);
        // end subcommands

        clap::Command::new("aur_helper")
            .about("a simple aur package helper for updating, building and installing AUR packages in a directory")
            .arg_required_else_help(true)
            .arg(aur_path_arg)
            .subcommand_required(true)
            .subcommand(update)
            .subcommand(build)
            .subcommand(install)
            .subcommand(check)
            .subcommand(search)
    }
}
