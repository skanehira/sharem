use crossterm::terminal;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    terminal::enable_raw_mode()?;
    let pty_system = NativePtySystem::default();

    let window_size = terminal::window_size()?;

    let pair = pty_system.openpty(PtySize {
        rows: window_size.rows,
        cols: window_size.columns,
        pixel_width: window_size.width,
        pixel_height: window_size.height,
    })?;

    let mut cmd = CommandBuilder::new(std::env!("SHELL"));
    cmd.cwd(std::env::current_dir().unwrap_or(std::env!("HOME").into()));

    let mut child = pair.slave.spawn_command(cmd)?;

    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;

    std::thread::spawn(move || {
        let mut out = std::io::stdout().lock();
        loop {
            let mut buf = [0; 1024];
            let n = reader.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).unwrap();
            out.flush().unwrap();
        }
    });

    let mut writer = pair.master.take_writer()?;

    if cfg!(target_os = "macos") {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        std::io::copy(&mut stdin, &mut writer).unwrap();
    });

    let status = child.wait()?;
    if !status.success() {
        eprintln!("child exited: {}", status.exit_code());
    }
    terminal::disable_raw_mode()?;
    Ok(())
}
