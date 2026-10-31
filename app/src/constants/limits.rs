pub const BROWSER_SESSION_ABSOLUTE_HOURS: i64 = 24;
pub const BROWSER_SESSION_IDLE_HOURS: i64 = 8;
pub const CLI_SESSION_ABSOLUTE_DAYS: i64 = 90;
pub const CLI_SESSION_IDLE_DAYS: i64 = 30;
pub const RECENT_AUTHENTICATION_MINUTES: i64 = 10;
/// How long a browser CLI login stays open for approval.
pub const CLI_DEVICE_CODE_MINUTES: i64 = 10;
/// The slowest the CLI may poll; faster polls get `slowDown`.
pub const CLI_DEVICE_POLL_INTERVAL_SECONDS: i64 = 5;
/// Cap on unexpired pending requests, so the anonymous endpoint cannot
/// grow the table without bound.
pub const MAX_PENDING_CLI_DEVICE_REQUESTS: i64 = 1_000;
/// Per client address, so one client cannot use up the global cap. Behind
/// a proxy that is not in `trusted_proxies`, every client shares one.
pub const MAX_PENDING_CLI_DEVICE_REQUESTS_PER_CLIENT: i64 = 20;
/// How long a sign-in may stay at the identity provider.
pub const SSO_FLOW_MINUTES: i64 = 10;
/// How long the Admin UI has to exchange a finished sign-in for a session.
pub const SSO_HANDOFF_SECONDS: i64 = 60;
pub const MAX_PENDING_SSO_FLOWS: i64 = 1_000;
pub const MAX_PENDING_SSO_FLOWS_PER_CLIENT: i64 = 20;

pub const MAX_SECRETS_PER_ENVIRONMENT: usize = 1_000;
/// How many environments one comparison reads against its base.
pub const MAX_COMPARE_ENVIRONMENTS: usize = 3;
pub const MAX_SECRET_COLLECTION_BYTES: usize = 2 * 1024 * 1024;
/// Upper bound for a stored `.env` editor layout (comments, ordering, and
/// empty `KEY=` slots — never secret values).
pub const MAX_ENV_LAYOUT_BYTES: usize = 64 * 1024;
