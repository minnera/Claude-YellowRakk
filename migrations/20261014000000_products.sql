-- A product is "the thing" you care about; listings are where it is sold.
-- One product can have many listings across shops (the original link,
-- plus look-alikes found by search and confirmed by you or by vision).

CREATE TABLE products (
    id          BLOB PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL CHECK (length(trim(name)) > 0),
    notes       TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
) STRICT;

CREATE TABLE listings (
    id          BLOB PRIMARY KEY NOT NULL,
    product_id  BLOB NOT NULL REFERENCES products (id) ON DELETE CASCADE,
    url         TEXT NOT NULL UNIQUE,
    shop        TEXT NOT NULL,
    title       TEXT NOT NULL,
    image_url   TEXT,
    -- source:    the link you pasted
    -- candidate: found by search, not yet reviewed
    -- confirmed: same product, verified
    -- rejected:  not the same product
    status      TEXT NOT NULL CHECK (status IN ('source', 'candidate', 'confirmed', 'rejected')),
    created_at  TEXT NOT NULL
) STRICT;

CREATE INDEX listings_product_id ON listings (product_id);

-- Append-only price history per listing.
CREATE TABLE price_observations (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    listing_id   BLOB NOT NULL REFERENCES listings (id) ON DELETE CASCADE,
    minor_units  INTEGER NOT NULL,
    currency     TEXT NOT NULL CHECK (length(currency) = 3),
    observed_at  TEXT NOT NULL
) STRICT;

CREATE INDEX price_observations_listing ON price_observations (listing_id, observed_at);
