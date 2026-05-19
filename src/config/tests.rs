use std::sync::{
   Mutex,
   OnceLock,
};

use super::*;

// Use a lock to prevent parallel test execution for env var tests
fn env_test_lock() -> &'static Mutex<()> {
   static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
   LOCK.get_or_init(|| Mutex::new(()))
}

const TEST_VAR: &str = "TEST_BOOL_VAR";

#[test]
fn parse_bool_unset_defaults_to_none() {
   let _lock = env_test_lock().lock().expect("env test lock");

   // Store any existing value
   let previous = env::var(TEST_VAR).ok();
   unsafe { env::remove_var(TEST_VAR) };

   let result = parse_bool(TEST_VAR).unwrap();
   assert_eq!(result, None);

   // Restore
   match previous {
      Some(value) => unsafe { env::set_var(TEST_VAR, value) },
      None => unsafe { env::remove_var(TEST_VAR) },
   }
}

#[test]
fn parse_bool_truthy_values_parse_as_true() {
   let _lock = env_test_lock().lock().expect("env test lock");
   let truthy = ["true", "1", "yes", "on", "TRUE", "True", "YES", "ON"];

   for value in truthy {
      unsafe { env::set_var(TEST_VAR, value) };
      let result = parse_bool(TEST_VAR).unwrap();
      assert_eq!(result, Some(true), "expected '{value}' to parse as true");
   }

   unsafe { env::remove_var(TEST_VAR) };
}

#[test]
fn parse_bool_falsy_values_parse_as_false() {
   let _lock = env_test_lock().lock().expect("env test lock");
   let falsy = ["false", "0", "no", "off", "FALSE", "False", "NO", "OFF"];

   for value in falsy {
      unsafe { env::set_var(TEST_VAR, value) };
      let result = parse_bool(TEST_VAR).unwrap();
      assert_eq!(
         result,
         Some(false),
         "expected '{value}' to parse as false"
      );
   }

   unsafe { env::remove_var(TEST_VAR) };
}

#[test]
fn parse_bool_invalid_returns_error() {
   let _lock = env_test_lock().lock().expect("env test lock");
   let invalid = ["maybe", "invalid", "2", ""];

   for value in invalid {
      unsafe { env::set_var(TEST_VAR, value) };
      let result = parse_bool(TEST_VAR);
      assert!(result.is_err(), "expected '{value}' to return an error");
   }

   unsafe { env::remove_var(TEST_VAR) };
}

#[test]
fn startup_config_from_env_defaults_rotate_password_false() {
   let _lock = env_test_lock().lock().expect("env test lock");

   // Save any existing value
   let previous = env::var("TWITCH_RELAY_ROTATE_PASSWORD").ok();
   unsafe { env::remove_var("TWITCH_RELAY_ROTATE_PASSWORD") };

   let result = parse_bool("TWITCH_RELAY_ROTATE_PASSWORD").unwrap();
   assert_eq!(result, None);

   // Restore
   match previous {
      Some(value) => unsafe { env::set_var("TWITCH_RELAY_ROTATE_PASSWORD", value) },
      None => unsafe { env::remove_var("TWITCH_RELAY_ROTATE_PASSWORD") },
   }
}

#[test]
fn parse_bool_invalid_rotate_password_returns_config_error() {
   const VAR_NAME: &str = "TWITCH_RELAY_ROTATE_PASSWORD";
   let _lock = env_test_lock().lock().expect("env test lock");

   // Save any existing value
   let previous = env::var(VAR_NAME).ok();
   unsafe { env::set_var(VAR_NAME, "invalid_value") };

   let result = parse_bool(VAR_NAME);
   assert!(result.is_err());
   if let Err(AppError::Config(msg)) = result {
      assert!(msg.contains("TWITCH_RELAY_ROTATE_PASSWORD"));
      assert!(msg.contains("expected boolean"));
   } else {
      panic!("expected AppError::Config for invalid bool value");
   }

   // Restore
   match previous {
      Some(value) => unsafe { env::set_var(VAR_NAME, value) },
      None => unsafe { env::remove_var(VAR_NAME) },
   }
}

// ====================================================================
// Invidious config tests
// ====================================================================

#[test]
fn invidious_config_unset_returns_none() {
   let _lock = env_test_lock().lock().expect("env test lock");

   // Save existing values
   let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
   let prev_token = env::var("INVIDIOUS_TOKEN").ok();

   // Clear all Invidious env vars
   unsafe {
      env::remove_var("INVIDIOUS_BASE_URL");
      env::remove_var("INVIDIOUS_TOKEN");
   }

   let result = parse_invidious_config().unwrap();
   assert!(
      result.is_none(),
      "expected None when INVIDIOUS_BASE_URL is unset"
   );

   // Restore
   match prev_base {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
   }
   match prev_token {
      Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
      None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
   }
}

#[test]
fn invidious_config_normal_mode_requires_token() {
   let _lock = env_test_lock().lock().expect("env test lock");

   // Save existing values
   let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
   let prev_token = env::var("INVIDIOUS_TOKEN").ok();

   // Set base URL but not token
   unsafe {
      env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
      env::remove_var("INVIDIOUS_TOKEN");
   }

   let result = parse_invidious_config();
   assert!(
      result.is_err(),
      "expected error when base_url set but token missing"
   );
   if let Err(AppError::Config(msg)) = result {
      assert!(
         msg.contains("INVIDIOUS_TOKEN"),
         "error should mention INVIDIOUS_TOKEN"
      );
   } else {
      panic!("expected AppError::Config");
   }

   // Restore
   match prev_base {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
   }
   match prev_token {
      Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
      None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
   }
}

#[test]
fn invidious_config_normal_mode_with_token_ok() {
   let _lock = env_test_lock().lock().expect("env test lock");

   // Save existing values
   let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
   let prev_token = env::var("INVIDIOUS_TOKEN").ok();

   unsafe {
      env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
      env::set_var("INVIDIOUS_TOKEN", "test_token_123");
   }

   let result = parse_invidious_config().unwrap();
   assert!(result.is_some(), "expected Some config");
   let config = result.unwrap();
   assert_eq!(config.base_url, "https://invidious.example.com");
   assert_eq!(config.token, "test_token_123");
   assert!(config.sid_cookie.is_none());
   assert!(config.basic_auth_user.is_none());
   assert!(config.basic_auth_password.is_none());

   // Restore
   match prev_base {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
   }
   match prev_token {
      Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
      None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
   }
}

#[test]
fn invidious_config_invalid_url_scheme_fails() {
   let _lock = env_test_lock().lock().expect("env test lock");

   // Save existing values
   let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
   let prev_token = env::var("INVIDIOUS_TOKEN").ok();

   unsafe {
      env::set_var("INVIDIOUS_BASE_URL", "ftp://invidious.example.com");
      env::set_var("INVIDIOUS_TOKEN", "test_token_123");
   }

   let result = parse_invidious_config();
   assert!(result.is_err(), "expected error for invalid URL scheme");
   if let Err(AppError::Config(msg)) = result {
      assert!(
         msg.contains("http://") || msg.contains("https://"),
         "error should mention http/https"
      );
   } else {
      panic!("expected AppError::Config");
   }

   // Restore
   match prev_base {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
   }
   match prev_token {
      Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
      None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
   }
}

#[test]
fn invidious_config_basic_auth_requires_both_user_and_password() {
   let _lock = env_test_lock().lock().expect("env test lock");

   // Save existing values
   let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
   let prev_token = env::var("INVIDIOUS_TOKEN").ok();
   let prev_user = env::var("INVIDIOUS_BASIC_AUTH_USER").ok();
   let prev_pass = env::var("INVIDIOUS_BASIC_AUTH_PASSWORD").ok();
   let prev_sid = env::var("INVIDIOUS_SID").ok();

   // Test: user only, no password
   unsafe {
      env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
      env::set_var("INVIDIOUS_TOKEN", "test_token_123");
      env::set_var("INVIDIOUS_BASIC_AUTH_USER", "admin");
      env::remove_var("INVIDIOUS_BASIC_AUTH_PASSWORD");
      env::remove_var("INVIDIOUS_SID");
   }

   let result = parse_invidious_config();
   assert!(result.is_err(), "expected error when only user is set");
   if let Err(AppError::Config(msg)) = result {
      assert!(
         msg.contains("BASIC_AUTH_USER") && msg.contains("BASIC_AUTH_PASSWORD"),
         "error should mention both vars"
      );
   } else {
      panic!("expected AppError::Config");
   }

   // Test: password only, no user
   unsafe {
      env::remove_var("INVIDIOUS_BASIC_AUTH_USER");
      env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", "secret");
   }

   let result = parse_invidious_config();
   assert!(result.is_err(), "expected error when only password is set");
   if let Err(AppError::Config(msg)) = result {
      assert!(
         msg.contains("BASIC_AUTH_USER") && msg.contains("BASIC_AUTH_PASSWORD"),
         "error should mention both vars"
      );
   } else {
      panic!("expected AppError::Config");
   }

   // Restore
   match prev_base {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
   }
   match prev_token {
      Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
      None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
   }
   match prev_user {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_USER", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_USER") },
   }
   match prev_pass {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_PASSWORD") },
   }
   match prev_sid {
      Some(v) => unsafe { env::set_var("INVIDIOUS_SID", v) },
      None => unsafe { env::remove_var("INVIDIOUS_SID") },
   }
}

#[test]
fn invidious_config_basic_auth_requires_sid() {
   let _lock = env_test_lock().lock().expect("env test lock");

   // Save existing values
   let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
   let prev_token = env::var("INVIDIOUS_TOKEN").ok();
   let prev_user = env::var("INVIDIOUS_BASIC_AUTH_USER").ok();
   let prev_pass = env::var("INVIDIOUS_BASIC_AUTH_PASSWORD").ok();
   let prev_sid = env::var("INVIDIOUS_SID").ok();

   // Set Basic auth but no SID
   unsafe {
      env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
      env::set_var("INVIDIOUS_TOKEN", "test_token_123");
      env::set_var("INVIDIOUS_BASIC_AUTH_USER", "admin");
      env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", "secret");
      env::remove_var("INVIDIOUS_SID");
   }

   let result = parse_invidious_config();
   assert!(
      result.is_err(),
      "expected error when Basic auth set but SID missing"
   );
   if let Err(AppError::Config(msg)) = result {
      assert!(
         msg.contains("INVIDIOUS_SID"),
         "error should mention INVIDIOUS_SID"
      );
   } else {
      panic!("expected AppError::Config");
   }

   // Restore
   match prev_base {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
   }
   match prev_token {
      Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
      None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
   }
   match prev_user {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_USER", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_USER") },
   }
   match prev_pass {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_PASSWORD") },
   }
   match prev_sid {
      Some(v) => unsafe { env::set_var("INVIDIOUS_SID", v) },
      None => unsafe { env::remove_var("INVIDIOUS_SID") },
   }
}

#[test]
fn invidious_config_reverse_proxy_mode_ok() {
   let _lock = env_test_lock().lock().expect("env test lock");

   // Save existing values
   let prev_base = env::var("INVIDIOUS_BASE_URL").ok();
   let prev_token = env::var("INVIDIOUS_TOKEN").ok();
   let prev_user = env::var("INVIDIOUS_BASIC_AUTH_USER").ok();
   let prev_pass = env::var("INVIDIOUS_BASIC_AUTH_PASSWORD").ok();
   let prev_sid = env::var("INVIDIOUS_SID").ok();

   // Set all reverse-proxy mode vars, no token needed
   unsafe {
      env::set_var("INVIDIOUS_BASE_URL", "https://invidious.example.com");
      env::remove_var("INVIDIOUS_TOKEN");
      env::set_var("INVIDIOUS_BASIC_AUTH_USER", "admin");
      env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", "secret");
      env::set_var("INVIDIOUS_SID", "sid_session_123");
   }

   let result = parse_invidious_config().unwrap();
   assert!(result.is_some(), "expected Some config");
   let config = result.unwrap();
   assert_eq!(config.base_url, "https://invidious.example.com");
   assert_eq!(config.token, ""); // Token is optional in reverse-proxy mode
   assert_eq!(config.sid_cookie, Some("sid_session_123".to_string()));
   assert_eq!(config.basic_auth_user, Some("admin".to_string()));
   assert_eq!(config.basic_auth_password, Some("secret".to_string()));

   // Restore
   match prev_base {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASE_URL", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASE_URL") },
   }
   match prev_token {
      Some(v) => unsafe { env::set_var("INVIDIOUS_TOKEN", v) },
      None => unsafe { env::remove_var("INVIDIOUS_TOKEN") },
   }
   match prev_user {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_USER", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_USER") },
   }
   match prev_pass {
      Some(v) => unsafe { env::set_var("INVIDIOUS_BASIC_AUTH_PASSWORD", v) },
      None => unsafe { env::remove_var("INVIDIOUS_BASIC_AUTH_PASSWORD") },
   }
   match prev_sid {
      Some(v) => unsafe { env::set_var("INVIDIOUS_SID", v) },
      None => unsafe { env::remove_var("INVIDIOUS_SID") },
   }
}

// ====================================================================
// Stream resolver mode tests
// ====================================================================

#[test]
fn stream_resolver_mode_default_is_auto() {
   assert_eq!(StreamResolverMode::default(), StreamResolverMode::Auto);
}

#[test]
fn stream_resolver_mode_from_str_valid_values() {
   let test_cases = [
      ("auto", StreamResolverMode::Auto),
      ("native", StreamResolverMode::Native),
      ("streamlink", StreamResolverMode::Streamlink),
      ("AUTO", StreamResolverMode::Auto),
      ("Native", StreamResolverMode::Native),
      ("StreamLink", StreamResolverMode::Streamlink),
      ("  auto  ", StreamResolverMode::Auto), // with whitespace
   ];

   for (input, expected) in test_cases {
      let result: Result<StreamResolverMode, _> = input.parse();
      assert!(result.is_ok(), "expected '{input}' to parse successfully");
      assert_eq!(
         result.unwrap(),
         expected,
         "expected '{input}' to parse to {expected:?}"
      );
   }
}

#[test]
fn stream_resolver_mode_from_str_invalid_values() {
   let invalid = ["invalid", "unknown", "auto2", "", "cdn"];

   for input in invalid {
      let result: Result<StreamResolverMode, _> = input.parse();
      assert!(result.is_err(), "expected '{input}' to return an error");
      let err_msg = result.unwrap_err();
      assert!(
         err_msg.contains("STREAM_RESOLVER_MODE"),
         "error should mention STREAM_RESOLVER_MODE: {err_msg}"
      );
   }
}

#[test]
fn stream_resolver_mode_parse_enum_unset_defaults_to_none() {
   let _lock = env_test_lock().lock().expect("env test lock");

   let previous = env::var("STREAM_RESOLVER_MODE").ok();
   unsafe { env::remove_var("STREAM_RESOLVER_MODE") };

   let result: Result<Option<StreamResolverMode>, AppError> = parse_enum("STREAM_RESOLVER_MODE");
   assert_eq!(result.unwrap(), None);

   // Restore
   match previous {
      Some(value) => unsafe { env::set_var("STREAM_RESOLVER_MODE", value) },
      None => unsafe { env::remove_var("STREAM_RESOLVER_MODE") },
   }
}

#[test]
fn stream_resolver_mode_parse_enum_valid_value() {
   let _lock = env_test_lock().lock().expect("env test lock");

   let previous = env::var("STREAM_RESOLVER_MODE").ok();
   unsafe { env::set_var("STREAM_RESOLVER_MODE", "streamlink") };

   let result: Result<Option<StreamResolverMode>, AppError> = parse_enum("STREAM_RESOLVER_MODE");
   assert_eq!(result.unwrap(), Some(StreamResolverMode::Streamlink));

   // Restore
   match previous {
      Some(value) => unsafe { env::set_var("STREAM_RESOLVER_MODE", value) },
      None => unsafe { env::remove_var("STREAM_RESOLVER_MODE") },
   }
}

#[test]
fn stream_resolver_mode_parse_enum_invalid_returns_error() {
   let _lock = env_test_lock().lock().expect("env test lock");

   let previous = env::var("STREAM_RESOLVER_MODE").ok();
   unsafe { env::set_var("STREAM_RESOLVER_MODE", "invalid_mode") };

   let result: Result<Option<StreamResolverMode>, AppError> = parse_enum("STREAM_RESOLVER_MODE");
   assert!(result.is_err(), "expected invalid value to return an error");
   if let Err(AppError::Config(msg)) = result {
      assert!(
         msg.contains("STREAM_RESOLVER_MODE"),
         "error should mention STREAM_RESOLVER_MODE"
      );
   } else {
      panic!("expected AppError::Config for invalid enum value");
   }

   // Restore
   match previous {
      Some(value) => unsafe { env::set_var("STREAM_RESOLVER_MODE", value) },
      None => unsafe { env::remove_var("STREAM_RESOLVER_MODE") },
   }
}

// ====================================================================
// Stream delivery mode tests
// ====================================================================

#[test]
fn stream_delivery_mode_default_is_cdn_first() {
   assert_eq!(StreamDeliveryMode::default(), StreamDeliveryMode::CdnFirst);
}

#[test]
fn stream_delivery_mode_from_str_valid_values() {
   let test_cases = [
      ("cdn_first", StreamDeliveryMode::CdnFirst),
      ("relay", StreamDeliveryMode::Relay),
      ("CDN_FIRST", StreamDeliveryMode::CdnFirst),
      ("Relay", StreamDeliveryMode::Relay),
      ("  cdn_first  ", StreamDeliveryMode::CdnFirst), // with whitespace
   ];

   for (input, expected) in test_cases {
      let result: Result<StreamDeliveryMode, _> = input.parse();
      assert!(result.is_ok(), "expected '{input}' to parse successfully");
      assert_eq!(
         result.unwrap(),
         expected,
         "expected '{input}' to parse to {expected:?}"
      );
   }
}

#[test]
fn stream_delivery_mode_from_str_invalid_values() {
   let invalid = ["invalid", "unknown", "cdn", "first", ""];

   for input in invalid {
      let result: Result<StreamDeliveryMode, _> = input.parse();
      assert!(result.is_err(), "expected '{input}' to return an error");
      let err_msg = result.unwrap_err();
      assert!(
         err_msg.contains("STREAM_DELIVERY_MODE"),
         "error should mention STREAM_DELIVERY_MODE: {err_msg}"
      );
   }
}

#[test]
fn stream_delivery_mode_parse_enum_unset_defaults_to_none() {
   let _lock = env_test_lock().lock().expect("env test lock");

   let previous = env::var("STREAM_DELIVERY_MODE").ok();
   unsafe { env::remove_var("STREAM_DELIVERY_MODE") };

   let result: Result<Option<StreamDeliveryMode>, AppError> = parse_enum("STREAM_DELIVERY_MODE");
   assert_eq!(result.unwrap(), None);

   // Restore
   match previous {
      Some(value) => unsafe { env::set_var("STREAM_DELIVERY_MODE", value) },
      None => unsafe { env::remove_var("STREAM_DELIVERY_MODE") },
   }
}

#[test]
fn stream_delivery_mode_parse_enum_valid_value() {
   let _lock = env_test_lock().lock().expect("env test lock");

   let previous = env::var("STREAM_DELIVERY_MODE").ok();
   unsafe { env::set_var("STREAM_DELIVERY_MODE", "relay") };

   let result: Result<Option<StreamDeliveryMode>, AppError> = parse_enum("STREAM_DELIVERY_MODE");
   assert_eq!(result.unwrap(), Some(StreamDeliveryMode::Relay));

   // Restore
   match previous {
      Some(value) => unsafe { env::set_var("STREAM_DELIVERY_MODE", value) },
      None => unsafe { env::remove_var("STREAM_DELIVERY_MODE") },
   }
}

#[test]
fn stream_delivery_mode_parse_enum_invalid_returns_error() {
   let _lock = env_test_lock().lock().expect("env test lock");

   let previous = env::var("STREAM_DELIVERY_MODE").ok();
   unsafe { env::set_var("STREAM_DELIVERY_MODE", "invalid_mode") };

   let result: Result<Option<StreamDeliveryMode>, AppError> = parse_enum("STREAM_DELIVERY_MODE");
   assert!(result.is_err(), "expected invalid value to return an error");
   if let Err(AppError::Config(msg)) = result {
      assert!(
         msg.contains("STREAM_DELIVERY_MODE"),
         "error should mention STREAM_DELIVERY_MODE"
      );
   } else {
      panic!("expected AppError::Config for invalid enum value");
   }

   // Restore
   match previous {
      Some(value) => unsafe { env::set_var("STREAM_DELIVERY_MODE", value) },
      None => unsafe { env::remove_var("STREAM_DELIVERY_MODE") },
   }
}
