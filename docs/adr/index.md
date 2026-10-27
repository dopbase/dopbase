---
title: Architecture decision records
description: Recorded decisions for the an1by/dopbase fork that diverge from upstream dopbase/dopbase.
---

# Architecture decision records

This directory holds **ADR**s for [an1by/dopbase](https://github.com/an1by/dopbase):
a long-lived fork of [dopbase/dopbase](https://github.com/dopbase/dopbase). Upstream
owns the public product contract; these documents explain what we add, how we merge
upstream releases, and what we intentionally do not ship.

| ADR | Title | Status |
| --- | ----- | ------ |
| [0001](./0001-an1by-fork.md) | Maintain the an1by fork alongside upstream | Accepted |

New fork-only behavior should get an ADR (or an update to 0001) when it affects
migrations, CLI contracts, deployment, or compatibility with upstream binaries.
