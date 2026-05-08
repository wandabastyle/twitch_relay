use rand::{Rng, distr::Alphanumeric};

pub fn generate_alphanumeric(length: usize) -> String {
    rand::rng()
        .sample_iter(Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub fn generate_session_token() -> String {
    generate_alphanumeric(48)
}

pub fn generate_qr_session_token() -> String {
    generate_alphanumeric(32)
}

pub fn generate_access_code() -> String {
    generate_alphanumeric(24)
}

pub fn generate_oauth_state() -> String {
    generate_alphanumeric(42)
}

pub fn generate_ticket() -> String {
    generate_alphanumeric(48)
}
