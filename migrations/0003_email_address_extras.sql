-- Switch email-address billing from "5-pack" model to "per extra address".
-- Old column counted packs purchased (each pack = 5 addresses, ₹49 / $0.50).
-- New column counts individual extras purchased (each = 1 address, ₹99 / $1).
ALTER TABLE tenants RENAME COLUMN email_address_packs_purchased TO email_address_extras_purchased;
