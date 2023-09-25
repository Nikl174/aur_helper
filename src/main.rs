use std::path::PathBuf;

mod cli;
mod dir_func;

// TODO:
// aur_helper update [dir]
// aur_helper build [dir]
// ? aur_helper install [dir]
// clap for cli tool
// TODO: error output improve
// TODO: improve code-structure
#[tokio::main]
async fn main() {
    let command_matches = cli::Cli::new().get_cli_command().get_matches();

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
        Some(("update", sub_matches)) => {
            cli::update_command(dirs, sub_matches.to_owned());
        }
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
        Some((cmd, sub_matches)) => {
            println!("Unknown command '{cmd} {:?}'", sub_matches);
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn utf8_conversion_test() {
        let tmp_path = "/tmp/aur_helper_rs_test/utf8_conversion_test/";
        prepair_tmp_dir(tmp_path);
        Command::new("git")
            .current_dir(&tmp_path)
            .arg("clone")
            .arg("https://aur.archlinux.org/sway-audio-idle-inhibit-git.git")
            .status()
            .expect("Failed to clone the repository!");
        let output = Command::new("git")
            .arg("pull")
            .current_dir(&(tmp_path.to_owned() + "sway-audio-idle-inhibit-git"))
            .output()
            .expect("Failed to execute git pull!");

        assert_eq!(
            from_utf8(output.stdout.as_slice()).unwrap(),
            "Already up to date.\n"
        );
        clean_up_tmp_dir(tmp_path);
    }

    use std::{
        iter,
        path::{Path, PathBuf},
        process::Command,
        str::from_utf8,
    };

    use super::*;

    // Helper function for a test preperation, creates a temp directory for test purposes
    fn prepair_tmp_dir(tmp_dir_abs_path: &str) {
        clean_up_tmp_dir(tmp_dir_abs_path);
        Command::new("mkdir")
            .arg("-p")
            .arg(tmp_dir_abs_path)
            .status()
            .expect("mkdir couldn't execute to create temporary test dir");
    }

    // Helper function for clean up the created test directory after test
    fn clean_up_tmp_dir(tmp_dir_abs_path: &str) {
        if Command::new("ls")
            .arg(tmp_dir_abs_path)
            .output()
            .expect("Couldn't ls a directory")
            .status
            .success()
        {
            println!("Removing: {}", tmp_dir_abs_path);
            Command::new("rm")
                .arg("-rf")
                .arg(tmp_dir_abs_path)
                .status()
                .expect("Couldn't remove test directory with rm");
        }
    }

    fn prepair_aur_test_dir(tmp_dir_abs_path: &str, git_links: &Vec<String>) {
        prepair_tmp_dir(tmp_dir_abs_path);
        for link in git_links {
            Command::new("git")
                .current_dir(tmp_dir_abs_path)
                .arg("clone")
                .arg(link)
                .status()
                .expect("Failed to execute git clone in tmp dir");
        }
    }

    #[test]
    fn get_dirs_detects_right_test() {
        // prepair
        let tmp_path = "/tmp/aur_helper_rs_test/get_dirs_detects_right_test".to_string();
        prepair_tmp_dir(&tmp_path.as_str());
        Command::new("mkdir")
            .current_dir(tmp_path.as_str())
            .arg("test_dir1")
            .arg("test_dir2")
            .arg("test_dir3")
            .status()
            .expect("Couldn't create subdirs in test");

        // TEST
        let paths = dir_func::get_dirs(Path::new("/tmp/aur_helper_rs_test/"), true);

        assert!(paths.is_ok());

        let mut actual_paths: Vec<PathBuf> = Vec::new();
        actual_paths.push(Path::new(&(tmp_path.clone() + "test_dir1")).to_path_buf());
        actual_paths.push(Path::new(&(tmp_path.clone() + "test_dir2")).to_path_buf());
        actual_paths.push(Path::new(&(tmp_path.clone() + "test_dir3")).to_path_buf());

        assert_eq!(paths.unwrap().sort(), actual_paths.sort());

        // clean up
        clean_up_tmp_dir(&tmp_path);
    }

    #[test]
    fn get_dirs_error_test() {
        // prepair
        let tmp_path = "/tmp/aur_helper_rs_test/get_dirs_error_test";
        let test_file = tmp_path.to_owned() + "/test_file.txt";
        prepair_tmp_dir(&tmp_path);
        println!("{}", test_file.as_str());

        Command::new("touch")
            .arg(test_file.as_str())
            .status()
            .expect("Failed to touch a file in the test directory");

        // TEST
        let err = dir_func::get_dirs(Path::new(tmp_path), true);
        let err2 = dir_func::get_dirs(Path::new(test_file.as_str()), true);

        assert!(err.is_err());
        assert!(err2.is_err());

        // clean up
        clean_up_tmp_dir(&tmp_path);
    }

    #[test]
    fn update_packages_test() {
        let tmp_path = "/tmp/aur_helper_rs_test/update_packages_test/";
        let mut git_links: Vec<String> = Vec::new();
        git_links.push("https://aur.archlinux.org/swaylock-blur-bin.git".to_owned());
        git_links.push("https://aur.archlinux.org/yofi-bin.git".to_owned());
        prepair_aur_test_dir(tmp_path, &git_links);
        let update_dir_path = tmp_path.to_owned() + "yofi-bin";

        Command::new("git")
            .current_dir(&update_dir_path)
            .arg("reset")
            .arg("HEAD^^")
            .status()
            .expect("Failed to reset the repo yofi-bin");
        Command::new("git")
            .current_dir(&update_dir_path)
            .arg("restore")
            .arg(".")
            .status()
            .expect("Failed to restore the repo yofi-bin");
        let dirs = dir_func::get_dirs(Path::new(tmp_path), true);
        assert!(dirs.is_ok());
        let mut updated_dirs: Vec<PathBuf> = Vec::new();
        updated_dirs.push(Path::new(&update_dir_path).to_path_buf());
        let success_dirs = dir_func::update_packages(dirs.unwrap());
        assert!(success_dirs.is_ok());
        for i in iter::zip(success_dirs.unwrap(), updated_dirs) {
            assert_eq!(i.0, i.1);
        }

        clean_up_tmp_dir(tmp_path);
    }

    #[test]
    fn build_packages_test() {
        let tmp_path = "/tmp/aur_helper_rs_test/build_packages_test/";
        let mut git_links: Vec<String> = Vec::new();
        git_links.push("https://aur.archlinux.org/swaylock-blur-bin.git".to_owned());
        git_links.push("https://aur.archlinux.org/yofi-bin".to_owned());
        prepair_aur_test_dir(tmp_path, &git_links);

        let mut dirs = dir_func::get_dirs(&Path::new(tmp_path), true);
        assert!(dirs.is_ok());
        let no_err = dir_func::build_packages(dirs.unwrap());
        assert!(no_err.is_ok());

        clean_up_tmp_dir(tmp_path);

        git_links.clear();
        git_links.push("https://github.com/Nikl174/simple_aur_helper.git".to_owned());

        prepair_aur_test_dir(tmp_path, &git_links);

        dirs = dir_func::get_dirs(&Path::new(tmp_path), true);
        assert!(dirs.is_ok());
        let err = dir_func::build_packages(dirs.unwrap());
        assert!(err.is_err());

        clean_up_tmp_dir(tmp_path);
    }

    #[test]
    fn install_packages_test() {
        let tmp_path = "/tmp/aur_helper_rs_test/find_build_packages/";
        let mut git_links: Vec<String> = Vec::new();
        git_links.push("https://aur.archlinux.org/swaylock-blur-bin.git".to_owned());
        git_links.push("https://aur.archlinux.org/packages/yofi-bin".to_owned());
        git_links.push("https://aur.archlinux.org/piow-bin.git".to_owned());
        prepair_aur_test_dir(tmp_path, &git_links);

        let dirs = dir_func::get_dirs(&Path::new(tmp_path), true);
        assert!(dirs.is_ok());

        let dirs = dirs.unwrap();

        let no_err = dir_func::build_packages(dirs.clone());
        assert!(no_err.is_ok());

        let no_err = dir_func::install_packages(dirs.clone());
        assert!(no_err.is_ok());

        clean_up_tmp_dir(tmp_path);
    }

    #[test]
    fn cli_test() {
        cli::Cli::new().get_cli_command().debug_assert();
    }
}
