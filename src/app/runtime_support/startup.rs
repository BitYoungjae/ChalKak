#[derive(Debug, Clone, Copy, Default)]
pub(crate) enum StartupCaptureMode {
    #[default]
    None,
    Full,
    Region,
    Window,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StartupConfig {
    pub(crate) capture: StartupCaptureMode,
    pub(crate) show_launchpad: bool,
}

impl StartupConfig {
    pub(crate) fn from_args() -> Self {
        Self::from_iter(std::env::args().skip(1))
    }

    fn from_iter<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut config = Self {
            capture: StartupCaptureMode::None,
            show_launchpad: false,
        };

        for arg in args {
            match arg.as_ref() {
                "--capture-full" | "--full" => {
                    config.capture = StartupCaptureMode::Full;
                }
                "--capture-region" | "--region" => {
                    config.capture = StartupCaptureMode::Region;
                }
                "--capture-window" | "--window" => {
                    config.capture = StartupCaptureMode::Window;
                }
                "--launchpad" => {
                    config.show_launchpad = true;
                }
                _ => {}
            }
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_config_parses_capture_modes() {
        let full = StartupConfig::from_iter(["--full"]);
        assert!(matches!(full.capture, StartupCaptureMode::Full));

        let region = StartupConfig::from_iter(["--capture-region"]);
        assert!(matches!(region.capture, StartupCaptureMode::Region));

        let window = StartupConfig::from_iter(["--capture-window"]);
        assert!(matches!(window.capture, StartupCaptureMode::Window));
    }

    #[test]
    fn startup_config_enables_launchpad_flag() {
        let config = StartupConfig::from_iter(["--launchpad"]);
        assert!(config.show_launchpad);
    }

    #[test]
    fn startup_config_last_capture_flag_wins() {
        let config = StartupConfig::from_iter(["--full", "--region", "--window"]);
        assert!(matches!(config.capture, StartupCaptureMode::Window));
    }
}
