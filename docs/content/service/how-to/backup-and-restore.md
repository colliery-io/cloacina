---
title: "Back Up and Restore"
description: "Back up and restore the Cloacina database — PostgreSQL (per-tenant or full) and SQLite."
weight: 35
---

# Back Up and Restore

Cloacina has no built-in backup command — it stores all state in your database, so
you back up and restore with the standard tooling for that backend. This guide
gives the concrete commands for both.

{{< hint type=warning >}}
Take backups against a quiescent or point-in-time-consistent source, and test a
restore before you rely on it. Include the bootstrap/admin key material in your
secret backups separately — it lives in `~/.cloacina/`, not the database.
{{< /hint >}}

## PostgreSQL

### Full database

```bash
# Back up everything (all tenant schemas)
pg_dump -h "$PGHOST" -d cloacina -Fc -f cloacina.dump

# Restore into a fresh database
pg_restore -h "$PGHOST" -d cloacina --clean --if-exists cloacina.dump
```

### A single tenant

Each tenant is an isolated PostgreSQL **schema**, so you can back up and restore one
tenant independently:

```bash
# Back up one tenant's schema
pg_dump -h "$PGHOST" -d cloacina --schema=tenant_acme -f tenant_acme.sql

# Restore that tenant
psql -h "$PGHOST" -d cloacina -f tenant_acme.sql
```

See [Multi-Tenancy]({{< ref "/service/explanation/multi-tenancy" >}}) for how
schemas map to tenants.

## SQLite

SQLite stores everything in a single file. Use the `sqlite3` online-backup command
(safe while the server is running) rather than copying the file underneath a live
process:

```bash
# Consistent online backup
sqlite3 /var/lib/cloacina/cloacina.db ".backup '/backups/cloacina-$(date +%F).db'"

# Equivalent using VACUUM INTO
sqlite3 /var/lib/cloacina/cloacina.db "VACUUM INTO '/backups/cloacina.db'"
```

To restore, stop the server, put the backup file back in place, and start it again:

```bash
# (server stopped)
cp /backups/cloacina-2026-06-15.db /var/lib/cloacina/cloacina.db
# (start the server — it re-runs migrations on connect if needed)
```

## See also

- [Multi-Tenant Recovery]({{< ref "/service/how-to/multi-tenant-recovery" >}}) — recovering interrupted work after a failure.
- [Production Deployment]({{< ref "/service/how-to/production-deployment" >}}) — where backups fit in the production checklist.
- [Database Backends]({{< ref "/service/explanation/database-backends" >}}) — PostgreSQL vs SQLite trade-offs.
