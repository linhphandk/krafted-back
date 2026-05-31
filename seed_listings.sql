DO $$
DECLARE
    seller_id UUID := '780cb99c-9010-4365-906c-68cd6f22562f';
    cat_id UUID;
    craft_cats UUID[] := '{}';
    supply_cats UUID[] := '{}';
    listing_id UUID;
    titles_craft TEXT[] := ARRAY[
        'Handcrafted Stoneware Mug', 'Woven Basket', 'Ceramic Vase', 'Knitted Beanie',
        'Leather Wallet', 'Wooden Cutting Board', 'Silver Ring', 'Crochet Scarf',
        'Oil Painting – Sunset', 'Hand-thrown Bowl', 'Macrame Wall Hanging', 'Clay Earrings',
        'Embroidered Hoop Art', 'Scented Soy Candle', 'Hand-carved Spoon', 'Terracotta Planter',
        'Wool Sweater', 'Leather Tote Bag', 'Watercolor Print', 'Beaded Bracelet',
        'Ceramic Teapot', 'Knitted Baby Blanket', 'Wooden Jewelry Box', 'Candle Set – Lavender',
        'Lino Print – Bird', 'Felted Wool Ornament', 'Copper Wire Pendant', 'Crochet Amigurumi',
        'Porcelain Plate Set', 'Handbound Journal', 'Terrazzo Coaster Set', 'Leather Belt',
        'Acrylic Abstract Canvas', 'Stoneware Dinner Plate', 'Knitted Mittens', 'Boho Earrings',
        'Wooden Salad Bowl', 'Beeswax Wraps – Set of 3', 'Ceramic Butter Dish', 'Crochet Market Bag',
        'Hand-printed Tea Towel', 'Silver Pendant Necklace', 'Clay Candle Holder', 'Woven Wall Tapestry',
        'Leather Keychain', 'Porcelain Soap Dispenser', 'Knitted Dog Sweater', 'Wooden Toy Car',
        'Resin Geode Coaster', 'Watercolor Greeting Card Set', 'Hand-carved Rubber Stamp',
        'Cotton Macrame Plant Hanger', 'Stoneware Pitcher', 'Leather Passport Cover', 'Painted Silk Scarf',
        'Beaded Dreamcatcher', 'Ceramic Spoon Rest', 'Knitted Leg Warmers', 'Wooden Picture Frame'
    ];
    titles_supply TEXT[] := ARRAY[
        'Premium Merino Yarn – 100g', 'Cotton Fabric Bundle', 'Clay – Stoneware 5lb',
        'Wood Carving Tools Set', 'Bead Mix – 500pc', 'Paint Brush Set – 10pc',
        'Candle Making Kit', 'Embroidery Hoop – 8"', 'Leather Hide – 3sqft',
        'Pottery Wheel – Tabletop', 'Knitting Needles – Set of 8', 'Fabric Dye – Assorted',
        'Polymer Clay – 24 Colors', 'Sewing Pattern – Tote Bag', 'Jewelry Pliers Set',
        'Watercolor Paint Set – 12', 'Candle Wax – Soy 2lb', 'Wire – Silver 20ga',
        'Felt Sheets – 20pc', 'Wood Planks – Pine 12"x6"', 'Embroidery Thread Set – 36',
        'Pinch Pot Clay Kit', 'Canvas Panels – 5pc', 'Resin Epoxy – 16oz',
        'Screen Printing Frame', 'Basket Weaving Reed', 'Glass Beads – 200pc',
        'Leather Stitching Awl', 'Calligraphy Pen Set', 'Air Dry Clay – 2.2lb',
        'Heat Gun for Crafts', 'Stamp Carving Block', 'Needle Felting Kit',
        'Seed Beads – Czech 10/0', 'Linen Fabric – Natural', 'Glaze Set – 6 Colors',
        'Macrame Cord – 100m', 'Wax Seal Stamp Kit', 'Soldering Iron – Jewelry',
        'Sanding Block Assortment', 'Earring Hooks – 50pc', 'Bone Folder – Bookbinding',
        'Peg Loom – Weaving', 'Dye – Indigo 50g', 'Copper Sheet – 6"x6"',
        'Jump Rings – Sterling Silver', 'Fabric Scissors – Tailor', 'Chunky Glitter Pack',
        'Rosin – For Violin Bow', 'Spinning Wheel – Drop Spindle'
    ];
    descriptions TEXT[] := ARRAY[
        'Perfect for daily use, handcrafted with care.',
        'Each piece is unique — slight variations add to its charm.',
        'Made from high-quality materials sourced locally.',
        'Ideal gift for craft enthusiasts.',
        'Designed to last a lifetime with proper care.',
        'Handmade in small batches to ensure quality.',
        'Sustainable and eco-friendly materials used throughout.',
        'A beautiful addition to any home decor.',
        'Comfortable and stylish for everyday wear.',
        'Thoughtfully designed with attention to every detail.',
        'Perfect for beginners and experienced makers alike.',
        'Bulk pack — great for workshops and classes.',
        'Premium grade — professional quality you can trust.',
        'Versatile and easy to work with.',
        'Limited stock — hand-selected for consistency.'
    ];
    conditions TEXT[] := ARRAY['handmade', 'new', 'vintage', 'refurbished'];
    i INTEGER;
    title TEXT;
    cat_type TEXT;
    cat_ids UUID[];
    chosen_cat UUID;
    price INTEGER;
    cond TEXT;
    qty INTEGER;
BEGIN
    -- Collect category IDs by kind
    SELECT ARRAY_AGG(id) INTO craft_cats FROM categories WHERE kind = 'craft';
    SELECT ARRAY_AGG(id) INTO supply_cats FROM categories WHERE kind = 'supply';

    -- 200 craft listings
    FOR i IN 1..200 LOOP
        listing_id := gen_random_uuid();
        title := titles_craft[1 + (i % array_length(titles_craft, 1))];
        chosen_cat := craft_cats[1 + (i % array_length(craft_cats, 1))];
        price := (random() * 9500 + 500)::INT;
        cond := conditions[1 + (i % array_length(conditions, 1))];
        qty := GREATEST(1, (random() * 10)::INT);

        INSERT INTO listings (id, seller_id, title, description, price_cents, category_id, status, condition, quantity, created_at, updated_at)
        VALUES (
            listing_id, seller_id,
            title,
            descriptions[1 + (i % array_length(descriptions, 1))],
            price, chosen_cat, 'active', cond, qty,
            NOW() - (i || ' hours')::INTERVAL,
            NOW() - (i || ' hours')::INTERVAL
        );
    END LOOP;

    -- 100 supply listings
    FOR i IN 1..100 LOOP
        listing_id := gen_random_uuid();
        title := titles_supply[1 + (i % array_length(titles_supply, 1))];
        chosen_cat := supply_cats[1 + (i % array_length(supply_cats, 1))];
        price := (random() * 5000 + 200)::INT;
        cond := conditions[1 + (i % array_length(conditions, 1))];
        qty := GREATEST(1, (random() * 50)::INT);

        INSERT INTO listings (id, seller_id, title, description, price_cents, category_id, status, condition, quantity, created_at, updated_at)
        VALUES (
            listing_id, seller_id,
            title,
            descriptions[1 + (i % array_length(descriptions, 1))],
            price, chosen_cat, 'active', cond, qty,
            NOW() - (i || ' hours')::INTERVAL,
            NOW() - (i || ' hours')::INTERVAL
        );
    END LOOP;
END $$;
