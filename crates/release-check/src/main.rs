//! `release-check <file> <file.sig> <version> (--conf <tauri.conf.json> | --key <base64>)`
//!
//! Exits 0 when `<file.sig>` is a signature of `<file>` by the app's updater
//! key, made for `<version>`; 1 when it is refused (the reason is printed);
//! 2 on wrong use or a file that cannot be read. `--key` is for the release
//! workflow's dry run, which signs with a throwaway key.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [file, sig, version, how, source] = args.as_slice() else {
        eprintln!("usage: release-check <file> <file.sig> <version> (--conf <tauri.conf.json> | --key <base64>)");
        return ExitCode::from(2);
    };
    let read = |path: &str| std::fs::read(path).map_err(|e| eprintln!("{path}: {e}"));
    let key = match how.as_str() {
        "--key" => source.clone(),
        "--conf" => {
            let Ok(conf) = read(source) else {
                return ExitCode::from(2);
            };
            match release_check::key_from_conf(&String::from_utf8_lossy(&conf)) {
                Ok(key) => key,
                Err(refusal) => {
                    eprintln!("{source}: {refusal}");
                    return ExitCode::from(1);
                }
            }
        }
        _ => {
            eprintln!("give the key with --conf or --key");
            return ExitCode::from(2);
        }
    };
    let (Ok(data), Ok(signature)) = (read(file), read(sig)) else {
        return ExitCode::from(2);
    };
    match release_check::check(&data, &String::from_utf8_lossy(&signature), &key, version) {
        Ok(()) => {
            println!("{file}: signed by the updater key for version {version}");
            ExitCode::SUCCESS
        }
        Err(refusal) => {
            eprintln!("{file}: refused: {refusal}");
            ExitCode::from(1)
        }
    }
}
