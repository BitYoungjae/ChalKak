use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigPathError {
    MissingHomeDirectory,
}

pub(crate) fn config_env_dirs() -> (Option<PathBuf>, Option<PathBuf>) {
    (
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        std::env::var_os("HOME").map(PathBuf::from),
    )
}

pub(crate) fn app_config_path(
    app_dir: &str,
    file_name: &str,
    xdg_config_home: Option<&Path>,
    home: Option<&Path>,
) -> Result<PathBuf, ConfigPathError> {
    let mut path = config_root(xdg_config_home, home)?;
    path.push(app_dir);
    path.push(file_name);
    Ok(path)
}

fn config_root(
    xdg_config_home: Option<&Path>,
    home: Option<&Path>,
) -> Result<PathBuf, ConfigPathError> {
    if let Some(xdg) = xdg_config_home.filter(|path| !path.as_os_str().is_empty()) {
        return Ok(xdg.to_path_buf());
    }

    let home = home.ok_or(ConfigPathError::MissingHomeDirectory)?;
    Ok(home.join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_config_path_prefers_xdg_config_home() {
        let path = app_config_path(
            "chalkak",
            "theme.json",
            Some(Path::new("/tmp/config-root")),
            Some(Path::new("/tmp/home")),
        )
        .expect("path should resolve");

        assert_eq!(path, PathBuf::from("/tmp/config-root/chalkak/theme.json"));
    }

    #[test]
    fn app_config_path_falls_back_to_home_dot_config() {
        let path = app_config_path("chalkak", "theme.json", None, Some(Path::new("/tmp/home")))
            .expect("path should resolve");

        assert_eq!(path, PathBuf::from("/tmp/home/.config/chalkak/theme.json"));
    }

    #[test]
    fn app_config_path_errors_when_home_missing_and_xdg_unset() {
        let error = app_config_path("chalkak", "theme.json", None, None).unwrap_err();
        assert_eq!(error, ConfigPathError::MissingHomeDirectory);
    }
}
