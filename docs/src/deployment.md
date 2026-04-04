# Deployment

## CI/CD

- **Garnix** runs `nix flake check` on every push and PR (tests, clippy, formatting)
- **GitHub Actions** deploys to Cloudflare Workers on push to `main`
- **GitHub Actions** builds and deploys documentation to GitHub Pages on changes to `docs/`

## Manual Deploy

```bash
nix develop
wrangler deploy
```

## Database Migration

After deploying schema changes:

```bash
wrangler d1 execute concierge-worker --remote --file migrations/0001_create_schema.sql
```

To reset the database (drops all data):

```bash
wrangler d1 execute concierge-worker --remote --command "DROP TABLE IF EXISTS tenants; DROP TABLE IF EXISTS whatsapp_messages; DROP TABLE IF EXISTS lead_form_submissions; DROP TABLE IF EXISTS instagram_messages;"
wrangler d1 execute concierge-worker --remote --file migrations/0001_create_schema.sql
```

## Pre-commit Hooks

The Nix dev shell installs git hooks via `cachix/git-hooks.nix`:

- `rustfmt` — code formatting
- `nixpkgs-fmt` — Nix formatting
- `check-toml`, `check-yaml`, `check-json` — config validation
- `detect-private-keys` — prevents accidental key commits
- `end-of-file-fixer`, `trim-trailing-whitespace` — file hygiene
