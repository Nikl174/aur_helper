use raur::Raur;
use std::fs::{DirEntry, ReadDir};
use std::process::{Command, ExitStatus};
use std::str::from_utf8;
use std::time::{Duration, SystemTime};
use std::{collections::HashSet, io, path::PathBuf};
use std::{fs, path::Path};

pub fn download_packages_from_git(
    current_path: &Path,
    git_links: Vec<String>,
) -> Result<Vec<PathBuf>, (Vec<PathBuf>, Vec<String>)> {
    let mut cloned_pkgs: Vec<PathBuf> = Vec::new();
    let mut failed_cloned_pkgs: Vec<String> = Vec::new();

    for link in git_links {
        let pkg_name = match link.split("/").last() {
            Some(name) => {
                if name.find(".").is_some() {
                    name.split(".")
                        .next()
                        .expect("Should be unreachable, name should have a '.'")
                } else {
                    name
                }
            }
            None => {
                failed_cloned_pkgs.push(link);
                continue;
            }
        };

        let cloned = match Command::new("git")
            .current_dir(current_path)
            .arg("clone")
            .arg(link.clone())
            .output()
        {
            Ok(proc) => proc.status,
            Err(_) => {
                failed_cloned_pkgs.push(link);
                continue;
            }
        };
        if cloned.success() {
            let pkg_path = current_path.join(pkg_name);
            cloned_pkgs.push(pkg_path);
        }
    }
    if failed_cloned_pkgs.is_empty() {
        return Ok(cloned_pkgs);
    } else {
        return Err((cloned_pkgs, failed_cloned_pkgs));
    }
}

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
            .arg("-s")
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
    let mut dep = pkg.depends;
    let mut mdep = pkg.make_depends;
    let mut odep = pkg.opt_depends;
    let mut cdep = pkg.check_depends;
    let max_dep_size = dep.len().max(mdep.len()).max(odep.len()).max(cdep.len());
    dep.resize(max_dep_size, "".to_string());
    mdep.resize(max_dep_size, "".to_string());
    odep.resize(max_dep_size, "".to_string());
    cdep.resize(max_dep_size, "".to_string());

    let big_dep = dep
        .iter()
        .zip(mdep.iter())
        .zip(odep.iter())
        .zip(cdep.iter()); // hehe ;)

    println!(
        "{: <25} | {: <25} | {: <25} | {: <25}",
        "[Runtime]", "[Make]", "[Optional]", "[Check]"
    );
    for (((d, md), od), cd) in big_dep {
        println!("{: <25} | {: <25} | {: <25} | {: <25}", d, md, od, cd);
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

// helper function to get the diff between a bigger and a set that is contained in the bigger one
pub fn get_set_diff(bigger_dir: Vec<PathBuf>, containing_dir: Vec<PathBuf>) -> Vec<PathBuf> {
    let bigger_set: HashSet<PathBuf> = bigger_dir.into_iter().collect();
    let containing_set: HashSet<PathBuf> = containing_dir.into_iter().collect();

    let diff: HashSet<PathBuf> = bigger_set
        .difference(&containing_set)
        .into_iter()
        .map(|x| x.clone())
        .collect();
    return diff.into_iter().collect();
}

pub async fn ext_search_aur(search_name: &String) {
    let raur_handler = raur::Handle::new();
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
}

pub async fn search_aur(search_name: &String) {
    let raur_handler = raur::Handle::new();
    match raur_handler.info(&[search_name.clone()]).await {
        Ok(pkg_vec) => {
            //println!("Pkg_vec len {}", pkg_vec.len());
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
#[cfg(test)]
mod tests {
    use super::*;
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
