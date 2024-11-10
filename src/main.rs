use libamx::{LegacyMode, StxFile};
use librespack::RespackFile;
use std::error::Error;
use std::io;
use std::io::{Cursor, Write};
use std::path::Path;
use std::{env, fs};
use strum::IntoEnumIterator;

const STEP_DAT_FILE: &str = "STEP.DAT";
const STEP_DIR: &str = "STEP";

type Result<T = ()> = std::result::Result<T, Box<dyn Error + 'static>>;

fn pause() -> Result {
    let mut stdout = io::stdout();
    write!(stdout, "\nPress ENTER to continue... ")?;
    stdout.flush()?;
    io::stdin().read_line(&mut String::new())?;

    Ok(())
}

fn help(path: &Path) -> Result {
    let filename = {
        let filename = String::from(path.file_name().unwrap().to_str().unwrap());
        let extension = path
            .extension()
            .unwrap_or(Default::default())
            .to_str()
            .unwrap_or("")
            .to_lowercase();
        match extension.as_str() {
            "exe" => filename,
            _ => format!("./{}", filename),
        }
    };
    println!(
        "»» {} v{} by Aldo_MX\n",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    println!("» Description: {}\n", env!("CARGO_PKG_DESCRIPTION"));
    println!("» Example: {} +20\n", filename);
    pause()?;

    Ok(())
}

fn apply_offset(stx_file: &mut StxFile, offset: i32) -> Result {
    for mode in LegacyMode::iter() {
        let mut step_data = stx_file.read_step_data(mode)?;

        let first_split = step_data
            .splits
            .first_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No splits found."))?;

        if first_split.blocks.is_empty() {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "No blocks found in the first split.",
            ))?
        }

        for block in &mut first_split.blocks {
            block.delay_ms = block.delay_ms.saturating_add(offset.saturating_mul(10));
        }

        stx_file.set_step_data(stx_file.get_version(), step_data)?;
    }

    Ok(())
}

fn is_step_dat() -> Option<bool> {
    let path = Path::new(".");

    if let Ok(metadata) = path.join(STEP_DAT_FILE).metadata() {
        if metadata.is_file() {
            return Some(true);
        }
    }

    if let Ok(metadata) = path.join(STEP_DIR).metadata() {
        if metadata.is_dir() {
            return Some(false);
        }
    }

    None
}

fn walk_step_dat(offset: i32) -> Result {
    let mut stderr = io::stderr();
    let mut stdout = io::stdout();

    let mut respack_file = RespackFile::load(Path::new(STEP_DAT_FILE).into())?;
    for file_name in respack_file.get_file_names_sorted_by_name() {
        let extension = file_name
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();

        if extension == "STX" {
            write!(stdout, "Applying offset to {}... ", file_name)?;
            match (|| -> Result {
                let mut cursor = Cursor::new(respack_file.read_file_data(&file_name)?);
                let mut stx_file = StxFile::from_cursor(Path::new(&file_name).into(), &mut cursor)?;
                apply_offset(&mut stx_file, offset)?;
                respack_file
                    .set_file_data(&file_name, &mut stx_file.to_buffer(stx_file.get_version())?)?;
                fs::write(STEP_DAT_FILE, respack_file.to_buffer()?)?;
                Ok(())
            })() {
                Ok(_) => writeln!(stdout, "OK")?,
                Err(e) => {
                    writeln!(stdout, "ERROR")?;
                    writeln!(stderr, "Error in {}: {}", file_name, e)?;
                }
            }
        }
    }

    Ok(())
}

fn walk_step_dir(offset: i32) -> Result {
    let mut stderr = io::stderr();
    let mut stdout = io::stdout();

    let path = Path::new(STEP_DIR);
    for entry in path.read_dir()? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let extension = file_name
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_ascii_uppercase();

            if extension == "STX" {
                let stx_path = entry.path();
                write!(stdout, "Applying offset to {}... ", stx_path.display())?;
                match (|| -> Result {
                    let mut stx_file = StxFile::from_path(stx_path.clone())?;
                    apply_offset(&mut stx_file, offset)?;
                    fs::write(&stx_path, stx_file.to_buffer(stx_file.get_version())?)?;
                    Ok(())
                })() {
                    Ok(_) => writeln!(stdout, "OK")?,
                    Err(e) => {
                        writeln!(stdout, "ERROR")?;
                        writeln!(stderr, "Error in {}: {}", stx_path.display(), e)?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn main() -> Result {
    let raw_args: Vec<String> = env::args().collect();
    let (exe, args) = raw_args.split_first().unwrap();
    if args.len() == 0 || args.len() > 1 {
        let path = Path::new(exe);
        help(&path)?;
        return Ok(());
    }

    let offset = args[0].parse::<i32>().unwrap_or(0);

    let mut has_errors = false;
    let mut stderr = io::stderr();

    match is_step_dat() {
        Some(true) => Ok(walk_step_dat(offset)?),
        Some(false) => Ok(walk_step_dir(offset)?),
        None => Err("No STEP.DAT or STEP directory found."),
    }
    .unwrap_or_else(|error| {
        has_errors = true;
        writeln!(stderr, "Error: {}", error).unwrap();
    });

    if has_errors {
        pause()?;
    }

    Ok(())
}
