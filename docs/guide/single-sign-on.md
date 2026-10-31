---
title: "Single sign-on (OIDC) for the Admin UI"
description: "Let people sign in to the Admin UI with Google, Microsoft Entra ID, Okta, Keycloak, or another OpenID Connect provider."
---

# Single sign-on

Dopbase can sign people in through an OpenID Connect identity provider such as
Google, Microsoft Entra ID, Okta, Auth0, Amazon Cognito, Clerk, Supabase,
Keycloak, Authentik, Rauthy, or Zitadel.
Accounts still come from the Users page. Single sign-on only signs in people
who already have a Dopbase account; it never creates one.

## Before you start

Only the root account can configure single sign-on, and the server needs an
HTTPS public URL.

### Why the public URL must be HTTPS

The provider sends each person back to Dopbase with a one-time sign-in code in
the URL. Over plain HTTP anyone on the network path could read that code, so
Dopbase refuses to set up, test, or use single sign-on unless its public URL
starts with `https://`. The settings page explains this and keeps its fields
locked until the URL is fixed.

The one exception is the machine itself: `http://localhost`, `http://127.0.0.1`,
and `http://[::1]` are allowed so you can try single sign-on during
development.

Set the public URL in one of these ways, then restart the server:

```bash
dopbase server start --public-url https://dopbase.example.com
DOPBASE_PUBLIC_URL=https://dopbase.example.com dopbase server start
```

or `public_url = "https://dopbase.example.com"` in `server.toml`. Behind a
reverse proxy that terminates TLS, use the URL people open in their browser.

## Set it up

1. Open **Single sign-on** in the Admin UI.
2. At your provider, create a web application that uses the authorization code
   flow and has a client secret. Add the redirect URI shown on the page, for
   example `https://dopbase.example.com/api/v1/auth/sso/callback`, and allow
   the `openid`, `email`, and `profile` scopes.
3. Back in Dopbase, choose the provider and enter the issuer URL, client ID,
   and client secret. Saving reads the provider's discovery document first, so
   a wrong issuer URL fails right away.
4. Select **Test sign-in**. You sign in at the provider and return to the
   settings page. This links your root account to that identity and proves the
   settings work.
5. Select **Turn on**. The sign-in page now shows a **Continue with …** button
   above the password form.

[Sign in with Google](./sso-google) walks through these steps for Google,
including the Google Cloud console side.
[Sign in with Keycloak](./sso-keycloak) does the same with a local Keycloak in
Docker, which is a quick way to try single sign-on.

Changing the issuer URL, client ID, or client secret turns single sign-on off
until root runs another test sign-in.

| Provider           | Issuer URL                                                               |
| ------------------ | ------------------------------------------------------------------------ |
| Google             | `https://accounts.google.com`                                            |
| Microsoft Entra ID | `https://login.microsoftonline.com/<tenant-id>/v2.0`                     |
| Okta               | `https://<your-org>.okta.com`                                            |
| Auth0              | `https://<tenant>.<region>.auth0.com/` (with the trailing slash)         |
| Amazon Cognito     | `https://cognito-idp.<region>.amazonaws.com/<user-pool-id>`              |
| Clerk              | Your Frontend API URL, for example `https://clerk.example.com`           |
| Supabase           | `https://<project-ref>.supabase.co/auth/v1`                              |
| Keycloak           | `https://<host>/realms/<realm>`                                          |
| Authentik          | `https://<host>/application/o/<application-slug>/`                       |
| Other providers    | The `issuer` value in the provider's `/.well-known/openid-configuration` |

Issuers must match exactly. If the discovery document only works with or
without a trailing slash, Dopbase stores the form the provider publishes.

If your provider isn't in the list, choose **Other OpenID Connect provider**.
It works with any provider that publishes a discovery document. The list
itself comes from a file in the Dopbase source, and `CONTRIBUTING.md` explains
how to add an entry.

A few providers need one setting first:

- Auth0: create a Regular Web Application.
- Amazon Cognito: create an app client with a client secret and allow the
  `openid`, `email`, and `profile` scopes.
- Clerk: add an OAuth application in the Clerk dashboard.
- Supabase: turn on the OAuth 2.1 server, which is in beta, and switch the
  project to asymmetric JWT signing keys (RS256 or ES256). Dopbase does not
  accept HS256-signed tokens.

Firebase Authentication cannot be used: it signs people in to your own apps
but does not act as an OpenID Connect provider for other services. If your
people sign in with Google through Firebase, choose the Google preset.

## How sign-ins find an account

The first time someone signs in, Dopbase looks for an account with the same
email. It links the provider identity to that account only if the provider
marks the email as verified and the account is an active person. Root is never
linked by email; it is linked by the test sign-in. AI agents never sign in, so
they never link.

After the first link, Dopbase recognizes the person by the provider's subject
identifier and ignores the email. If the email changes at the provider, the
person still reaches the same account, and nobody else can claim it by taking
the old address.

Dopbase refuses a sign-in whose email has no account. Create the account on the
Users page first. When a provider does not verify emails, the person can sign
in with their password and link single sign-on from the Account page. Do this
before you require single sign-on, which turns that password off.

### Microsoft Entra ID

Entra does not send `email_verified`, and users in other tenants can set an
email claim to any address. Use the issuer URL that contains your tenant ID.
Dopbase links an Entra account by email only when the ID token carries the
`xms_edov` claim, which says your tenant owns the email's domain. Add
`xms_edov` as an optional claim in the app registration's token configuration,
or ask people to link from the Account page.

## Passwords

Accounts keep the password they have. While single sign-on is on, an
administrator can also create an account without a password on the Users page,
and that person signs in only through the provider. They can set a password
later on the Account page, after signing in with single sign-on.

### Require single sign-on

Turn on **Require single sign-on** to refuse password sign-in for every account
except root, in the browser and in the CLI. Root keeps its password so you can
get back in when the provider is down or misconfigured. Turning it on signs
everyone except root out, so sessions started with a password end there.
Turning single sign-on off lifts the requirement as well.

### Hide the password form

With single sign-on on, root can turn on **Hide the password form**. The
sign-in page then says the server uses that provider and shows only its
button. Nothing else changes:
passwords still work in the CLI and the API, and **Require single sign-on**
is the setting that turns them off. Root can still choose **Signing in as
root? Use your password**, or open `/login?password`, which is how you get in
when the provider is down.

### Confirming sensitive actions

Revealing or exporting secrets, managing users, and creating personal tokens ask
you to confirm it's you. An account linked to single sign-on can confirm at the
provider: Dopbase opens a small window that asks the provider for a fresh
sign-in and checks the `auth_time` it returns. If the provider reuses an older
sign-in instead of asking again, the confirmation fails. Accounts without a
password confirm only this way, and so does everyone except root while single
sign-on is required.

Dopbase sends `max_age=0`, and `prompt=login` for every preset except Google,
which rejects it.

## The CLI

The CLI never talks to the identity provider. `dopbase login --browser` prints a
code that you approve in the Admin UI, where single sign-on works. Once single
sign-on is on, plain `dopbase login` uses the browser on its own. See
[browser login](/reference/identity#browser-login).

When an account cannot confirm with a password, because it has none or single
sign-on is required, commands that reveal secrets print a link and a code to
approve in the Admin UI instead of asking for one.

## Unlink an identity

Administrators can unlink an account on the Users page, for example after the
person's identity was recreated at the provider. With a verified email, their
next sign-in links it again. People who have a password can unlink themselves
on the Account page.

## Switch to another provider

To move to a different provider, for example from Google to Keycloak, enter
the new issuer URL, client ID, and client secret and save. Identities from the
old issuer can never match a sign-in again, so saving a new issuer removes
every linked identity, root's included. It also turns single sign-on off and
lifts **Require single sign-on**. Nobody is signed out.

Run a test sign-in against the new provider and turn it back on. From then on,
each person's first sign-in works like the first one ever did: it links to
their account when the new provider reports the same email as verified, and
later sign-ins match the new subject identifier.

Until you turn it back on, accounts without a password cannot sign in. If
someone's email at the new provider differs from their Dopbase email, or isn't
verified, their sign-in is refused. Set them a password on the Users page so
they can sign in and link from the Account page.

## Turn it off or remove it

**Turn off** keeps the settings and linked identities, and everyone signs in
with a password. **Remove provider**, in its own card at the bottom of the
page, deletes the provider and every linked identity.

Accounts without a password cannot sign in while single sign-on is off or
removed. The settings page shows how many there are; set them a password on the
Users page first.

## Security details

- Sign-ins use the authorization code flow with PKCE. The `state` value is tied
  to the browser that started the sign-in, so a callback URL opened in another
  browser is refused, and the ID token's nonce must match.
- Dopbase accepts only asymmetric ID token signatures (RSA, ECDSA, and EdDSA)
  and checks the issuer, audience, and expiry. It refetches the provider's keys
  once when a token names a key it has not seen.
- Provider access and refresh tokens are discarded after the sign-in. The
  client secret is encrypted with the master key and re-encrypted when the key
  is rotated.
- Backups include the single sign-on settings and linked identities.
- One client can hold at most 20 sign-ins in progress. Behind a reverse proxy,
  list it in `trusted_proxies` so clients are told apart.
- The redirect URI comes from the public URL, never from the request's `Host`
  header.
- Settings changes, links, and failed sign-ins appear in the
  [audit log](/reference/audit-events).
