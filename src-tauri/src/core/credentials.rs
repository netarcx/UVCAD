/// Default OAuth credentials, embedded at compile time.
///
/// Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET environment variables
/// before building. For development, create a .env file and source it.
///
/// These are Google "Desktop" type OAuth credentials. Google's documentation
/// acknowledges that client_secret for desktop apps is not truly secret.

pub fn default_client_id() -> &'static str {
    env!("GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_ID env var must be set at build time")
}

pub fn default_client_secret() -> &'static str {
    env!("GOOGLE_CLIENT_SECRET", "GOOGLE_CLIENT_SECRET env var must be set at build time")
}
