use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::process::Command;
use std::process::ExitStatus;

// TODO:
// aur_helper update [dir]
// aur_helper build [dir]
// ? aur_helper install [dir]
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

    match get_dirs(Path::new(&path), true) {
        Ok(x) => {
            for i in &x {
                let p = i.to_str().unwrap();
                println!("{p}");
            }
        }
        Err(x) => println!("Error: {}", x),
    }
}

// goes through the directories and calls 'git pull' and returns the exit status, if a command
// fails
fn update_packages(dirs: Vec<PathBuf>) -> Result<(), Vec<(PathBuf, ExitStatus)>> {
    let mut failed_dirs: Vec<(PathBuf, ExitStatus)> = Vec::new();
    for dir in dirs {
        let status = Command::new("git")
            .arg("pull")
            .current_dir(dir.clone())
            .status()
            .expect("Failed to execute git pull!");
        if !status.success() {
            failed_dirs.push((dir, status));
        }
    }
    if failed_dirs.is_empty() {
        return Ok(());
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
    use std::os::fd::AsFd;

    use super::*;
    #[test]
    fn get_dirs_detects_right_test() {
        // prepair
        let status = Command::new("mkdir")
            .current_dir("/tmp/")
            .arg("aur_relper_rs_test")
            .status()
            .expect("Couldn't created directory in /tmp!");
        if !status.success() {
            Command::new("rm")
                .current_dir("/tmp")
                .arg("-r")
                .arg("aur_relper_rs_test")
                .status()
                .expect("Error on remove of test directory BEFORE TEST!");

            Command::new("mkdir")
                .current_dir("/tmp/")
                .arg("aur_relper_rs_test")
                .status()
                .expect("Couldn't created directory in /tmp!");
        }
        Command::new("mkdir")
            .current_dir("/tmp/aur_relper_rs_test")
            .arg("test_dir1")
            .arg("test_dir2")
            .arg("test_dir3")
            .status()
            .expect("Couldn't create subdirs in test");
        // test
        let paths = get_dirs(Path::new("/tmp/aur_relper_rs_test/"), true);
        assert!(paths.is_ok());
        let actual_paths_cmd = Command::new("ls")
            .arg("-d1")
            .arg(r#"/tmp/aur_relper_rs_test/*"#)
            .output()
            .expect("ls couldn't excute!");
        let mut actual_paths: Vec<PathBuf> = Vec::new();
        // let str_path = actual_paths_cmd.stdout.escape_ascii();
        for x in actual_paths_cmd.stdout {
            let mut p = PathBuf::new();
            p.push(Path::new(x.escape_ascii().to_string()));
            actual_paths.push(p);
        }
        assert_eq!(paths.unwrap(), actual_paths);
        // clean up
        Command::new("rm")
            .current_dir("/tmp/")
            .arg("-r")
            .arg("aur_relper_rs_test")
            .status()
            .expect("Couldn't remove test directory AFTER TEST!!");
    }
}
