-- Seed mock data: 5 users, each with 10 listings
-- Run: psql -U krafted -d krafted -f scripts/seed_mock_data.sql
-- Password for all users: password123

-- Ensure pgcrypto is available
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ─── Users ────────────────────────────────────────────────────

INSERT INTO users (id, email, name, password_hash, created_at, updated_at)
VALUES
  (gen_random_uuid(), 'alice@crafts.com',   'Alice Chen',    crypt('password123', gen_salt('bf')), now(), now()),
  (gen_random_uuid(), 'bob@woodworks.com',  'Bob Martinez',  crypt('password123', gen_salt('bf')), now(), now()),
  (gen_random_uuid(), 'carol@jewelry.com',  'Carol Singh',   crypt('password123', gen_salt('bf')), now(), now()),
  (gen_random_uuid(), 'david@pottery.com',  'David Kim',     crypt('password123', gen_salt('bf')), now(), now()),
  (gen_random_uuid(), 'emma@textiles.com',  'Emma Johnson',  crypt('password123', gen_salt('bf')), now(), now());

-- ─── Listings ─────────────────────────────────────────────────

-- Alice: Pottery & Ceramics
INSERT INTO listings (id, seller_id, title, description, price_cents, category_id, status, condition, quantity, created_at, updated_at)
SELECT
  gen_random_uuid(),
  u.id,
  t.title,
  t.desc,
  t.price,
  (SELECT id FROM categories WHERE slug = 'pottery-ceramics'),
  t.status,
  t.condition,
  t.qty,
  now() - (random() * interval '30 days'),
  now() - (random() * interval '7 days')
FROM (
  VALUES
    ('Handcrafted Stoneware Mug',         'Microwave-safe stoneware mug with organic glaze',                     2200,  'active',   'handmade', 5),
    ('Porcelain Tea Set (6 cups)',        'Delicate hand-thrown porcelain tea set with bamboo tray',             8500,  'active',   'handmade', 2),
    ('Rustic Clay Planter Set',           'Set of 3 terracotta planters with drainage holes',                    3400,  'active',   'handmade', 8),
    ('Ceramic Pasta Bowl',                'Large hand-thrown pasta bowl, dishwasher safe',                       3800,  'active',   'handmade', 4),
    ('Speckled Dinner Plate Set',         'Set of 4 speckled stoneware dinner plates',                           5200,  'active',   'handmade', 3),
    ('Glazed Vase – Ocean Blue',          'Tall ceramic vase with reactive ocean-blue glaze',                   4600,  'draft',    'handmade', 1),
    ('Mini Succulent Pots (set of 5)',    'Tiny hand-built succulent pots with drainage',                        1800,  'active',   'handmade', 10),
    ('Sake Set – Modern White',           'Minimalist porcelain sake set: flask + 4 cups',                       4200,  'active',   'handmade', 2),
    ('Textured Mug – Speckle Glaze',      'Unique textured mug with speckled oatmeal glaze',                     2400,  'paused',   'handmade', 3),
    ('Large Serving Platter',             'Hand-thrown oval platter, perfect for entertaining',                  5500,  'active',   'handmade', 2)
) AS t(title, "desc", price, status, condition, qty)
CROSS JOIN (SELECT id FROM users WHERE email = 'alice@crafts.com') u;

-- Bob: Woodworking
INSERT INTO listings (id, seller_id, title, description, price_cents, category_id, status, condition, quantity, created_at, updated_at)
SELECT
  gen_random_uuid(),
  u.id,
  t.title,
  t.desc,
  t.price,
  (SELECT id FROM categories WHERE slug = 'woodworking'),
  t.status,
  t.condition,
  t.qty,
  now() - (random() * interval '30 days'),
  now() - (random() * interval '7 days')
FROM (
  VALUES
    ('Walnut Cutting Board',              'End-grain walnut cutting board with juice groove',                    6500,  'active',   'handmade', 3),
    ('Live Edge Coffee Table',            'Solid oak live-edge coffee table with epoxy river',                   45000, 'active',   'handmade', 1),
    ('Cherry Wood Salad Bowl Set',        'Set of 2 hand-turned cherry wood bowls',                              4800,  'active',   'handmade', 4),
    ('Bamboo Phone Stand',                'Minimalist angled bamboo phone stand',                                1200,  'active',   'handmade', 15),
    ('Teak Wall Shelf',                   'Floating teak wall shelf with hidden bracket',                        3500,  'draft',    'handmade', 2),
    ('Maple Rolling Pin',                 'Smooth maple rolling pin, tapered ends',                              2800,  'active',   'handmade', 6),
    ('Oak Jewelry Box',                   'Hand-dovetailed oak jewelry box with velvet lining',                  7200,  'active',   'handmade', 2),
    ('Pine Coat Rack',                    'Rustic pine coat rack with 6 hooks',                                  4200,  'active',   'handmade', 3),
    ('Walnut Serving Tray',               'Elegant walnut serving tray with leather handles',                    3800,  'paused',   'handmade', 2),
    ('Butcher Block – Maple',             'Heavy-duty maple butcher block, 18x24',                               12000, 'active',   'handmade', 1)
) AS t(title, "desc", price, status, condition, qty)
CROSS JOIN (SELECT id FROM users WHERE email = 'bob@woodworks.com') u;

-- Carol: Jewelry
INSERT INTO listings (id, seller_id, title, description, price_cents, category_id, status, condition, quantity, created_at, updated_at)
SELECT
  gen_random_uuid(),
  u.id,
  t.title,
  t.desc,
  t.price,
  (SELECT id FROM categories WHERE slug = 'jewelry'),
  t.status,
  t.condition,
  t.qty,
  now() - (random() * interval '30 days'),
  now() - (random() * interval '7 days')
FROM (
  VALUES
    ('Silver Dangle Earrings',            'Hand-forged sterling silver dangle earrings',                         3800,  'active',   'handmade', 5),
    ('Gold Hoop Earrings (small)',         '14k gold-filled small hoop earrings',                                4200,  'active',   'handmade', 8),
    ('Beaded Wrap Bracelet',              'Adjustable beaded wrap bracelet – turquoise',                         2600,  'active',   'handmade', 6),
    ('Raw Crystal Pendant',               'Amethyst raw crystal pendant on leather cord',                       3400,  'active',   'handmade', 4),
    ('Pearl Stud Earrings',               'Freshwater pearl studs on sterling silver posts',                    2800,  'active',   'handmade', 7),
    ('Copper Wire Ring',                  'Hammered copper wire ring – adjustable',                              800,  'draft',    'handmade', 12),
    ('Leather Wrap Cuff',                 'Braided leather cuff with magnetic clasp',                            3200,  'active',   'handmade', 3),
    ('Statement Resin Earrings',          'Lightweight resin earrings with dried flowers',                       2400,  'active',   'handmade', 5),
    ('Stacking Rings Set (3)',            'Set of 3 thin sterling silver stacking rings',                        3600,  'paused',   'handmade', 4),
    ('Choker Necklace – Moon Phase',      'Silver moon phase pendant on adjustable chain',                       4500,  'active',   'handmade', 2)
) AS t(title, "desc", price, status, condition, qty)
CROSS JOIN (SELECT id FROM users WHERE email = 'carol@jewelry.com') u;

-- David: Pottery (also)
INSERT INTO listings (id, seller_id, title, description, price_cents, category_id, status, condition, quantity, created_at, updated_at)
SELECT
  gen_random_uuid(),
  u.id,
  t.title,
  t.desc,
  t.price,
  (SELECT id FROM categories WHERE slug = 'pottery-ceramics'),
  t.status,
  t.condition,
  t.qty,
  now() - (random() * interval '30 days'),
  now() - (random() * interval '7 days')
FROM (
  VALUES
    ('Raku Fired Vase',                   'One-of-a-kind raku-fired vase with crackle glaze',                    5800,  'active',   'handmade', 1),
    ('Stoneware Beer Stein',              'Heavy stoneware beer stein, holds 500ml',                             2200,  'active',   'handmade', 6),
    ('Ceramic Diffuser – Lotus',          'Essential oil diffuser in lotus shape',                              3200,  'draft',    'handmade', 3),
    ('Hand-built Wall Pocket',            'Decorative ceramic wall pocket for dried florals',                   2800,  'active',   'handmade', 4),
    ('Ramen Bowl Set (2)',                'Deep ramen bowls with chopstick rest',                               4800,  'active',   'handmade', 3),
    ('Mosaic Tile Coaster Set (4)',       'Hand-cut ceramic mosaic coasters with cork bottom',                  1800,  'active',   'handmade', 8),
    ('Tea Light Candle Holders (set 3)',  'Minimalist ceramic tea light holders',                                1400,  'active',   'handmade', 10),
    ('Large Garden Urn',                  'Weather-resistant ceramic garden urn, 40cm tall',                     9500,  'paused',   'handmade', 1),
    ('Matcha Bowl – Hand-thrown',         'Traditional-style hand-thrown matcha bowl',                           3600,  'active',   'handmade', 2),
    ('Pitcher – Nordic Blue',             'Hand-thrown water pitcher with Nordic blue glaze',                    4200,  'active',   'handmade', 2)
) AS t(title, "desc", price, status, condition, qty)
CROSS JOIN (SELECT id FROM users WHERE email = 'david@pottery.com') u;

-- Emma: Textiles
INSERT INTO listings (id, seller_id, title, description, price_cents, category_id, status, condition, quantity, created_at, updated_at)
SELECT
  gen_random_uuid(),
  u.id,
  t.title,
  t.desc,
  t.price,
  (SELECT id FROM categories WHERE slug = 'yarn-fiber'),
  t.status,
  t.condition,
  t.qty,
  now() - (random() * interval '30 days'),
  now() - (random() * interval '7 days')
FROM (
  VALUES
    ('Hand-dyed Merino Wool Sock Yarn',    'Superwash merino, 400m per skein – Sunset gradient',                 2800,  'active',   'handmade', 10),
    ('Alpaca Blend Chunky Yarn',           'Luxurious alpaca-merino blend, 150m per skein',                      2400,  'active',   'handmade', 8),
    ('Linen-Cotton Kitchen Towels (set 4)','Handwoven linen-cotton towels with loop',                            3200,  'active',   'handmade', 5),
    ('Knitted Beanie – Rust Orange',       'Chunky knit beanie in rust orange',                                  2200,  'active',   'handmade', 4),
    ('Macrame Wall Hanging',              'Large macrame wall hanging, 80cm drop',                               6500,  'active',   'handmade', 2),
    ('Woven Scarf – Natural Indigo',       'Handwoven cotton scarf dyed with natural indigo',                     4800,  'draft',    'handmade', 3),
    ('Crochet Amigurumi Bunny',            'Adorable crochet bunny, approx 20cm tall',                            1800,  'active',   'handmade', 6),
    ('Tassel Garland – Rainbow',           'DIY tassel garland kit, makes 3m',                                   1400,  'active',   'handmade', 12),
    ('Needle Felted Owl',                  'Realistic needle felted owl ornament',                               2600,  'active',   'handmade', 3),
    ('Wool Blanket – Herringbone',         'Woven wool throw blanket, herringbone pattern',                       12000, 'paused',   'handmade', 1)
) AS t(title, "desc", price, status, condition, qty)
CROSS JOIN (SELECT id FROM users WHERE email = 'emma@textiles.com') u;
