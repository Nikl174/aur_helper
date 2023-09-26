use raur::Raur;
use std::fs::{DirEntry, ReadDir};
use std::process::{Command, ExitStatus};
use std::str::from_utf8;
use std::time::{Duration, SystemTime};
use std::{collections::HashSet, io, path::PathBuf};
use std::{fs, path::Path};

use clap::ArgMatches;

pub fn remove_uninstalled_dirs(paths: Vec<PathBuf>) -> Option<Command> {
    let mut rm_cmd = Command::new("rm");
    rm_cmd.arg("-R").arg("-f");
    if paths.is_empty() {
        return None;
    }
    for path in paths {
        rm_cmd.arg(path.to_str().unwrap());
    }
    Some(rm_cmd)
}

pub fn check_installed(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>, io::Error> {
    let mut found_pgks: Vec<PathBuf> = Vec::new();

    for path in paths {
        let status = Command::new("pacman")
            .arg("-Q")
            .arg(
                path.file_name()
                    .expect("couldn't get a filename -> maybe got . or .."),
            )
            .output()?
            .status;
        if status.success() {
            found_pgks.push(path);
        }
    }
    Ok(found_pgks)
}

pub fn install_packages(dirs: Vec<PathBuf>) -> Result<Command, (Command, Vec<PathBuf>)> {
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
    let mut inst_cmd = Command::new("sudo");
    inst_cmd.arg("pacman");
    inst_cmd.arg("-U");
    for package in packages {
        inst_cmd.arg(package.to_str().expect("Couldn't convert path to string"));
    }
    if failed_packges.is_empty() {
        Ok(inst_cmd)
    } else {
        Err((inst_cmd, failed_packges))
    }
}

// finds the build package-files in a directory and fails, if not file was found
pub fn get_latest_build_package(dir: ReadDir) -> Result<PathBuf, io::Error> {
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
            if file_modifed > min_modified {
                min = file;
                min_modified = file_modifed;
            };
        }
        return Ok(min.path());
    }
}

// build all packages in dir, returns build packages of on err the failed packages and the status
pub fn build_packages(dirs: Vec<PathBuf>) -> Result<Vec<PathBuf>, Vec<(PathBuf, ExitStatus)>> {
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
pub fn update_packages(dirs: Vec<PathBuf>) -> Result<Vec<PathBuf>, Vec<(PathBuf, ExitStatus)>> {
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
                println!("{} updated!", dir.display());
            }
            println!("{} up to date!", dir.display());
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
pub fn get_dirs(current_path: &Path, warn_wrong_dir: bool) -> Result<Vec<PathBuf>, io::Error> {
    let mut paths: Vec<PathBuf> = Vec::new();
    println!("{:?}", current_path.canonicalize());
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

pub fn print_detailed_pkg_info(pkg: raur::Package) {
    // ------ calculate time
    let last_mod = Duration::new(pkg.last_modified.unsigned_abs(), 0);
    let last_mod = SystemTime::now() - last_mod;
    // last_mod is SystemTime, convert SystemTime to unix time
    let last_mod = last_mod
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Negative result on duration calculation");

    let days = last_mod.as_secs() / (60 * 60 * 24);

    let last_mod = match days {
        0 => format!("today"),
        1 => format!("before {} day", days),
        2..=31 => format!("before {} days", days),
        32..=365 => format!("before {} months", (days / 31)),
        _ => format!("before {} years", (days / 365)),
    };

    // ------ format print
    println!(
        "Name: {}; Version: {}; Updated: {}",
        pkg.name, pkg.version, last_mod
    );
    println!(
        "Description: {}",
        pkg.description
            .unwrap_or("no description available".to_string())
    );
    let dep = pkg.depends.iter();
    let mdep = pkg.make_depends.iter();
    let odep = pkg.opt_depends.iter();
    let big_dep = dep.zip(mdep).zip(odep); // hehe ;)

    println!(
        "{: <20} | {: <20} | {: <20}",
        "[Runtime]", "[Make]", "[Optional]"
    );
    for ((d, md), od) in big_dep {
        println!("{: <20} | {: <20} | {: <20}", d, md, od);
    }
    println!("");
    println!(
        "Upstream: {}",
        pkg.url.unwrap_or("not available".to_string())
    );
    println!("Git Url: https://aur.archlinux.org/{}", pkg.package_base);
}

pub fn print_simple_pkg_info(pkg: raur::Package) {
    println!("Name: {}; Popularity: {}", pkg.name, pkg.popularity);
    println!(
        "Description: {}",
        pkg.description
            .unwrap_or("no description available".to_string())
    );
    println!("==================================");
}
//
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

// helper function to get the diff between a bigger and a set that is contained in the bigger one
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
            if confirm_ask().is_err() {
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
            if confirm_ask().is_err() {
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
pub async fn search_command(sub_matches: ArgMatches) {
    let raur_handler = raur::Handle::new();
    let ext_search = sub_matches.get_flag("search");
    let search_name: &String = sub_matches
        .get_one::<String>("search_name")
        .expect("search_name argument required but couldn't get it");
    if ext_search {
        match raur_handler.search(search_name.clone()).await {
            Ok(pkg_vec) => {
                for pkg in pkg_vec {
                    print_simple_pkg_info(pkg);
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
                print_detailed_pkg_info(pkg);
            }
            Err(err) => {
                println!("Error while searching for {}: \n {}", search_name, err);
            }
        }
    }
}
