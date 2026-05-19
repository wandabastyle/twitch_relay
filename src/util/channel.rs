pub fn normalize_channel_login(input: &str) -> Result<String, String> {
   let normalized = input.trim().to_ascii_lowercase();
   if normalized.is_empty() {
      return Err("channel login cannot be empty".to_string());
   }
   Ok(normalized)
}
