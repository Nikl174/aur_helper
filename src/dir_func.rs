use std::fs;
use std::fs::DirEntry;
use std::fs::ReadDir;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use std::str::from_utf8;

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
