-- Add per-tenant locale (BCP-47 tag, e.g. "en-IN", "en-US"). Drives
-- number/currency grouping in the UI. Currency stays as a separate column
-- so a tenant can read English-IN copy with USD prices if they want.
ALTER TABLE tenants ADD COLUMN locale TEXT NOT NULL DEFAULT 'en-IN';
