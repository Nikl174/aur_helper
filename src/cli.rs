use std::{collections::HashSet, env, ffi::OsString, io, path::PathBuf};

use crate::dir_func;
use clap::Arg;
use raur::Raur;

pub struct Cli {
    default_dir: OsString,
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
        home_dir.push("AUR");
        Self {
            default_dir: home_dir,
        }
    }
    // build the CLI with clap, -> pacman as inspiration
    // may panic, when no AUR_PATH is given
    pub fn get_matches_cli(self) -> clap::ArgMatches {
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

        // TODO: needs improvement for default value
        let aur_path_arg = Arg::new("AUR_PATH")
            .default_value(&self.default_dir)
            .value_name("AUR_PATH")
            .value_parser(clap::builder::PathBufValueParser::new())
            .value_hint(clap::ValueHint::DirPath)
            .help("The path to the aur-directories");

        let search_arg = Arg::new("search")
            .short('s')
            .action(clap::ArgAction::SetTrue)
            .help("extended search for package name and description");

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
            .arg(
                Arg::new("search_name")
                    .required(true)
                    .value_hint(clap::ValueHint::Other)
                    .action(clap::ArgAction::Set)
                    .value_parser(clap::builder::StringValueParser::new())
                    .help("a package name to search for")
                    .num_args(1),
            )
            .arg(search_arg);
        // end subcommands

        clap::Command::new("aur_helper")
            .about("a simple aur package helper for updating, building and installing AUR packages in a directory")
            .arg(aur_path_arg)
            .arg_required_else_help(true)
            .subcommand_required(true)
            .subcommand(update)
            .subcommand(build)
            .subcommand(install)
            .subcommand(check)
            .subcommand(search)
            .get_matches()
    }
}
// .subcommand(
//     clap::Command::new("remove")
//         .short_flag('R')
//         .about("removed not installed directory from the aur-directory"),
// )

// ask for confirmation on stdout
fn confirm_ask() -> Result<(), ()> {
    println!("Continue? [Y|n]");
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

fn get_set_diff(bigger_dir: Vec<PathBuf>, containing_dir: Vec<PathBuf>) -> Vec<PathBuf> {
    let bigger_set: HashSet<PathBuf> = bigger_dir.into_iter().collect();
    let containing_set: HashSet<PathBuf> = containing_dir.into_iter().collect();

    let diff: HashSet<PathBuf> = bigger_set
        .difference(&containing_set)
        .into_iter()
        .map(|x| x.clone())
        .collect();
    return diff.into_iter().collect();
}

pub fn update_cli(dirs: Vec<PathBuf>, build: bool, install: bool) {
    let updated_dirs = dir_func::update_packages(dirs.clone());

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
            if confirm_ask().is_err() {
                return;
            }
        }
        build_cli(updated_dirs, install);
    }
}

pub fn build_cli(dirs: Vec<PathBuf>, install: bool) {
    let build_pkgs = dir_func::build_packages(dirs.clone());
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
            if confirm_ask().is_err() {
                return;
            }
        }
        install_cli(build_pkgs);
    }
}

pub fn install_cli(dirs: Vec<PathBuf>) {
    let install_cmd = dir_func::install_packages(dirs);
    let mut install_cmd = match install_cmd {
        Ok(cmd) => cmd,
        Err((cmd, err_paths)) => {
            println!(
                "Error on some Packages (not found or a read error): \n {:?}",
                err_paths
            );
            match confirm_ask() {
                Ok(_) => cmd,
                Err(_) => return,
            }
        }
    };
    println!("Calling the following command: \n {:?}", install_cmd);
    match confirm_ask() {
        Ok(_) => {
            install_cmd.status().expect("Error calling pacman");
        }
        Err(_) => return,
    }
}

pub fn check_cli(dirs: Vec<PathBuf>, remove: bool) {
    let inst_pkgs = dir_func::check_installed(dirs.clone());
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
        remove_cli(dirs);
    }
}

pub fn remove_cli(dirs: Vec<PathBuf>) {
    let cmd = dir_func::remove_uninstalled_dirs(dirs);
    match cmd {
        Some(mut c) => {
            print!("Calling: \n{}", c.get_program().to_str().unwrap());
            for arg in c.get_args() {
                print!(" {}", arg.to_str().unwrap());
            }
            match confirm_ask() {
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
pub async fn search_cli(search_name: String, ext_search: bool) {
    let raur_handler = raur::Handle::new();
    if ext_search {
        match raur_handler.search(search_name.clone()).await {
            Ok(pkg_vec) => {
                for pkg in pkg_vec {
                    dir_func::print_simple_pkg_info(pkg);
                }
            }
            Err(err) => {
                println!("Error while searching for {}: \n {}", search_name, err);
            }
        }
    } else {
        match raur_handler.info(&[search_name.clone()]).await {
            Ok(pkg_vec) => {
                println!("Pkg_vec len {}", pkg_vec.len());
                let pkg = match pkg_vec.first() {
                    Some(p) => p,
                    None => {
                        println!(
                            "Couldn't find a package named '{}', try -Ss.",
                            search_name.clone()
                        );
                        return;
                    }
                };
                let pkg = pkg.clone();
                dir_func::print_detailed_pkg_info(pkg);
            }
            Err(err) => {
                println!("Error while searching for {}: \n {}", search_name, err);
            }
        }
    }
}
