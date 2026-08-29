pub const BROWSER_SESSION_ABSOLUTE_HOURS: i64 = 24;
pub const BROWSER_SESSION_IDLE_HOURS: i64 = 8;
pub const CLI_SESSION_ABSOLUTE_DAYS: i64 = 90;
pub const CLI_SESSION_IDLE_DAYS: i64 = 30;
pub const RECENT_AUTHENTICATION_MINUTES: i64 = 10;

pub const MAX_SECRETS_PER_ENVIRONMENT: usize = 1_000;
pub const MAX_SECRET_COLLECTION_BYTES: usize = 2 * 1024 * 1024;
/// Upper bound for a stored `.env` editor layout (comments, ordering, and
/// empty `KEY=` slots — never secret values).
pub const MAX_ENV_LAYOUT_BYTES: usize = 64 * 1024;
