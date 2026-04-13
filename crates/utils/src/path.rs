use std::path::PathBuf;

pub fn hermes_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes")
}

pub fn config_path() -> PathBuf {
    hermes_home().join("config.yaml")
}

pub fn data_path() -> PathBuf {
    hermes_home().join("data")
}

pub fn sessions_path() -> PathBuf {
    data_path().join("sessions")
}
