use std::env;

/// Daemon configuration loaded from environment variables with GSDUI_ prefix.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Address to listen on (default: "0.0.0.0:3000")
    pub listen_addr: String,
    /// SQLite database URL (default: "sqlite:data/gsdui.db?mode=rwc")
    pub database_url: String,
    /// Directory for static files (default: "static")
    pub static_dir: String,
}

impl DaemonConfig {
    /// Load configuration from environment variables with defaults.
    ///
    /// Environment variables:
    /// - GSDUI_LISTEN_ADDR (default: "0.0.0.0:3000")
    /// - GSDUI_DATABASE_URL (default: "sqlite:data/gsdui.db?mode=rwc")
    /// - GSDUI_STATIC_DIR (default: "static")
    pub fn from_env() -> Self {
        Self {
            listen_addr: env::var("GSDUI_LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            database_url: env::var("GSDUI_DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/gsdui.db?mode=rwc".to_string()),
            static_dir: env::var("GSDUI_STATIC_DIR").unwrap_or_else(|_| "static".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        // Clear any env vars that might interfere
        // SAFETY: Tests run single-threaded with cargo test -- --test-threads=1
        // or we accept the race since these are unique env var names.
        unsafe {
            env::remove_var("GSDUI_LISTEN_ADDR");
            env::remove_var("GSDUI_DATABASE_URL");
            env::remove_var("GSDUI_STATIC_DIR");
        }

        let config = DaemonConfig::from_env();
        assert_eq!(config.listen_addr, "0.0.0.0:3000");
        assert_eq!(config.database_url, "sqlite:data/gsdui.db?mode=rwc");
        assert_eq!(config.static_dir, "static");
    }

    #[test]
    fn test_custom_config_from_env() {
        // SAFETY: Tests run single-threaded with cargo test -- --test-threads=1
        // or we accept the race since these are unique env var names.
        unsafe {
            env::set_var("GSDUI_LISTEN_ADDR", "127.0.0.1:8080");
            env::set_var("GSDUI_DATABASE_URL", "sqlite:custom.db");
            env::set_var("GSDUI_STATIC_DIR", "/var/www/static");
        }

        let config = DaemonConfig::from_env();
        assert_eq!(config.listen_addr, "127.0.0.1:8080");
        assert_eq!(config.database_url, "sqlite:custom.db");
        assert_eq!(config.static_dir, "/var/www/static");

        // Clean up
        unsafe {
            env::remove_var("GSDUI_LISTEN_ADDR");
            env::remove_var("GSDUI_DATABASE_URL");
            env::remove_var("GSDUI_STATIC_DIR");
        }
    }
}
