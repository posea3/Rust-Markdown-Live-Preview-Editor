use std::{
    env,
    fs::{self, File, OpenOptions},
    io::Write as _,
    path::{Path, PathBuf},
};

use mdedit_input::{EditorInput, EditorSession, EditorTrace};

pub struct TraceCapture {
    path: PathBuf,
    file: File,
}

impl TraceCapture {
    pub fn from_env(session: &EditorSession) -> Option<Self> {
        let path = env::var_os("MDEDIT_TRACE_FILE").map(PathBuf::from)?;
        match create_trace(&path, &EditorTrace::from_session(session).encode()) {
            Ok(file) => {
                eprintln!("mdedit editor trace enabled: {}", path.display());
                Some(Self { path, file })
            }
            Err(error) => {
                eprintln!("editor trace create error: {error}");
                None
            }
        }
    }

    pub fn record(&mut self, input: &EditorInput) {
        let encoded = EditorTrace::encode_event_line(input);
        if let Err(error) = self.file.write_all(encoded.as_bytes()) {
            eprintln!(
                "editor trace append error ({}): {error}",
                self.path.display()
            );
        }
    }
}

fn create_trace(path: &Path, header: &str) -> std::io::Result<File> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    file.write_all(header.as_bytes())?;
    Ok(file)
}
