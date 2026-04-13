use std::process::Command;
use std::time::Duration;

pub fn execute_code(code: &str, language: &str, timeout_secs: u64) -> String {
    let timeout = Duration::from_secs(timeout_secs.max(1).min(300));

    let result: Result<std::process::Output, String> = match language {
        "python" => {
            let interpreter = if cfg!(windows) { "python" } else { "python3" };
            run_command(interpreter, &["-c", code], timeout)
        }
        "javascript" => run_command("node", &["-e", code], timeout),
        "powershell" => {
            if cfg!(windows) {
                run_command("powershell", &["-Command", code], timeout)
            } else {
                Err("PowerShell is only available on Windows".to_string())
            }
        }
        "bash" => {
            if cfg!(windows) {
                run_command("cmd", &["/C", code], timeout)
            } else {
                run_command("sh", &["-c", code], timeout)
            }
        }
        "cmd" => {
            if cfg!(windows) {
                run_command("cmd", &["/C", code], timeout)
            } else {
                Err("cmd is only available on Windows".to_string())
            }
        }
        _ => Err(format!("Unsupported language: {}", language)),
    };

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let exit_code = output.status.code().unwrap_or(-1);

            serde_json::json!({
                "stdout": stdout.as_ref(),
                "stderr": stderr.as_ref(),
                "exit_code": exit_code,
                "error": if exit_code != 0 && stdout.is_empty() && stderr.is_empty() {
                    "Process exited with non-zero status but no output"
                } else {
                    ""
                }
            })
            .to_string()
        }
        Err(e) => serde_json::json!({
            "stdout": "",
            "stderr": "",
            "exit_code": -1,
            "error": e,
        })
        .to_string(),
    }
}

fn run_command(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<std::process::Output, String> {
    let mut cmd = Command::new(program);
    cmd.args(args);

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(format!("Interpreter not found: {}", program));
            }
            return Err(format!("Failed to spawn process: {}", e));
        }
    };

    match child.wait_with_timeout(timeout) {
        Ok(output) => Ok(output),
        Err(e) => Err(format!("Execution error: {}", e)),
    }
}

trait WaitWithTimeout {
    fn wait_with_timeout(self, timeout: Duration) -> Result<std::process::Output, std::io::Error>;
}

impl WaitWithTimeout for std::process::Child {
    fn wait_with_timeout(
        mut self,
        timeout: Duration,
    ) -> Result<std::process::Output, std::io::Error> {
        let start = std::time::Instant::now();

        loop {
            match self.try_wait() {
                Ok(Some(status)) => {
                    let stdout = self.stdout.take().map_or(Vec::new(), |mut h| {
                        let mut buf = Vec::new();
                        let _ = std::io::Read::read_to_end(&mut h, &mut buf);
                        buf
                    });
                    let stderr = self.stderr.take().map_or(Vec::new(), |mut h| {
                        let mut buf = Vec::new();
                        let _ = std::io::Read::read_to_end(&mut h, &mut buf);
                        buf
                    });
                    return Ok(std::process::Output {
                        status,
                        stdout,
                        stderr,
                    });
                }
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let _ = self.kill();
                        let _ = self.wait();
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            format!("Process timed out after {} seconds", timeout.as_secs()),
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}
