-- Add currency preference to tenants
ALTER TABLE tenants ADD COLUMN currency TEXT NOT NULL DEFAULT 'INR';
