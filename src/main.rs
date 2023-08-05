use std::env;
use std::fs;
use std::io;
use std::iter;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use std::str;
use std::str::from_utf8;

// TODO:
// aur_helper update [dir]
// aur_helper build [dir]
// ? aur_helper install [dir]
// clap for cli tool
fn main() {
    let mut args = env::args();
    // skip programm name
    args.next();
    let dir = args.next();

    let path = match dir {
        Some(x) => x,
        None => {
            println!("No argument provided!");
            return;
        }
    };

    let mut dirs: Vec<PathBuf> = Vec::new();
    match get_dirs(Path::new(&path), true) {
        Ok(x) => {
            for i in &x {
                let p = i.to_str().unwrap();
                println!("{p}");
            }
            dirs = x;
        }
        Err(x) => println!("Error: {}", x),
    }
    match update_packages(&dirs) {
        Ok(vec) => {
            for x in vec {
                if x.1 {
                    println!("Updated package: {}", x.0.to_str().unwrap());
                } else {
                    println!("Package {} already up to date", x.0.to_str().unwrap());
                }
            }
        }
        Err(vec) => {
            for x in vec {
                println!(
                    "Couldn't update package {}, failed with error {}",
                    x.0.to_str().unwrap(),
                    x.1.to_string()
                )
            }
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    /// update packages
    #[arg(short, long)]
    update: Option<bool>,

    /// build packages with 'makepkg'
    #[arg(short, long)]
    build: Option<bool>,

    /// install the packages with pacman, requires root!
    #[arg(short, long)]
    install: Option<bool>,
}

// fn install_packages(dirs: &Vec<PathBuf>) -> Result<Vec<PathBuf>, Vec<PathBuf>> {
fn install_packages(dirs: Vec<PathBuf>) -> Result<Command, Vec<PathBuf>> {
    let mut failed_packges: Vec<PathBuf> = Vec::new();
    let mut packages: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        match fs::read_dir(dir.clone()) {
            Ok(read_dir) => {
                let files = get_latest_build_package(read_dir);
                match files {
                    Ok(path_buf) => {
                        packages.push(path_buf);
                    }
                    Err(e) => {
                        println!("WARNING: couldn't find a build package, error: {}", e);
                        failed_packges.push(dir.to_path_buf());
                    }
                }
            }
            Err(_) => {
                failed_packges.push(dir.to_path_buf());
            }
        }
    }

    if failed_packges.is_empty() {
        let mut inst_cmd = Command::new("sudo");
        inst_cmd.arg("pacman");
        inst_cmd.arg("-U");
        for package in packages {
            inst_cmd.arg(package.to_str().expect("Couldn't convert path to string"));
        }
        println!("{:?}", inst_cmd);
        Ok(inst_cmd)
    } else {
        Err(failed_packges)
    }
}

// finds the build package-files in a directory
fn get_latest_build_package(dir: ReadDir) -> Result<PathBuf, io::Error> {
    let mut possible_packages: Vec<DirEntry> = Vec::new();
    for file_res in dir {
        // TODO: better error handling
        let file = file_res.expect("read_dir failed to retreave a file");
        let file_metadata = file.metadata().expect("couldn't get metadata");
        if file_metadata.is_file() {
            let file_name = file
                .file_name()
                .into_string()
                .expect("Failed to convert from OsString to String");
            let file_name_slice: Vec<&str> = file_name.split(".").collect();
            match file_name_slice.last() {
                Some(last) => {
                    if last == &"zst" {
                        possible_packages.push(file);
                    }
                }
                None => {}
            }
        }
    }
    if possible_packages.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Couldn't find a build package-file",
        ));
    } else {
        // TODO: don't depend on metada, depend on pkg version name
        let mut min: DirEntry = possible_packages.pop().unwrap();
        let mut min_modified = min.metadata()?.modified().unwrap();
        for file in possible_packages {
            let file_modifed = file.metadata()?.modified().unwrap(); // unwrap because it is always available in linux
            if file_modifed < min_modified {
                min = file;
                min_modified = file_modifed;
            };
        }
        return Ok(min.path());
    }
}

// build all packages in dir, returns build packages of on err the failed packages and the status
fn build_packages(dirs: Vec<PathBuf>) -> Result<Vec<PathBuf>, Vec<(PathBuf, ExitStatus)>> {
    let mut failed_dirs: Vec<(PathBuf, ExitStatus)> = Vec::new();
    let mut success_dirs: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        let status = Command::new("makepkg")
            .current_dir(dir.clone())
            .status()
            .expect("Failed to execute makepkg");
        if status.success() {
            success_dirs.push(dir.clone());
        } else {
            failed_dirs.push((dir.clone(), status));
        }
    }
    if failed_dirs.is_empty() {
        return Ok(success_dirs);
    }
    Err(failed_dirs)
}

// goes through the directories and calls 'git pull' and returns a tuple vector of successful updated
// dirs and on fail a vector of the failed dirs and their  the exit status
fn update_packages(dirs: Vec<PathBuf>) -> Result<Vec<PathBuf>, Vec<(PathBuf, ExitStatus)>> {
    let mut failed_dirs: Vec<(PathBuf, ExitStatus)> = Vec::new();
    let mut true_success_dirs: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        let output = Command::new("git")
            .arg("pull")
            .current_dir(dir.clone())
            .output()
            .expect("Failed to execute git pull!");

        if output.status.success() {
            if !(from_utf8(output.stdout.as_slice()).unwrap() == "Already up to date.\n") {
                true_success_dirs.push(dir.clone());
            }
        } else {
            failed_dirs.push((dir.clone(), output.status));
        }
    }
    if failed_dirs.is_empty() {
        return Ok(true_success_dirs);
    }
    Err(failed_dirs)
}

// returns the directories in path and warns if it's a wrong directory
fn get_dirs(current_path: &Path, warn_wrong_dir: bool) -> Result<Vec<PathBuf>, io::Error> {
    let mut paths: Vec<PathBuf> = Vec::new();
    if !current_path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Given Path is not a directory!",
        ));
    }

    for path in fs::read_dir(current_path).expect("Couldn't read path!") {
        let dir_ent = path.expect("Dir-entry error!").path();

        if warn_wrong_dir && !dir_ent.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "AUR Path contains not only directorys, right AUR-dir?",
            ));
        }

        paths.push(dir_ent);
    }
    Ok(paths)
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
        let paths = get_dirs(Path::new("/tmp/aur_helper_rs_test/"), true);

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
        let err = get_dirs(Path::new(tmp_path), true);
        let err2 = get_dirs(Path::new(test_file.as_str()), true);

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
        let dirs = get_dirs(Path::new(tmp_path), true);
        assert!(dirs.is_ok());
        let mut updated_dirs: Vec<PathBuf> = Vec::new();
        updated_dirs.push(Path::new(&update_dir_path).to_path_buf());
        let success_dirs = update_packages(dirs.unwrap());
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

        let mut dirs = get_dirs(&Path::new(tmp_path), true);
        assert!(dirs.is_ok());
        let no_err = build_packages(dirs.unwrap());
        assert!(no_err.is_ok());

        clean_up_tmp_dir(tmp_path);

        git_links.clear();
        git_links.push("https://github.com/Nikl174/simple_aur_helper.git".to_owned());

        prepair_aur_test_dir(tmp_path, &git_links);

        dirs = get_dirs(&Path::new(tmp_path), true);
        assert!(dirs.is_ok());
        let err = build_packages(dirs.unwrap());
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

        let dirs = get_dirs(&Path::new(tmp_path), true);
        assert!(dirs.is_ok());

        let dirs = dirs.unwrap();

        let no_err = build_packages(dirs.clone());
        assert!(no_err.is_ok());

        let no_err = install_packages(dirs.clone());
        assert!(no_err.is_ok());

        clean_up_tmp_dir(tmp_path);
    }
}
