#[derive(Debug, Clone)]
pub struct LiteLLMConfig {
    pub api_key: String,
    pub url: String,
}

impl Default for LiteLLMConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            url: "https://litellm.example.in".to_string(),
        }
    }
}