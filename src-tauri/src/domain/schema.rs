use rusqlite::{Connection, params};

const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS app_meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS profile (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  locale TEXT NOT NULL,
  timezone TEXT NOT NULL,
  household_size INTEGER NOT NULL CHECK (household_size > 0),
  height_cm REAL,
  weight_kg REAL,
  birth_date TEXT,
  sex_for_energy TEXT,
  activity_level TEXT NOT NULL DEFAULT 'low_active',
  calorie_target_mode TEXT NOT NULL DEFAULT 'manual',
  manual_calorie_target_kcal REAL,
  dietary_style TEXT,
  nutrition_targets_json TEXT NOT NULL,
  weekday_max_minutes INTEGER NOT NULL CHECK (weekday_max_minutes >= 0),
  weekend_max_minutes INTEGER NOT NULL CHECK (weekend_max_minutes >= 0),
  default_servings REAL NOT NULL CHECK (default_servings > 0),
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS equipment (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  category TEXT NOT NULL,
  available INTEGER NOT NULL DEFAULT 1,
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS recipes (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  summary TEXT NOT NULL DEFAULT '',
  servings REAL NOT NULL CHECK (servings > 0),
  prep_minutes INTEGER NOT NULL CHECK (prep_minutes >= 0),
  cook_minutes INTEGER NOT NULL CHECK (cook_minutes >= 0),
  difficulty TEXT NOT NULL DEFAULT 'easy',
  cuisine TEXT NOT NULL DEFAULT '',
  meal_types_json TEXT NOT NULL DEFAULT '[]',
  tags_json TEXT NOT NULL DEFAULT '[]',
  favorite INTEGER NOT NULL DEFAULT 0,
  archived INTEGER NOT NULL DEFAULT 0,
  source_kind TEXT NOT NULL DEFAULT 'generated',
  confidence REAL NOT NULL DEFAULT 0.7 CHECK (confidence >= 0 AND confidence <= 1),
  calories_kcal REAL NOT NULL DEFAULT 0,
  protein_g REAL NOT NULL DEFAULT 0,
  carbs_g REAL NOT NULL DEFAULT 0,
  fat_g REAL NOT NULL DEFAULT 0,
  fiber_g REAL NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_recipes_title ON recipes(title COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_recipes_updated ON recipes(updated_at DESC);

CREATE TABLE IF NOT EXISTS recipe_ingredients (
  id TEXT PRIMARY KEY,
  recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  quantity REAL NOT NULL CHECK (quantity >= 0),
  unit TEXT NOT NULL,
  aisle TEXT NOT NULL DEFAULT 'Sonstiges',
  preparation TEXT,
  optional INTEGER NOT NULL DEFAULT 0,
  calories_kcal REAL NOT NULL DEFAULT 0,
  protein_g REAL NOT NULL DEFAULT 0,
  carbs_g REAL NOT NULL DEFAULT 0,
  fat_g REAL NOT NULL DEFAULT 0,
  fiber_g REAL NOT NULL DEFAULT 0,
  position INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_ingredients_recipe ON recipe_ingredients(recipe_id, position);

CREATE TABLE IF NOT EXISTS recipe_steps (
  id TEXT PRIMARY KEY,
  recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  instruction TEXT NOT NULL,
  timer_minutes INTEGER CHECK (timer_minutes IS NULL OR timer_minutes >= 0)
);

CREATE INDEX IF NOT EXISTS idx_steps_recipe ON recipe_steps(recipe_id, position);

CREATE TABLE IF NOT EXISTS recipe_sources (
  id TEXT PRIMARY KEY,
  recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  url TEXT,
  publisher TEXT,
  source_type TEXT NOT NULL DEFAULT 'web',
  accessed_at TEXT
);

CREATE TABLE IF NOT EXISTS recipe_images (
  id TEXT PRIMARY KEY,
  recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  url TEXT NOT NULL,
  kind TEXT NOT NULL DEFAULT 'remote',
  alt_text TEXT,
  attribution TEXT,
  position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS recipe_ratings (
  id TEXT PRIMARY KEY,
  recipe_id TEXT NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
  score INTEGER NOT NULL CHECK (score BETWEEN 1 AND 5),
  comment TEXT,
  cooked_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ratings_recipe ON recipe_ratings(recipe_id, created_at DESC);

CREATE TABLE IF NOT EXISTS plan_entries (
  id TEXT PRIMARY KEY,
  date TEXT NOT NULL,
  slot TEXT NOT NULL,
  recipe_id TEXT REFERENCES recipes(id) ON DELETE SET NULL,
  title_override TEXT,
  servings REAL NOT NULL CHECK (servings > 0),
  status TEXT NOT NULL DEFAULT 'planned',
  notes TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(date, slot, sort_order)
);

CREATE INDEX IF NOT EXISTS idx_plan_date ON plan_entries(date, sort_order);

CREATE TABLE IF NOT EXISTS shopping_lists (
  id TEXT PRIMARY KEY,
  range_start TEXT NOT NULL,
  range_end TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(range_start, range_end)
);

CREATE TABLE IF NOT EXISTS shopping_items (
  id TEXT PRIMARY KEY,
  list_id TEXT NOT NULL REFERENCES shopping_lists(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL,
  quantity REAL NOT NULL CHECK (quantity >= 0),
  unit TEXT NOT NULL,
  aisle TEXT NOT NULL DEFAULT 'Sonstiges',
  checked INTEGER NOT NULL DEFAULT 0,
  manual INTEGER NOT NULL DEFAULT 0,
  recipe_ids_json TEXT NOT NULL DEFAULT '[]',
  note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_shopping_list ON shopping_items(list_id, aisle, name);
CREATE INDEX IF NOT EXISTS idx_shopping_key ON shopping_items(list_id, normalized_name, unit);

CREATE TABLE IF NOT EXISTS pantry_items (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  normalized_name TEXT NOT NULL UNIQUE,
  exclude_from_shopping INTEGER NOT NULL DEFAULT 1,
  note TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS memories (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  content TEXT NOT NULL,
  confidence REAL NOT NULL CHECK (confidence >= 0 AND confidence <= 1),
  evidence_json TEXT NOT NULL DEFAULT '[]',
  status TEXT NOT NULL DEFAULT 'proposed',
  preference_score REAL CHECK (preference_score IS NULL OR (preference_score >= 1 AND preference_score <= 10)),
  source TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_used_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_memories_status_kind ON memories(status, kind, updated_at DESC);

CREATE TABLE IF NOT EXISTS agent_sessions (
  id TEXT PRIMARY KEY,
  codex_thread_id TEXT,
  title TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'active',
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES agent_sessions(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  item_id TEXT,
  tool_name TEXT,
  tool_payload_json TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON agent_messages(session_id, created_at);

CREATE TABLE IF NOT EXISTS undo_history (
  id TEXT PRIMARY KEY,
  entity_kind TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  action TEXT NOT NULL,
  before_json TEXT,
  after_json TEXT,
  undone INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  undone_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_undo_active ON undo_history(undone, created_at DESC);

PRAGMA user_version = 2;
"#;

pub fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let transaction = conn.transaction()?;
    transaction.execute_batch(SCHEMA)?;
    // `CREATE TABLE IF NOT EXISTS` does not add fields to installations that
    // already have a MealZ profile. Keep these migrations additive so local
    // user data is never discarded when the nutrition profile evolves.
    add_profile_column_if_missing(&transaction, "sex_for_energy", "TEXT")?;
    add_profile_column_if_missing(
        &transaction,
        "activity_level",
        "TEXT NOT NULL DEFAULT 'low_active'",
    )?;
    add_profile_column_if_missing(
        &transaction,
        "calorie_target_mode",
        "TEXT NOT NULL DEFAULT 'manual'",
    )?;
    add_profile_column_if_missing(&transaction, "manual_calorie_target_kcal", "REAL")?;
    seed_profile(&transaction)?;
    seed_equipment(&transaction)?;
    seed_memories(&transaction)?;
    seed_recipes(&transaction)?;
    transaction.execute(
        "INSERT OR REPLACE INTO app_meta(key, value) VALUES ('schema_version', '2')",
        [],
    )?;
    transaction.commit()
}

fn add_profile_column_if_missing(
    conn: &Connection,
    name: &str,
    declaration: &str,
) -> rusqlite::Result<()> {
    let mut statement = conn.prepare("PRAGMA table_info(profile)")?;
    let exists = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .any(|column| column == name);
    if !exists {
        conn.execute_batch(&format!(
            "ALTER TABLE profile ADD COLUMN {name} {declaration}"
        ))?;
    }
    Ok(())
}

fn seed_profile(conn: &Connection) -> rusqlite::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO profile (
            id, name, locale, timezone, household_size, nutrition_targets_json,
            weekday_max_minutes, weekend_max_minutes, default_servings, notes,
            created_at, updated_at
         ) VALUES (?1, '', 'de-DE', 'Europe/Berlin', 1, ?2, 30, 75, 1, ?3, ?4, ?4)",
        params![
            "profile-local",
            r#"{"caloriesKcal":null,"proteinG":null,"carbsG":null,"fatG":null,"fiberG":35.0}"#,
            "Unter der Woche schnell und alltagstauglich; am Wochenende darf Kochen mehr Zeit bekommen.",
            now
        ],
    )?;
    Ok(())
}

fn seed_equipment(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM equipment", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let now = chrono::Utc::now().to_rfc3339();
    let equipment = [
        ("equipment-stove", "Herd mit 4 Kochfeldern", "Kochen"),
        ("equipment-oven", "Backofen", "Backen"),
        (
            "equipment-airfryer",
            "Heißluftfritteuse mit 2 Fächern",
            "Garen",
        ),
        ("equipment-blender", "Mixer", "Vorbereitung"),
        ("equipment-chopper", "Gemüse-Zerkleinerer", "Vorbereitung"),
        ("equipment-grill", "Kontaktgrill", "Grillen"),
        ("equipment-toaster", "Toaster", "Backen"),
        (
            "equipment-microwave",
            "Mikrowelle mit Ofenfunktion",
            "Aufwärmen",
        ),
        (
            "equipment-foodprocessor",
            "Küchenmaschine (Monsieur Cuisine)",
            "Kochen",
        ),
    ];
    for (id, name, category) in equipment {
        conn.execute(
            "INSERT INTO equipment(id, name, category, available, created_at, updated_at)
             VALUES (?1, ?2, ?3, 1, ?4, ?4)",
            params![id, name, category, now],
        )?;
    }
    Ok(())
}

fn seed_memories(conn: &Connection) -> rusqlite::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO memories (
            id, kind, content, confidence, evidence_json, status, preference_score, source, created_at, updated_at
         ) VALUES (?1, 'routine', ?2, 1.0, ?3, 'confirmed', NULL, 'onboarding', ?4, ?4)",
        params![
            "memory-no-breakfast",
            "Normalerweise gibt es kein Frühstück; der Fokus liegt auf spätem Mittagessen, Abendessen und Snacks oder Proteinshakes.",
            r#"["Vom Nutzer im Produktbriefing ausdrücklich angegeben"]"#,
            now
        ],
    )?;
    Ok(())
}

fn seed_recipes(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM recipes", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO recipes (
            id, title, summary, servings, prep_minutes, cook_minutes, difficulty, cuisine,
            meal_types_json, tags_json, favorite, archived, source_kind, confidence,
            calories_kcal, protein_g, carbs_g, fat_g, fiber_g, created_at, updated_at
         ) VALUES (?1, ?2, ?3, 2, 15, 25, 'easy', 'modern-europäisch', ?4, ?5, 0, 0,
                   'starter', 0.9, 1270, 102, 122, 40, 25, ?6, ?6)",
        params![
            "recipe-seed-chicken-bowl",
            "Knusprige Paprika-Hähnchen-Bowl",
            "Schnelle Feierabend-Bowl mit Ofenkartoffeln, Paprika, Hähnchen und Kräuterquark.",
            r#"["lunch","dinner"]"#,
            r#"["high-protein","weekday","airfryer"]"#,
            now
        ],
    )?;

    let ingredients = [
        (
            "ing-seed-chicken",
            "Hähnchenbrust",
            320.0,
            "g",
            "Fleisch",
            528.0,
            99.0,
            0.0,
            11.0,
            0.0,
        ),
        (
            "ing-seed-potatoes",
            "Kartoffeln",
            500.0,
            "g",
            "Gemüse",
            385.0,
            10.0,
            85.0,
            0.5,
            11.0,
        ),
        (
            "ing-seed-pepper",
            "Paprika",
            2.0,
            "Stück",
            "Gemüse",
            80.0,
            3.0,
            14.0,
            0.8,
            5.0,
        ),
        (
            "ing-seed-quark",
            "Magerquark",
            250.0,
            "g",
            "Kühlregal",
            168.0,
            30.0,
            10.0,
            0.5,
            0.0,
        ),
        (
            "ing-seed-oil",
            "Olivenöl",
            20.0,
            "ml",
            "Vorrat",
            177.0,
            0.0,
            0.0,
            20.0,
            0.0,
        ),
        (
            "ing-seed-herbs",
            "Frische Kräuter",
            15.0,
            "g",
            "Gemüse",
            5.0,
            0.0,
            1.0,
            0.0,
            1.0,
        ),
    ];
    for (position, item) in ingredients.iter().enumerate() {
        conn.execute(
            "INSERT INTO recipe_ingredients (
                id, recipe_id, name, quantity, unit, aisle, calories_kcal, protein_g,
                carbs_g, fat_g, fiber_g, position
             ) VALUES (?1, 'recipe-seed-chicken-bowl', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                item.0,
                item.1,
                item.2,
                item.3,
                item.4,
                item.5,
                item.6,
                item.7,
                item.8,
                item.9,
                position as i64
            ],
        )?;
    }

    let steps = [
        "Kartoffeln würfeln, mit der Hälfte des Öls würzen und im Airfryer oder Ofen knusprig garen.",
        "Hähnchen und Paprika schneiden, würzen und mit dem restlichen Öl braten oder im zweiten Airfryer-Fach garen.",
        "Magerquark mit Kräutern, Salz, Pfeffer und etwas Wasser cremig rühren.",
        "Alles auf zwei Bowls verteilen und mit dem Kräuterquark servieren.",
    ];
    for (position, instruction) in steps.iter().enumerate() {
        conn.execute(
            "INSERT INTO recipe_steps(id, recipe_id, position, instruction)
             VALUES (?1, 'recipe-seed-chicken-bowl', ?2, ?3)",
            params![
                format!("step-seed-{position}"),
                position as i64,
                instruction
            ],
        )?;
    }
    Ok(())
}
