use std::{io, path::PathBuf};

use clap::ArgMatches;

mod cli;

// TODO:
// aur_helper update [dir]
// aur_helper build [dir]
// ? aur_helper install [dir]
// clap for cli tool
// TODO: error output improve
// TODO: improve code-structure
#[tokio::main]
async fn main() {
    let cli = cli::Cli::new(None);
    let dir = cli.get_aur_dir();
    let command_matches = cli.get_cli_command().get_matches();

    let path = command_matches
        .get_one::<PathBuf>("AUR_PATH")
        .expect("AUR_PATH argument is required but not found!");

    let commands = ["update", "build", "install", "check"];

    match command_matches.subcommand() {
        // Some(("update", sub_matches))
        // | Some(("build", sub_matches))
        // | Some(("install", sub_matches))
        // | Some(("check", sub_matches)) => {
        Some((cmd, sub_matches)) if commands.contains(&cmd) => {
            let dirs = match get_dirs(path, sub_matches) {
                Ok(dirs) => dirs,
                Err(err) => {
                    println!(
                        "ERROR: Couldn't get the directories in the AUR-Directory, error: \n {}",
                        err
                    );
                    return;
                }
            };

            match cmd {
                "update" => cli::update_command(dirs, sub_matches.to_owned()),
                "build" => cli::build_command(dirs, sub_matches.to_owned()),
                "install" => cli::install_command(dirs),
                "check" => cli::check_command(dirs, sub_matches.to_owned()),
                _ => unreachable!(),
            }
        }
        Some(("search", sub_matches)) => {
            cli::search_command(sub_matches.to_owned()).await;
        }
        Some(("get-aur-dir", _)) => {
            println!("{dir}");
        }
        Some((cmd, sub_matches)) => {
            println!("Unknown command '{cmd} {:?}'", sub_matches);
        }
        _ => unreachable!(),
    }
}

fn get_dirs(aur_path: &PathBuf, sub_matches: &ArgMatches) -> Result<Vec<PathBuf>, io::Error> {
    let aur_package: Vec<PathBuf> = match sub_matches.get_many::<String>("AUR_PACKAGES") {
        Some(packages) => packages
            .map_while(|pkg_str| {
                let mut pkg_path = aur_path.clone();
                pkg_path.push(pkg_str);
                if !pkg_path.is_dir() {
                    print!(
                        "{} is not an existing directory!",
                        pkg_path
                            .to_str()
                            .expect("Couldn't convert pkg_path to string!")
                    );
                    return None;
                }
                Some(pkg_path)
            })
            .collect(),
        None => dir_func::get_dirs(aur_path.as_path(), true)?,
    };
    if aur_package.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "One of the packages is not a is not a directory in the AUR!",
        ));
    }
    Ok(aur_package)
}

#[test]
fn cli_test() {
    cli::Cli::new(None).get_cli_command().debug_assert();
}
