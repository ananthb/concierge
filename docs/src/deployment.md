# Deployment

## Prerequisites

- [Cloudflare account](https://dash.cloudflare.com/sign-up)
- [Nix](https://nixos.org/) with flakes enabled (recommended)
- Or: Rust toolchain with `wasm32-unknown-unknown` target

## Setup

### 1. Clone and enter dev environment

```bash
git clone https://github.com/ananthb/concierge-worker.git
cd concierge-worker

# Using Nix (recommended)
nix develop

# Or using direnv
direnv allow
```

### 2. Create Cloudflare resources

```bash
# Create D1 database
wrangler d1 create concierge-worker

# Create KV namespace
wrangler kv namespace create concierge-worker

# Create R2 bucket (for file uploads)
wrangler r2 bucket create concierge-worker-uploads
```

### 3. Update wrangler.toml

Replace the placeholder IDs with the values from the commands above:

```toml
[[d1_databases]]
binding = "DB"
database_name = "concierge-worker"
database_id = "YOUR_DATABASE_ID"  # from wrangler d1 create

[[kv_namespaces]]
binding = "KV"
id = "YOUR_KV_ID"  # from wrangler kv namespace create

[[r2_buckets]]
binding = "UPLOADS"
bucket_name = "concierge-worker-uploads"
```

### 4. Run database migrations

```bash
wrangler d1 migrations apply concierge-worker
```

### 5. Deploy

**Option A: Deploy from local machine**

```bash
wrangler deploy
```

**Option B: Deploy via GitHub Actions**

1. Go to your GitHub repo Settings > Secrets and variables > Actions
2. Add a secret named `CLOUDFLARE_API_TOKEN` with a [Cloudflare API token](https://developers.cloudflare.com/fundamentals/api/get-started/create-token/)
3. Push to `main` branch to trigger automatic deployment

## Cloudflare Access Setup

The admin dashboard requires Cloudflare Access for authentication.

1. Go to [Cloudflare Zero Trust](https://one.dash.cloudflare.com/)
2. Navigate to **Access** > **Applications**
3. Click **Add an application** > **Self-hosted**
4. Configure:
   - **Application name**: Concierge Worker Admin
   - **Session duration**: 24 hours (or your preference)
   - **Application domain**: `your-worker.your-subdomain.workers.dev`
   - **Path**: `/admin`
5. Add a policy to control who can access (e.g., email ends with `@yourdomain.com`)
6. Save

## Local Development

```bash
# Run locally with simulated bindings
wrangler dev

# Run locally with real D1/KV/R2 (requires account)
wrangler dev --remote
```

For local development, set `ENVIRONMENT = "development"` in `wrangler.toml` to bypass Access authentication.

## Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `ENVIRONMENT` | Set to `development` to bypass auth | No |

See [Configuration](./configuration.md) for secrets setup.
