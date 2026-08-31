---
title: "Self-hosted or Cloud"
description: "Compare self-hosting Dopbase with Dopbase Cloud: who runs the server, TLS, backups, and master-key infrastructure in each model."
---

# Self-hosted or Cloud

Both deployments use the same client concepts. The difference is who operates the server.

| Responsibility            | Self-hosted                     | Dopbase Cloud                   |
| ------------------------- | ------------------------------- | ------------------------------- |
| Server process            | You                             | Dopbase                         |
| Network and TLS           | You                             | Managed                         |
| Database operations       | You                             | Managed                         |
| Master-key infrastructure | You                             | Managed                         |
| Backups and recovery      | You                             | Managed                         |
| Updates and monitoring    | You                             | Managed                         |
| CLI workflow              | Same Dopbase client             | Same Dopbase client             |
| Project model             | Projects, environments, secrets | Projects, environments, secrets |

## Choose self-hosting when

You need direct control over deployment, data location, networking, and key infrastructure, and you are prepared to own backup, recovery, monitoring, and updates.

## Choose Cloud when

You prefer a managed server endpoint and do not want your team to operate secrets infrastructure.

Cloud capabilities, availability, regions, and service terms are not final. Revisit this page when the service is announced.
