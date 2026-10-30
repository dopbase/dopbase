---
title: "Admin UI: project access"
description: "Decide who reaches a project and its environments, at which level, and protect production in the Dopbase Admin UI."
---

# Project access

Each project has an **Access** page that lists who reaches it and lets its
project managers change that. Open it from the shield beside the project in the
project rail. Project managers see and change it; people with `access.report`,
such as auditors, can read it.

## Environments

The first section lists the project's environments with a **Protected** switch.
A protected environment ignores grants on the whole project below project
manager: only a grant on the environment itself, or a project manager, reaches
it. Protect production so that write on the project doesn't mean write on
production. Turning protection on or off needs manage on the environment.

## Who has access

The second section lists every grant on the project and on each of its
environments, for users and for [groups](./groups). Root and people who reach
every project through an all-projects grant aren't listed here.

To give access, choose **Give access** and pick:

- **who**: a user or a group;
- **where**: the whole project, or one environment;
- the **level**:

| Level           | Allows                                                                  |
| --------------- | ----------------------------------------------------------------------- |
| No access       | Environment only. Hides it, even from a project grant                   |
| Names only      | Secret names, versions, history, and the `.env` layout. No values       |
| Read            | Also reveal, export, and run with values                                |
| Write           | Also change, import, and roll back secrets                              |
| Manage          | Also rename, protect, or delete the environment and issue runner tokens |
| Project manager | Whole project only. Everything, including this page                     |

Giving someone access where they already have a grant replaces it. A grant on
one environment overrides the grant on the whole project, so you can give a
team write on the project and names only on production.

A person can't get more than their role allows: a member stops at manage, an
AI agent at read. You can't change your own access or give anyone access to
root. Choose **Remove** to take a grant away; it takes effect on the person's
next request, personal tokens included.

Every change is recorded in the [audit log](/reference/audit-events) as
`grant.set`, `grant.removed`, `environment.protected`, or
`environment.unprotected`.

See [identity and access](/reference/identity#access-to-projects-and-environments)
for how grants combine.
