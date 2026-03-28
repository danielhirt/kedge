---
inclusion: manual
---

# Deployment Procedures

## Environments

| Environment | Purpose                  | Auto-deploy |
|-------------|--------------------------|-------------|
| dev         | Feature development       | Yes (on push to feature branch) |
| staging     | Pre-release validation    | Yes (on merge to main) |
| production  | Live traffic              | No (manual approval required) |

## Release Process

1. Merge feature branch to `main` via reviewed PR
2. CI builds and publishes Docker images tagged with git SHA
3. Staging auto-deploys; smoke tests run automatically
4. Production deploy requires:
   - Green staging smoke tests
   - Approval from on-call engineer
   - Deploy window: Tuesday-Thursday, 10:00-16:00 UTC

## Database Migrations

- Flyway manages all schema migrations
- Migrations must be backwards-compatible (no column drops in the same release)
- Large data migrations run as separate background jobs, not in Flyway

## Rollback

- Canary deployment with 5% traffic for 30 minutes before full rollout
- Automatic rollback if error rate exceeds 0.1% or p99 latency > 500ms
- Manual rollback via `deploy rollback --env production --to <sha>`
