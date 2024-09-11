use clap::{Arg, ArgMatches};
use dir_func::*;

use std::{collections::HashSet, env, io, path::PathBuf};

pub struct Config {
    aur_dir: String,
}

fn get_fallback_aur_dir() -> String {
    let mut home_dir = match env::var_os("HOME") {
        Some(path) => path,
        None => {
            println!(
                "WARNING: couldn't find HOME environmental variable, defaulting to current dir!"
            );
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
    home_dir
        .to_str()
        .expect("can't convert str of default dir to string")
        .to_string()
}

#[derive(Debug, Clone)]
pub struct Cli {
    default_dir: String,
}
impl Cli {
    pub fn new(config_file: Option<Config>) -> Self {
        let default_dir = match config_file {
            Some(config) => config.aur_dir,
            None => get_fallback_aur_dir(),
        };
        Self { default_dir }
    }
    pub fn get_aur_dir(&self) -> String {
        return self.default_dir.clone();
    }
    // build the CLI with clap, -> pacman as inspiration
    pub fn get_cli_command(self) -> clap::Command {
        // arguments
        let aur_packet_arg = Arg::new("AUR_PACKAGES")
            .value_name("AUR_PACKAGES")
            .help("package names in the AUR_DIR the current command should be applied to")
            .num_args(0..)
            .action(clap::ArgAction::Set);
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
            .long("install")
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
        let download_links_arg = Arg::new("download_links")
            .required(true)
            .num_args(0..)
            //.value_hint(clap::ValueHint::Other)
            .action(clap::ArgAction::Set)
            //.value_parser(clap::builder::StringValueParser::new())
            .help("a link or list of links for repos to clone; uses the AUR, if only a package name is provided");

        // end arguments
        //
        //
        // subcommands
        let download = clap::Command::new("download")
            .short_flag('D')
            .long_flag("download")
            .about("Clones the Repos to the AUR dir")
            .arg(build_arg.clone())
            .arg(install_arg.clone())
            .arg(download_links_arg);
        let check = clap::Command::new("check")
            .short_flag('C')
            .long_flag("check")
            .about("checks, which packages are actually installed")
            .arg(remove_arg.clone())
            .arg(aur_packet_arg.clone());
        let install = clap::Command::new("install")
            .short_flag('I')
            .long_flag("install")
            .about("generates the pacman command and installs the LAST BUILD packages, CALLS SUDO!")
            .arg(aur_packet_arg.clone());
        let update = clap::Command::new("update")
            .short_flag('U')
            .long_flag("update")
            .about("updates the git repos in the directory")
            .arg(build_arg.clone())
            .arg(install_arg.clone())
            .arg(aur_packet_arg.clone());
        let build = clap::Command::new("build")
            .short_flag('B')
            .long_flag("build")
            .about("builds the packages recursively")
            .arg(install_arg.clone())
            .arg(aur_packet_arg.clone());
        // TODO: optional: download after search and select afterward
        let search = clap::Command::new("search")
            .short_flag('S')
            .long_flag("search")
            .about("searches for packages by a given name and shows informations about the package")
            .arg(search_name_arg)
            .arg(search_arg);
        let get_aur_dir = clap::Command::new("get-aur-dir").hide(true);
        // end subcommands

        clap::Command::new("aur_helper")
            .about("a simple aur package helper for updating, building and installing AUR packages in a directory")
            // .arg_required_else_help(true)
            .arg(aur_path_arg)
            .subcommand_required(true)
            .subcommand(update)
            .subcommand(build)
            .subcommand(install)
            .subcommand(check)
            .subcommand(search)
            .subcommand(get_aur_dir)
            .subcommand(download)
    }
}

fn string_to_link(package: &String) -> String {
    let mut aur_link = "https://aur.archlinux.org/".to_owned();
    if package.find("/").is_none() {
        aur_link.push_str(package);
        aur_link.push_str(".git");
        aur_link.to_owned()
    } else {
        package.to_owned()
    }
}

pub fn download_command(aur_path: &PathBuf, sub_matches: ArgMatches) {
    let links: Vec<String> = sub_matches
        .get_many::<String>("download_links")
        .expect("search_name argument required but couldn't get it")
        .map(|item| string_to_link(item))
        .collect();

    let download_dirs = download_packages_from_git(aur_path, links);
    let build = sub_matches.get_flag("build");

    let (download_dirs, err) = match download_dirs {
        Ok(cloned_dirs) => {
            println!("Downloaded packages: \n {:?}", cloned_dirs);
            (cloned_dirs, false)
        }
        Err((cloned_dirs, failed_links)) => {
            println!("ERROR downloading packages!\n Failed downloads: \n {:?} \n Successfull downloads: \n {:?}", failed_links, cloned_dirs);
            (cloned_dirs, true)
        }
    };

    if build {
        if err {
            if confirm_ask(Some("Remove downloaded Packages?".to_owned())).is_ok() {
                remove_command(download_dirs.clone());
            } else {
                return;
            }
        }
        if confirm_ask(None).is_err() {
            return;
        }
        build_command(download_dirs, sub_matches);
    }
}

pub fn update_command(dirs: Vec<PathBuf>, sub_matches: ArgMatches) {
    let updated_dirs = update_packages(dirs.clone());
    let build = sub_matches.get_flag("build");

    let (updated_dirs, err) = match updated_dirs {
        Ok(paths) => (paths, false),
        Err(err_paths) => {
            println!("ERROR in paths: \n {:?}\n", err_paths.clone());
            (
                get_set_diff(dirs, err_paths.into_iter().map(|x| x.0).collect()),
                true,
            )
        }
    };
    println!("Updated packages: \n {:?}", updated_dirs);

    if build {
        if err {
            if confirm_ask(None).is_err() {
                return;
            }
        }
        build_command(updated_dirs, sub_matches);
    }
}

pub fn build_command(dirs: Vec<PathBuf>, sub_matches: ArgMatches) {
    let build_pkgs = build_packages(dirs.clone());
    let install = sub_matches.get_flag("install");

    let (build_pkgs, err) = match build_pkgs {
        Ok(paths) => (paths, false),
        Err(err_paths) => {
            println!("ERROR building some packages: \n {:?}", err_paths);
            (
                get_set_diff(dirs, err_paths.into_iter().map(|x| x.0).collect()),
                true,
            )
        }
    };
    if install {
        if err {
            if confirm_ask(None).is_err() {
                return;
            }
        }
        install_command(build_pkgs);
    }
}

pub fn install_command(dirs: Vec<PathBuf>) {
    let install_cmd = install_packages(dirs);
    let mut install_cmd = match install_cmd {
        Ok(cmd) => cmd,
        Err((cmd, err_paths)) => {
            println!(
                "Error on some Packages (not found or a read error): \n {:?}",
                err_paths
            );
            match confirm_ask(None) {
                Ok(_) => cmd,
                Err(_) => return,
            }
        }
    };
    println!("Calling the following command: \n {:?}", install_cmd);
    match confirm_ask(None) {
        Ok(_) => {
            install_cmd.status().expect("Error calling pacman");
        }
        Err(_) => return,
    }
}

pub fn check_command(dirs: Vec<PathBuf>, sub_matches: ArgMatches) {
    let inst_pkgs = check_installed(dirs.clone());
    let remove = sub_matches.get_flag("remove");

    let mut dirs_set: HashSet<PathBuf> = dirs.clone().into_iter().collect();
    println!("\nPackages installed: \n");
    for dir in inst_pkgs.expect("Io error occured on checking files") {
        dirs_set.remove(&dir);
        println!(
            "{}",
            dir.file_name()
                .expect("couldn't get filename!")
                .to_str()
                .unwrap()
        )
    }
    println!("\nPackages in directory and not installed: \n");
    for dir in dirs_set.clone() {
        println!(
            "{}",
            dir.file_name()
                .expect("couldn't get filename!")
                .to_str()
                .unwrap()
        )
    }
    if remove {
        let dirs = dirs_set.into_iter().collect();
        remove_command(dirs);
    }
}

pub fn remove_command(dirs: Vec<PathBuf>) {
    let cmd = remove_uninstalled_dirs(dirs);
    match cmd {
        Some(mut c) => {
            print!("Calling: \n{}", c.get_program().to_str().unwrap());
            for arg in c.get_args() {
                print!(" {}", arg.to_str().unwrap());
            }
            println!("");
            match confirm_ask(None) {
                Ok(_) => {
                    c.status().expect("error on removing");
                }
                Err(_) => return,
            }
        }
        None => {
            println!("No unused directory, everything is installed")
        }
    }
}
pub async fn search_command(sub_matches: ArgMatches) {
    let ext_search = sub_matches.get_flag("search");
    let search_name: &String = sub_matches
        .get_one::<String>("search_name")
        .expect("search_name argument required but couldn't get it");
    if ext_search {
        ext_search_aur(search_name).await;
    } else {
        search_aur(search_name).await;
    }
}

// ask for confirmation on stdout
fn confirm_ask(msg: Option<String>) -> Result<(), ()> {
    println!("{} [Y|n]", msg.unwrap_or("Continue?".to_owned()));
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => {
            if input == "\n" || input == "Y" {
                return Ok(());
            } else {
                println!("Aborting");
                return Err(());
            }
        }
        Err(err) => {
            println!("IO-error: {:?}", err);
            return Err(());
        }
    }
}
