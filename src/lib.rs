pub mod api;
pub mod config;
pub mod init;
pub mod repl;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn test_config_constants() {
        use crate::config::AVAILABLE_MODELS;
        assert!(!AVAILABLE_MODELS.is_empty());
        assert!(AVAILABLE_MODELS.contains(&"google/gemini-2.5-flash"));
    }
}