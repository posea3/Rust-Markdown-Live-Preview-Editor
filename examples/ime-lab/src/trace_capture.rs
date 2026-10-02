use std::{
    env, fs,
    path::{Path, PathBuf},
};

use mdedit_input::{EditorInput, EditorSession, EditorTrace};

pub struct TraceCapture {
    path: PathBuf,
    trace: EditorTrace,
}

impl TraceCapture {
    pub fn from_env(session: &EditorSession) -> Option<Self> {
        let path = env::var_os("MDEDIT_TRACE_FILE").map(PathBuf::from)?;
        let capture = Self {
            path,
            trace: EditorTrace::from_session(session),
        };
        capture.flush();
        eprintln!("mdedit editor trace enabled: {}", capture.path.display());
        Some(capture)
    }

    pub fn record(&mut self, input: &EditorInput) {
        self.trace.push(input.clone());
        self.flush();
    }

    fn flush(&self) {
        if let Err(error) = write_trace(&self.path, &self.trace.encode()) {
            eprintln!("editor trace write error: {error}");
        }
    }
}

fn write_trace(path: &Path, trace: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, trace)
}
