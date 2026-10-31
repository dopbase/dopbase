---
title: "Admin UI: groups"
description: "Give a whole team access to projects at once with groups, and add or remove members, in the Dopbase Admin UI."
---

# Groups

A group is a named set of people and AI agents. Give a group access and
everyone in it gets that access, but never more than their own role allows.
Add someone to the group and they get it too; take them out and they lose it
on their next request. Groups save you giving the same access to people one at
a time.

Open **Groups** from the sidebar. Anyone with `users.view` or `groups.manage`
can see it. The table shows each group's members, the projects it reaches, and
who created it. Creating, renaming, and deleting groups, and changing their
members or access, needs `groups.manage`, which admins have.

## Create a group

Choose **New group**, give it a name such as `Backend` and, if you like, a
description, then choose **Create group**. You land on the group's page. It
reaches nothing until you give it access.

A row's **⋯** menu has **Rename**, **Add members**, **Give access**, and
**Delete group**, so you can do most things without opening the group.

## A group's page

- **Members** lists who's in the group, their role, and who added them and
  when. Choose **Add members**, tick any number of people, and confirm. A member's **⋯** menu opens their page or takes them out of the
  group. Root never joins groups, because root already reaches everything.
- **Access** lists the group's grants. Choose **Give access** to give it a
  project or one environment, or use a row's **⋯** menu to change the level or
  remove it. The same grants show as **via** the group on each member's
  Access tab.
- **Danger zone** deletes the group.

To change a group's access, you must manage the project involved. Once root
gives a group access to a project you don't manage, only root, or a project
manager of that project, can change it.

## Delete a group

Choose **Delete group** and type its name. Its members lose the access the
group gave them. Their own grants stay.

Group changes are recorded in the audit log as `group.created`,
`group.updated`, `group.deleted`, `group_member.added`, and
`group_member.removed`.
