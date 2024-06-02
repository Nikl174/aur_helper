use std::path::PathBuf;

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

    let dirs = match dir_func::get_dirs(path, true) {
        Ok(dirs) => dirs,
        Err(err) => {
            println!(
                "WARNING: Couldn't get the aur directory paths because of this error:\n {}",
                err
            );
            return;
        }
    };

    match command_matches.subcommand() {
        Some(("update", sub_matches)) => cli::update_command(dirs, sub_matches.to_owned()),
        Some(("build", sub_matches)) => {
            cli::build_command(dirs, sub_matches.to_owned());
        }
        Some(("install", _sub_matches)) => {
            cli::install_command(dirs);
        }
        Some(("check", sub_matches)) => {
            cli::check_command(dirs, sub_matches.to_owned());
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

#[test]
fn cli_test() {
    cli::Cli::new(None).get_cli_command().debug_assert();
}
