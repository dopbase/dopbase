---
title: "Users and AI agents"
description: "Add teammates, choose their roles, and manage read-only AI accounts in the Admin UI."
---

# Users and AI agents

Root and admin can open **Users** to manage who uses the instance. Add accounts
from the Admin UI; there is no CLI command for creating users.

## Choose a role

- **Root** is the account created during setup. It has full access, including
  full-instance backups and factory reset. Other users cannot change or delete it.
- **Admin** manages projects and other users, including other admins.
- **Member** manages projects, environments, secrets, and runner tokens, but
  cannot manage users.
- **AI agent** can read project and environment details and secret names, but
  cannot read secret values or make changes.

All roles can see every project. Assigning users to individual projects and
customizing permissions are not available yet.

## Add a teammate

1. Open **Users** and choose **Add User**.
2. Enter the teammate's email and a password.
3. Choose **Member** or **Admin**.
4. Save the account and share the sign-in details securely.

Members can work with projects immediately. Choose Admin only when the person
also needs to manage accounts. Dopbase does not send invitation emails.

## Edit or remove a user

Choose **Edit** to change a user's email, role, or password. Leave the password
field empty to keep the existing password. After an update, the user needs to
sign in again.

Choose **Delete** and type the user's email to confirm before removing an account.
You cannot delete your own account or change your own role. Use
[Account settings](/ui/setup-and-sign-in#your-account) to change your own password.

## AI agents

Choose **Add AI agent** and give it a recognizable name. AI accounts use access
tokens instead of an email and password.

Choose **Get token** beside an agent to generate a credential. This prompts for
your administrator password to confirm your identity. Each confirmation issues
a fresh 30-day token and revokes any prior active token for that agent. The
plaintext token is shown once: copy and store it immediately, as it cannot be
recovered later.

AI access is read-only and does not include secret values. Secret names and
project details can still be sensitive, so create accounts only for agents you
trust.

Deleting an AI account also removes its access tokens. Choose **Delete** and
type the agent's name to confirm.

See [identity and tokens](/reference/identity) for the permissions of each role.
