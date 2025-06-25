pub mod api;
pub mod config;
pub mod init;
pub mod repl;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_config_roundtrip_serialization() {
        let original_config = config::Config {
            api_key: "sk-test-key-12345".to_string(),
            default_model: "google/gemini-2.5-flash".to_string(),
        };
        
        // Test serialization -> deserialization preserves data integrity
        let serialized = toml::to_string(&original_config).expect("Failed to serialize config");
        let deserialized: config::Config = toml::from_str(&serialized).expect("Failed to deserialize config");
        
        assert_eq!(original_config.api_key, deserialized.api_key);
        assert_eq!(original_config.default_model, deserialized.default_model);
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.toml");
        
        let config = config::Config {
            api_key: "test-key".to_string(),
            default_model: "openai/gpt-4o-mini".to_string(),
        };
        
        // Test save and load operations
        let toml_content = toml::to_string_pretty(&config).expect("Failed to serialize");
        fs::write(&config_path, &toml_content).expect("Failed to write config");
        
        let loaded_content = fs::read_to_string(&config_path).expect("Failed to read config");
        let loaded_config: config::Config = toml::from_str(&loaded_content).expect("Failed to parse config");
        
        assert_eq!(config.api_key, loaded_config.api_key);
        assert_eq!(config.default_model, loaded_config.default_model);
    }

    #[test]
    fn test_available_models_validation() {
        use crate::config::AVAILABLE_MODELS;
        
        // Ensure we have expected models available
        assert!(!AVAILABLE_MODELS.is_empty(), "No models available");
        
        // Test that common models are present
        let required_models = [
            "google/gemini-2.5-flash",
            "openai/gpt-4o-mini", 
            "anthropic/claude-3-5-sonnet"
        ];
        
        for model in &required_models {
            assert!(
                AVAILABLE_MODELS.contains(model), 
                "Required model '{}' not found in AVAILABLE_MODELS", 
                model
            );
        }
        
        // Validate model format (should contain provider/model pattern)
        for model in AVAILABLE_MODELS {
            assert!(
                model.contains('/'), 
                "Model '{}' doesn't follow provider/model format", 
                model
            );
        }
    }
}