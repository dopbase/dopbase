---
title: "Admin UI: groups"
description: "Give a whole team access to projects at once with groups, and add or remove members, in the Dopbase Admin UI."
---

# Groups

A group is a named set of people and AI agents. Give a group access on a
project's [Access](./project-access) page and everyone in it gets that access.
Add someone to the group and they get it too; remove them and they lose it on
their next request. Groups save you from giving the same access to people one
at a time.

Open **Groups** from the sidebar. Anyone with `users.view` or `groups.manage`
can see it.

## Create a group

Choose **New group**, give it a name such as `Backend team` and, if you like, a
description. Creating, renaming, and deleting groups needs `groups.manage`,
which admins have.

## Members

Open a group to see its members and the access it gives. Pick someone under
**Add a member** to add them, or choose **Remove** beside them. Root never
joins groups, because root already reaches everything.

To change a group, you must manage every project it has access to. Once root
gives a group access to a project you don't manage, only root, or a project
manager of every project involved, can change it.

## Delete a group

Choose **Delete group** and type its name. Its members lose the access the
group gave them. Their own grants stay.

Group changes are recorded in the audit log as `group.created`,
`group.updated`, `group.deleted`, `group_member.added`, and
`group_member.removed`.
