use std::{collections::HashSet, io, path::PathBuf};

use crate::dir_func;
use clap::Arg;
use raur::Raur;

// build the CLI with clap, -> pacman as inspiration
pub fn create_cli() -> clap::Command {
    // arguments
    let remove_arg = Arg::new("remove")
        .short('r')
        .action(clap::ArgAction::SetTrue)
        .help("removes the not installed directorys from the aur");

    let build_arg = Arg::new("build")
        .short('b')
        .action(clap::ArgAction::SetTrue)
        .help("builds the updated packages");

    let install_arg = Arg::new("install")
        .short('i')
        .action(clap::ArgAction::SetTrue)
        .help("generates the pacman command and installs the build packages, CALLS SUDO!");

    let aur_path_arg = Arg::new("AUR_PATH")
        .required(true)
        .value_name("AUR_PATH")
        .value_parser(clap::builder::PathBufValueParser::new())
        .value_hint(clap::ValueHint::FilePath)
        .help("The path to the aur-directories");

    // end arguments
    //
    //
    // subcommands
    let check = clap::Command::new("check")
        .short_flag('C')
        .about("checks, which packages are actually installed")
        .arg(remove_arg.clone());
    let install = clap::Command::new("install")
        .short_flag('I')
        .about("generates the pacman command and installs the LAST BUILD packages, CALLS SUDO!");
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
                .help("a possible name of a package to search for")
                .num_args(1),
        );
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
pub async fn search_cli(search_name: String) {
    let raur_handler = raur::Handle::new();
    match raur_handler.search(search_name.clone()).await {
        Ok(pkg_vec) => {
            for pkg in pkg_vec.clone() {
                println!("----------------------------------");
                println!(
                    "Name: {}; Version: {}; Updated: {}",
                    pkg.name, pkg.version, pkg.last_modified
                );
                println!(
                    "Description: {}",
                    pkg.description
                        .unwrap_or("no description available".to_string())
                );
                println!(
                    "Dependencies: \n runtime: {}, make: {}, check: {}, optional: {}",
                    pkg.depends.len(),
                    pkg.make_depends.len(),
                    pkg.check_depends.len(),
                    pkg.opt_depends.len()
                );
                // for dep in pkg.depends {
                //     print!("{}; ",dep);
                // }
                // for dep in pkg.make_depends {
                //     print!("{} [make]; ",dep);
                // }
                // for dep in pkg.opt_depends {
                //     print!("{} [optional]; ",dep);
                // }
                // println!("");
                println!(
                    "Upstream: {}",
                    pkg.url.unwrap_or("not available".to_string())
                );
                println!("Git Url: https://aur.archlinux.org/{}", pkg.package_base);
                println!("==================================");
            }
            println!("Found {} packages", pkg_vec.len());
        }
        Err(err) => {
            println!("Error while searching for {}: \n {}", search_name, err);
        }
    }
}
