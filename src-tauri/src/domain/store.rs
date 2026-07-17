use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::Path,
    sync::Arc,
};

use chrono::{Datelike, Duration, Local, NaiveDate};
use parking_lot::{Mutex, MutexGuard};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::{Value, json};
use thiserror::Error;
use uuid::Uuid;

use super::{models::*, schema};

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("SQLite-Fehler: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON-Fehler: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Dateisystem-Fehler: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ungültige Eingabe: {0}")]
    Validation(String),
    #[error("Nicht gefunden: {0}")]
    NotFound(String),
}

pub type DomainResult<T> = Result<T, DomainError>;

#[derive(Clone)]
pub struct MealzStore {
    connection: Arc<Mutex<Connection>>,
}

impl std::fmt::Debug for MealzStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("MealzStore").finish_non_exhaustive()
    }
}

impl MealzStore {
    pub fn open(path: impl AsRef<Path>) -> DomainResult<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn in_memory() -> DomainResult<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(mut connection: Connection) -> DomainResult<Self> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        schema::migrate(&mut connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn conn(&self) -> MutexGuard<'_, Connection> {
        self.connection.lock()
    }

    pub fn get_app_meta(&self, key: &str) -> DomainResult<Option<String>> {
        Ok(self
            .conn()
            .query_row("SELECT value FROM app_meta WHERE key=?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    pub fn set_app_meta(&self, key: &str, value: &str) -> DomainResult<()> {
        if key.trim().is_empty() {
            return Err(DomainError::Validation(
                "App-Metadaten benötigen einen Schlüssel".into(),
            ));
        }
        self.conn().execute(
            "INSERT INTO app_meta(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn onboarding_complete(&self) -> DomainResult<bool> {
        Ok(matches!(
            self.get_app_meta("onboarding_complete")?.as_deref(),
            Some("true" | "1")
        ))
    }

    pub fn bootstrap(&self) -> DomainResult<BootstrapData> {
        let today = Local::now().date_naive();
        let week_start = today - Duration::days(today.weekday().num_days_from_monday() as i64);
        let profile = self.get_profile()?;
        let equipment = self.list_equipment()?;
        let recipes = self.search_recipes(None, false, 24)?;
        let current_week = self.get_week(week_start)?;
        let memories = self.recall_memories(None, None, Some("confirmed"), 50)?;
        let shopping_list = self
            .get_shopping_list(week_start, week_start + Duration::days(6))
            .ok();
        Ok(BootstrapData {
            profile,
            equipment,
            recipes,
            current_week,
            memories,
            shopping_list,
        })
    }

    pub fn get_profile(&self) -> DomainResult<UserProfile> {
        read_profile(&self.conn())
    }

    pub fn update_profile(&self, patch: ProfilePatch) -> DomainResult<UserProfile> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let before = read_profile(&transaction)?;
        let mut after = before.clone();
        if let Some(value) = patch.name {
            after.name = value;
        }
        if let Some(value) = patch.locale {
            after.locale = value;
        }
        if let Some(value) = patch.timezone {
            after.timezone = value;
        }
        if let Some(value) = patch.household_size {
            after.household_size = value;
        }
        if let Some(value) = patch.height_cm {
            after.height_cm = value;
        }
        if let Some(value) = patch.weight_kg {
            after.weight_kg = value;
        }
        if let Some(value) = patch.birth_date {
            after.birth_date = value;
        }
        if let Some(value) = patch.sex_for_energy {
            after.sex_for_energy = value;
        }
        if let Some(value) = patch.activity_level {
            after.activity_level = value;
        }
        if let Some(value) = patch.calorie_target_mode {
            after.calorie_target_mode = value;
        }
        if let Some(value) = patch.manual_calorie_target_kcal {
            after.manual_calorie_target_kcal = value;
        }
        if let Some(value) = patch.dietary_style {
            after.dietary_style = value;
        }
        if let Some(value) = patch.nutrition_targets {
            // The UI's existing calorie input remains the manual override.
            // Preserve it even when the currently displayed target is an
            // automatically calculated EER.
            if after.calorie_target_mode == "manual" {
                after.manual_calorie_target_kcal = value.calories_kcal;
            }
            after.nutrition_targets = value;
        }
        if let Some(value) = patch.weekday_max_minutes {
            after.weekday_max_minutes = value;
        }
        if let Some(value) = patch.weekend_max_minutes {
            after.weekend_max_minutes = value;
        }
        if let Some(value) = patch.default_servings {
            after.default_servings = value;
        }
        if let Some(value) = patch.notes {
            after.notes = value;
        }
        validate_profile(&after)?;
        apply_calorie_target_mode(&mut after)?;
        after.updated_at = now_rfc3339();
        write_profile(&transaction, &after)?;
        record_undo(
            &transaction,
            "profile",
            &after.id,
            "update",
            Some(serde_json::to_value(&before)?),
            Some(serde_json::to_value(&after)?),
        )?;
        transaction.commit()?;
        Ok(after)
    }

    pub fn list_equipment(&self) -> DomainResult<Vec<Equipment>> {
        let connection = self.conn();
        let mut statement = connection.prepare(
            "SELECT id, name, category, available, notes, created_at, updated_at
             FROM equipment ORDER BY category COLLATE NOCASE, name COLLATE NOCASE",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(Equipment {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                available: row.get::<_, i64>(3)? != 0,
                notes: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_equipment(&self, mut equipment: Equipment) -> DomainResult<Equipment> {
        if equipment.name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Equipment benötigt einen Namen".into(),
            ));
        }
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        if equipment.id.is_empty() {
            equipment.id = new_id("equipment");
        }
        let before = read_equipment(&transaction, &equipment.id).ok();
        let now = now_rfc3339();
        if equipment.created_at.is_empty() {
            equipment.created_at = now.clone();
        }
        equipment.updated_at = now;
        transaction.execute(
            "INSERT INTO equipment(id, name, category, available, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, category=excluded.category,
                 available=excluded.available, notes=excluded.notes, updated_at=excluded.updated_at",
            params![
                equipment.id,
                equipment.name.trim(),
                equipment.category,
                equipment.available as i64,
                equipment.notes,
                equipment.created_at,
                equipment.updated_at
            ],
        )?;
        record_undo(
            &transaction,
            "equipment",
            &equipment.id,
            if before.is_some() { "update" } else { "create" },
            before.map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&equipment)?),
        )?;
        transaction.commit()?;
        Ok(equipment)
    }

    pub fn delete_equipment(&self, id: &str) -> DomainResult<()> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let before = read_equipment(&transaction, id)?;
        transaction.execute("DELETE FROM equipment WHERE id=?1", [id])?;
        record_undo(
            &transaction,
            "equipment",
            id,
            "delete",
            Some(serde_json::to_value(&before)?),
            None,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn save_recipe(&self, recipe: Recipe) -> DomainResult<Recipe> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let recipe = save_recipe_tx(&transaction, recipe, true)?;
        let affected_dates = planned_date_bounds_for_recipe(&transaction, &recipe.id)?;
        transaction.commit()?;
        drop(connection);
        if let Some((start, end)) = affected_dates {
            self.rebuild_existing_shopping_lists(start, end)?;
        }
        Ok(recipe)
    }

    pub fn get_recipe(&self, id: &str) -> DomainResult<Recipe> {
        read_recipe(&self.conn(), id)
    }

    pub fn delete_recipe(&self, id: &str) -> DomainResult<()> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let before = read_recipe(&transaction, id)?;
        let affected_dates = planned_date_bounds_for_recipe(&transaction, id)?;
        let planned_entries = read_plan_entries_for_recipe(&transaction, id)?;
        // A deleted recipe must not leave blank plan slots behind. Do this in
        // the same transaction and keep a compound undo snapshot so a single
        // undo restores both the recipe and its scheduled appearances.
        transaction.execute("DELETE FROM plan_entries WHERE recipe_id=?1", [id])?;
        transaction.execute("DELETE FROM recipes WHERE id=?1", [id])?;
        record_undo(
            &transaction,
            "recipe_with_plan",
            id,
            "delete",
            Some(json!({"recipe":before,"planEntries":planned_entries})),
            None,
        )?;
        transaction.commit()?;
        drop(connection);
        if let Some((start, end)) = affected_dates {
            self.rebuild_existing_shopping_lists(start, end)?;
        }
        Ok(())
    }

    pub fn search_recipes(
        &self,
        query: Option<&str>,
        include_archived: bool,
        limit: usize,
    ) -> DomainResult<Vec<RecipeSummary>> {
        let connection = self.conn();
        let search = format!("%{}%", query.unwrap_or_default().trim());
        let mut statement = connection.prepare(
            "SELECT id FROM recipes
             WHERE (?1 = '%%' OR title LIKE ?1 COLLATE NOCASE OR summary LIKE ?1 COLLATE NOCASE
                    OR tags_json LIKE ?1 COLLATE NOCASE)
               AND (?2 = 1 OR archived = 0)
             ORDER BY favorite DESC, updated_at DESC LIMIT ?3",
        )?;
        let ids = statement
            .query_map(
                params![search, include_archived as i64, limit.clamp(1, 250) as i64],
                |row| row.get::<_, String>(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.into_iter()
            .map(|id| read_recipe_summary(&connection, &id))
            .collect()
    }

    pub fn rate_recipe(&self, mut rating: RecipeRating) -> DomainResult<RecipeRating> {
        if !(1..=5).contains(&rating.score) {
            return Err(DomainError::Validation(
                "Bewertung muss zwischen 1 und 5 liegen".into(),
            ));
        }
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        ensure_recipe_exists(&transaction, &rating.recipe_id)?;
        if rating.id.is_empty() {
            rating.id = new_id("rating");
        }
        let before = read_rating(&transaction, &rating.id).ok();
        let now = now_rfc3339();
        if rating.created_at.is_empty() {
            rating.created_at = now.clone();
        }
        rating.updated_at = now;
        transaction.execute(
            "INSERT INTO recipe_ratings(id, recipe_id, score, comment, cooked_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET score=excluded.score, comment=excluded.comment,
                 cooked_at=excluded.cooked_at, updated_at=excluded.updated_at",
            params![
                rating.id,
                rating.recipe_id,
                rating.score,
                rating.comment,
                rating.cooked_at,
                rating.created_at,
                rating.updated_at
            ],
        )?;
        record_undo(
            &transaction,
            "rating",
            &rating.id,
            if before.is_some() { "update" } else { "create" },
            before.map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&rating)?),
        )?;
        transaction.commit()?;
        Ok(rating)
    }

    pub fn list_ratings(&self, recipe_id: &str) -> DomainResult<Vec<RecipeRating>> {
        let connection = self.conn();
        let mut statement = connection.prepare(
            "SELECT id, recipe_id, score, comment, cooked_at, created_at, updated_at
             FROM recipe_ratings WHERE recipe_id=?1 ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([recipe_id], row_rating)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn get_week(&self, week_start: NaiveDate) -> DomainResult<WeekPlan> {
        let week_end = week_start + Duration::days(6);
        let connection = self.conn();
        let entries = read_plan_entries(&connection, week_start, week_end)?;
        build_week(&connection, week_start, entries)
    }

    /// Reads the exact inclusive planning range requested by the calendar.
    /// The calendar deliberately caps this to 31 days so accidental broad
    /// queries cannot turn a quick local render into an unbounded history load.
    pub fn get_plan_range(&self, start: NaiveDate, end: NaiveDate) -> DomainResult<Vec<PlanEntry>> {
        if end < start {
            return Err(DomainError::Validation(
                "Enddatum darf nicht vor dem Startdatum liegen".into(),
            ));
        }
        if (end - start).num_days() > 30 {
            return Err(DomainError::Validation(
                "Kalenderbereiche dürfen höchstens 31 Tage enthalten".into(),
            ));
        }
        let connection = self.conn();
        read_plan_entries(&connection, start, end)
    }

    pub fn save_plan_entry(&self, entry: PlanEntry) -> DomainResult<PlanEntry> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let entry = save_plan_entry_tx(&transaction, entry, true)?;
        transaction.commit()?;
        drop(connection);
        self.rebuild_existing_shopping_lists(entry.date, entry.date)?;
        Ok(entry)
    }

    pub fn replace_week(
        &self,
        week_start: NaiveDate,
        entries: Vec<PlanEntry>,
    ) -> DomainResult<WeekPlan> {
        let week_end = week_start + Duration::days(6);
        if entries
            .iter()
            .any(|entry| entry.date < week_start || entry.date > week_end)
        {
            return Err(DomainError::Validation(
                "Alle Plan-Einträge müssen innerhalb der gewählten Woche liegen".into(),
            ));
        }
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let before = read_plan_entries(&transaction, week_start, week_end)?;
        transaction.execute(
            "DELETE FROM plan_entries WHERE date BETWEEN ?1 AND ?2",
            params![week_start.to_string(), week_end.to_string()],
        )?;
        let mut saved = Vec::with_capacity(entries.len());
        for entry in entries {
            saved.push(save_plan_entry_tx(&transaction, entry, false)?);
        }
        record_undo(
            &transaction,
            "week_plan",
            &week_start.to_string(),
            "replace",
            Some(serde_json::to_value(before)?),
            Some(serde_json::to_value(&saved)?),
        )?;
        transaction.commit()?;
        drop(connection);
        self.rebuild_existing_shopping_lists(week_start, week_end)?;
        self.get_week(week_start)
    }

    pub fn delete_plan_entry(&self, id: &str) -> DomainResult<()> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let before = read_plan_entry(&transaction, id)?;
        transaction.execute("DELETE FROM plan_entries WHERE id=?1", [id])?;
        record_undo(
            &transaction,
            "plan_entry",
            id,
            "delete",
            Some(serde_json::to_value(&before)?),
            None,
        )?;
        transaction.commit()?;
        drop(connection);
        self.rebuild_existing_shopping_lists(before.date, before.date)?;
        Ok(())
    }

    fn rebuild_existing_shopping_lists(
        &self,
        changed_start: NaiveDate,
        changed_end: NaiveDate,
    ) -> DomainResult<()> {
        let ranges = {
            let connection = self.conn();
            let mut statement = connection.prepare(
                "SELECT range_start, range_end FROM shopping_lists
                 WHERE range_start <= ?2 AND range_end >= ?1",
            )?;
            statement
                .query_map(
                    params![changed_start.to_string(), changed_end.to_string()],
                    |row| Ok((row.get::<_, NaiveDate>(0)?, row.get::<_, NaiveDate>(1)?)),
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        for (range_start, range_end) in ranges {
            self.rebuild_shopping_list(range_start, range_end)?;
        }
        Ok(())
    }

    fn rebuild_all_existing_shopping_lists(&self) -> DomainResult<()> {
        let ranges = {
            let connection = self.conn();
            let mut statement = connection.prepare(
                "SELECT range_start, range_end FROM shopping_lists ORDER BY range_start, range_end",
            )?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, NaiveDate>(0)?, row.get::<_, NaiveDate>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        for (range_start, range_end) in ranges {
            self.rebuild_shopping_list(range_start, range_end)?;
        }
        Ok(())
    }

    pub fn rebuild_shopping_list(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> DomainResult<ShoppingList> {
        if range_end < range_start {
            return Err(DomainError::Validation(
                "Das Enddatum der Einkaufsliste liegt vor dem Startdatum".into(),
            ));
        }
        if (range_end - range_start).num_days() > 62 {
            return Err(DomainError::Validation(
                "Eine Einkaufsliste darf höchstens 63 Tage umfassen".into(),
            ));
        }

        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let existing_id = transaction
            .query_row(
                "SELECT id FROM shopping_lists WHERE range_start=?1 AND range_end=?2",
                params![range_start.to_string(), range_end.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let list_id = existing_id.unwrap_or_else(|| new_id("shopping-list"));
        let now = now_rfc3339();
        transaction.execute(
            "INSERT INTO shopping_lists(id, range_start, range_end, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(range_start, range_end) DO UPDATE SET updated_at=excluded.updated_at",
            params![list_id, range_start.to_string(), range_end.to_string(), now],
        )?;

        let previous_items = read_shopping_items(&transaction, &list_id)?;
        let checked_by_key: HashMap<(String, String), bool> = previous_items
            .iter()
            .filter(|item| !item.manual)
            .map(|item| {
                (
                    (normalize_name(&item.name), item.unit.clone()),
                    item.checked,
                )
            })
            .collect();
        transaction.execute(
            "DELETE FROM shopping_items WHERE list_id=?1 AND manual=0",
            [&list_id],
        )?;

        #[derive(Default)]
        struct Aggregate {
            name: String,
            quantity: f64,
            unit: String,
            aisle: String,
            recipe_ids: BTreeSet<String>,
        }

        let mut aggregates: BTreeMap<(String, String), Aggregate> = BTreeMap::new();
        let pantry_exclusions = {
            let mut pantry_statement = transaction.prepare(
                "SELECT normalized_name FROM pantry_items WHERE exclude_from_shopping=1",
            )?;
            pantry_statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<BTreeSet<_>>>()?
        };
        let mut statement = transaction.prepare(
            "SELECT i.name, i.quantity, i.unit, i.aisle, p.servings, r.servings, r.id
             FROM plan_entries p
             JOIN recipes r ON r.id = p.recipe_id
             JOIN recipe_ingredients i ON i.recipe_id = r.id
             WHERE p.date BETWEEN ?1 AND ?2
               AND p.status NOT IN ('skipped', 'eating_out', 'cancelled')
               AND i.optional = 0
             ORDER BY p.date, p.sort_order, i.position",
        )?;
        let ingredients = statement.query_map(
            params![range_start.to_string(), range_end.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, f64>(4)?,
                    row.get::<_, f64>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )?;
        for ingredient in ingredients {
            let (name, quantity, unit, aisle, planned_servings, recipe_servings, recipe_id) =
                ingredient?;
            let scaled = quantity * planned_servings / recipe_servings.max(0.000_001);
            let (canonical_quantity, canonical_unit) = canonical_quantity(scaled, &unit);
            let normalized_name = normalize_name(&name);
            if pantry_exclusions.contains(&normalized_name) {
                continue;
            }
            let key = (normalized_name, canonical_unit.clone());
            let aggregate = aggregates.entry(key).or_default();
            if aggregate.name.is_empty() {
                aggregate.name = name;
                aggregate.unit = canonical_unit;
                aggregate.aisle = if aisle.trim().is_empty() {
                    "Sonstiges".into()
                } else {
                    aisle
                };
            }
            aggregate.quantity += canonical_quantity;
            aggregate.recipe_ids.insert(recipe_id);
        }
        drop(statement);

        for ((normalized_name, canonical_unit), aggregate) in aggregates {
            let checked = checked_by_key
                .get(&(normalized_name.clone(), canonical_unit.clone()))
                .copied()
                .unwrap_or(false);
            transaction.execute(
                "INSERT INTO shopping_items(
                    id, list_id, name, normalized_name, quantity, unit, aisle, checked,
                    manual, recipe_ids_json, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9, ?10, ?10)",
                params![
                    new_id("shopping-item"),
                    list_id,
                    aggregate.name,
                    normalized_name,
                    round_quantity(aggregate.quantity),
                    canonical_unit,
                    aggregate.aisle,
                    checked as i64,
                    serde_json::to_string(&aggregate.recipe_ids.into_iter().collect::<Vec<_>>())?,
                    now
                ],
            )?;
        }
        transaction.commit()?;
        drop(connection);
        self.get_shopping_list(range_start, range_end)
    }

    pub fn get_shopping_list(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
    ) -> DomainResult<ShoppingList> {
        let connection = self.conn();
        let (id, created_at, updated_at) = connection
            .query_row(
                "SELECT id, created_at, updated_at FROM shopping_lists
                 WHERE range_start=?1 AND range_end=?2",
                params![range_start.to_string(), range_end.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| DomainError::NotFound("Einkaufsliste".into()))?;
        let items = read_shopping_items(&connection, &id)?;
        Ok(ShoppingList {
            id,
            range_start,
            range_end,
            items,
            created_at,
            updated_at,
        })
    }

    pub fn set_shopping_checked(&self, id: &str, checked: bool) -> DomainResult<ShoppingItem> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let before = read_shopping_item(&transaction, id)?;
        let now = now_rfc3339();
        transaction.execute(
            "UPDATE shopping_items SET checked=?2, updated_at=?3 WHERE id=?1",
            params![id, checked as i64, now],
        )?;
        let after = read_shopping_item(&transaction, id)?;
        record_undo(
            &transaction,
            "shopping_item",
            id,
            "update",
            Some(serde_json::to_value(before)?),
            Some(serde_json::to_value(&after)?),
        )?;
        transaction.commit()?;
        Ok(after)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_manual_shopping_item(
        &self,
        range_start: NaiveDate,
        range_end: NaiveDate,
        name: String,
        quantity: f64,
        unit: String,
        aisle: Option<String>,
        note: Option<String>,
    ) -> DomainResult<ShoppingItem> {
        if name.trim().is_empty() || quantity <= 0.0 {
            return Err(DomainError::Validation(
                "Manuelle Einkaufsartikel benötigen Namen und positive Menge".into(),
            ));
        }
        let list = match self.get_shopping_list(range_start, range_end) {
            Ok(list) => list,
            Err(DomainError::NotFound(_)) => self.rebuild_shopping_list(range_start, range_end)?,
            Err(error) => return Err(error),
        };
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let now = now_rfc3339();
        let item = ShoppingItem {
            id: new_id("shopping-item"),
            list_id: list.id,
            name: name.trim().to_string(),
            quantity,
            unit: unit.trim().to_string(),
            aisle: aisle
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "Sonstiges".into()),
            checked: false,
            manual: true,
            recipe_ids: Vec::new(),
            note,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        write_shopping_item(&transaction, &item)?;
        record_undo(
            &transaction,
            "shopping_item",
            &item.id,
            "create",
            None,
            Some(serde_json::to_value(&item)?),
        )?;
        transaction.commit()?;
        Ok(item)
    }

    pub fn delete_shopping_item(&self, id: &str) -> DomainResult<()> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let before = read_shopping_item(&transaction, id)?;
        transaction.execute("DELETE FROM shopping_items WHERE id=?1", [id])?;
        record_undo(
            &transaction,
            "shopping_item",
            id,
            "delete",
            Some(serde_json::to_value(before)?),
            None,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_pantry_items(&self) -> DomainResult<Vec<PantryItem>> {
        let connection = self.conn();
        let mut statement = connection.prepare(
            "SELECT id, name, exclude_from_shopping, note, created_at, updated_at
             FROM pantry_items ORDER BY name COLLATE NOCASE",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(PantryItem {
                id: row.get(0)?,
                name: row.get(1)?,
                exclude_from_shopping: row.get::<_, i64>(2)? != 0,
                note: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn save_pantry_item(&self, mut item: PantryItem) -> DomainResult<PantryItem> {
        if item.name.trim().is_empty() {
            return Err(DomainError::Validation(
                "Vorratsartikel benötigt einen Namen".into(),
            ));
        }
        let connection = self.conn();
        if item.id.is_empty() {
            item.id = new_id("pantry-item");
        }
        let now = now_rfc3339();
        if item.created_at.is_empty() {
            item.created_at = now.clone();
        }
        item.updated_at = now;
        connection.execute(
            "INSERT INTO pantry_items(
                id, name, normalized_name, exclude_from_shopping, note, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(normalized_name) DO UPDATE SET name=excluded.name,
                exclude_from_shopping=excluded.exclude_from_shopping,
                note=excluded.note, updated_at=excluded.updated_at",
            params![
                item.id,
                item.name.trim(),
                normalize_name(&item.name),
                item.exclude_from_shopping as i64,
                item.note,
                item.created_at,
                item.updated_at
            ],
        )?;
        // Return the canonical row in case normalized-name upsert kept an existing id.
        connection
            .query_row(
                "SELECT id, name, exclude_from_shopping, note, created_at, updated_at
             FROM pantry_items WHERE normalized_name=?1",
                [normalize_name(&item.name)],
                |row| {
                    Ok(PantryItem {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        exclude_from_shopping: row.get::<_, i64>(2)? != 0,
                        note: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn delete_pantry_item(&self, id: &str) -> DomainResult<()> {
        let affected = self
            .conn()
            .execute("DELETE FROM pantry_items WHERE id=?1", [id])?;
        if affected == 0 {
            return Err(DomainError::NotFound(format!("Vorratsartikel {id}")));
        }
        Ok(())
    }

    pub fn save_memory(&self, mut memory: Memory) -> DomainResult<Memory> {
        validate_memory(&memory)?;
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        if memory.id.is_empty() {
            memory.id = new_id("memory");
        }
        let before = read_memory(&transaction, &memory.id).ok();
        let now = now_rfc3339();
        if memory.created_at.is_empty() {
            memory.created_at = now.clone();
        }
        memory.updated_at = now;
        write_memory(&transaction, &memory)?;
        record_undo(
            &transaction,
            "memory",
            &memory.id,
            if before.is_some() { "update" } else { "create" },
            before.map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&memory)?),
        )?;
        transaction.commit()?;
        Ok(memory)
    }

    pub fn recall_memories(
        &self,
        query: Option<&str>,
        kind: Option<&str>,
        status: Option<&str>,
        limit: usize,
    ) -> DomainResult<Vec<Memory>> {
        let connection = self.conn();
        let search = format!("%{}%", query.unwrap_or_default().trim());
        let kind = kind.unwrap_or_default();
        let status = status.unwrap_or_default();
        let mut statement = connection.prepare(
            "SELECT id, kind, content, confidence, evidence_json, status, preference_score,
                    source, created_at, updated_at, last_used_at
             FROM memories
             WHERE (?1 = '%%' OR content LIKE ?1 COLLATE NOCASE)
               AND (?2 = '' OR kind = ?2)
               AND (?3 = '' OR status = ?3)
             ORDER BY CASE status WHEN 'confirmed' THEN 0 WHEN 'proposed' THEN 1 ELSE 2 END,
                      confidence DESC, updated_at DESC
             LIMIT ?4",
        )?;
        let rows = statement.query_map(
            params![search, kind, status, limit.clamp(1, 500) as i64],
            row_memory,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn set_memory_status(
        &self,
        id: &str,
        status: &str,
        confidence: Option<f64>,
    ) -> DomainResult<Memory> {
        if !["proposed", "confirmed", "dismissed"].contains(&status) {
            return Err(DomainError::Validation("Unbekannter Memory-Status".into()));
        }
        let mut memory = read_memory(&self.conn(), id)?;
        memory.status = status.to_string();
        if let Some(confidence) = confidence {
            memory.confidence = confidence;
        }
        self.save_memory(memory)
    }

    pub fn delete_memory(&self, id: &str) -> DomainResult<()> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let before = read_memory(&transaction, id)?;
        transaction.execute("DELETE FROM memories WHERE id=?1", [id])?;
        record_undo(
            &transaction,
            "memory",
            id,
            "delete",
            Some(serde_json::to_value(before)?),
            None,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn create_agent_session(
        &self,
        codex_thread_id: Option<String>,
        title: String,
        metadata: Value,
    ) -> DomainResult<AgentSession> {
        let now = now_rfc3339();
        let session = AgentSession {
            id: new_id("session"),
            codex_thread_id,
            title: if title.trim().is_empty() {
                "MealZ Chat".into()
            } else {
                title
            },
            status: "active".into(),
            metadata,
            created_at: now.clone(),
            updated_at: now.clone(),
        };
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        // Exactly one conversation is current. Previous transcripts stay in
        // SQLite as archived history, but cannot accidentally receive new
        // thread IDs or tool activity.
        transaction.execute(
            "UPDATE agent_sessions SET status='archived', updated_at=?1 WHERE status='active'",
            [&now],
        )?;
        transaction.execute(
            "INSERT INTO agent_sessions(id, codex_thread_id, title, status, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                session.id,
                session.codex_thread_id,
                session.title,
                session.status,
                serde_json::to_string(&session.metadata)?,
                session.created_at,
                session.updated_at
            ],
        )?;
        transaction.commit()?;
        Ok(session)
    }

    pub fn upsert_agent_session(&self, session: AgentSession) -> DomainResult<AgentSession> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        if session.status == "active" {
            transaction.execute(
                "UPDATE agent_sessions SET status='archived', updated_at=?2 WHERE status='active' AND id<>?1",
                params![session.id, session.updated_at],
            )?;
        }
        transaction.execute(
            "INSERT INTO agent_sessions(id, codex_thread_id, title, status, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET codex_thread_id=excluded.codex_thread_id,
                 title=excluded.title, status=excluded.status, metadata_json=excluded.metadata_json,
                 updated_at=excluded.updated_at",
            params![
                session.id,
                session.codex_thread_id,
                session.title,
                session.status,
                serde_json::to_string(&session.metadata)?,
                session.created_at,
                session.updated_at
            ],
        )?;
        transaction.commit()?;
        Ok(session)
    }

    /// Lists every durable conversation. The active conversation is always
    /// first, followed by archived conversations in most-recently-used order.
    /// Tool-only timeline rows do not inflate the human message count or
    /// become the preview shown in the conversation picker.
    pub fn list_agent_sessions(&self) -> DomainResult<Vec<AgentSessionSummary>> {
        let connection = self.conn();
        let mut statement = connection.prepare(
            "SELECT s.id, s.codex_thread_id, s.title, s.status, s.metadata_json,
                    s.created_at, s.updated_at,
                    (SELECT COUNT(*) FROM agent_messages m
                     WHERE m.session_id=s.id AND TRIM(m.content)<>''),
                    (SELECT m.content FROM agent_messages m
                     WHERE m.session_id=s.id AND TRIM(m.content)<>''
                     ORDER BY m.created_at DESC, m.rowid DESC LIMIT 1)
             FROM agent_sessions s
             ORDER BY CASE WHEN s.status='active' THEN 0 ELSE 1 END,
                      s.updated_at DESC, s.created_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            let metadata: String = row.get(4)?;
            Ok((
                AgentSession {
                    id: row.get(0)?,
                    codex_thread_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    metadata: serde_json::from_str(&metadata).unwrap_or_else(|_| json!({})),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                },
                row.get::<_, i64>(7)?.max(0) as usize,
                row.get::<_, Option<String>>(8)?,
            ))
        })?;
        rows.map(|row| {
            let (session, message_count, preview) = row?;
            Ok(AgentSessionSummary {
                session,
                message_count,
                preview,
            })
        })
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(DomainError::from)
    }

    /// Makes an existing conversation current without changing its saved
    /// Codex thread id or transcript. The transaction guarantees exactly one
    /// active session at commit time.
    pub fn activate_agent_session(&self, session_id: &str) -> DomainResult<AgentSession> {
        if session_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Gesprächs-ID darf nicht leer sein".into(),
            ));
        }
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM agent_sessions WHERE id=?1)",
            [session_id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(DomainError::NotFound("Agenten-Gespräch".into()));
        }
        let now = now_rfc3339();
        transaction.execute(
            "UPDATE agent_sessions SET status='archived', updated_at=?1 WHERE status='active' AND id<>?2",
            params![now, session_id],
        )?;
        transaction.execute(
            "UPDATE agent_sessions SET status='active', updated_at=?2 WHERE id=?1",
            params![session_id, now],
        )?;
        let session = transaction.query_row(
            "SELECT id, codex_thread_id, title, status, metadata_json, created_at, updated_at
             FROM agent_sessions WHERE id=?1",
            [session_id],
            |row| {
                let metadata: String = row.get(4)?;
                Ok(AgentSession {
                    id: row.get(0)?,
                    codex_thread_id: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    metadata: serde_json::from_str(&metadata).unwrap_or_else(|_| json!({})),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )?;
        transaction.commit()?;
        Ok(session)
    }

    /// Returns the most recently touched active agent session, if one exists.
    /// The Codex host can use this without knowing anything about SQLite.
    pub fn current_agent_session(&self) -> DomainResult<Option<AgentSession>> {
        let connection = self.conn();
        let tuple = connection
            .query_row(
                "SELECT id, codex_thread_id, title, status, metadata_json, created_at, updated_at
                 FROM agent_sessions WHERE status='active' ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()?;
        tuple
            .map(|value| {
                Ok(AgentSession {
                    id: value.0,
                    codex_thread_id: value.1,
                    title: value.2,
                    status: value.3,
                    metadata: serde_json::from_str(&value.4)?,
                    created_at: value.5,
                    updated_at: value.6,
                })
            })
            .transpose()
    }

    /// Persists or clears the Codex thread id on the current session. If there
    /// is no session yet, a default local session is created atomically.
    pub fn set_current_codex_thread_id(
        &self,
        codex_thread_id: Option<String>,
    ) -> DomainResult<AgentSession> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let current = transaction
            .query_row(
                "SELECT id, title, status, metadata_json, created_at
                 FROM agent_sessions WHERE status='active' ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;
        let now = now_rfc3339();
        let session = if let Some(current) = current {
            transaction.execute(
                "UPDATE agent_sessions SET codex_thread_id=?2, updated_at=?3 WHERE id=?1",
                params![current.0, codex_thread_id, now],
            )?;
            AgentSession {
                id: current.0,
                codex_thread_id,
                title: current.1,
                status: current.2,
                metadata: serde_json::from_str(&current.3)?,
                created_at: current.4,
                updated_at: now,
            }
        } else {
            let session = AgentSession {
                id: new_id("session"),
                codex_thread_id,
                title: "MealZ Chat".into(),
                status: "active".into(),
                metadata: json!({}),
                created_at: now.clone(),
                updated_at: now,
            };
            transaction.execute(
                "INSERT INTO agent_sessions(id, codex_thread_id, title, status, metadata_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session.id,
                    session.codex_thread_id,
                    session.title,
                    session.status,
                    serde_json::to_string(&session.metadata)?,
                    session.created_at,
                    session.updated_at
                ],
            )?;
            session
        };
        transaction.commit()?;
        Ok(session)
    }

    pub fn append_agent_message(&self, mut message: AgentMessage) -> DomainResult<AgentMessage> {
        if message.content.trim().is_empty() && message.tool_name.is_none() {
            return Err(DomainError::Validation("Leere Agenten-Nachricht".into()));
        }
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let session_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM agent_sessions WHERE id=?1)",
            [&message.session_id],
            |row| row.get(0),
        )?;
        if !session_exists {
            return Err(DomainError::NotFound("Agenten-Session".into()));
        }
        if message.id.is_empty() {
            message.id = new_id("message");
        }
        if message.created_at.is_empty() {
            message.created_at = now_rfc3339();
        }
        transaction.execute(
            "INSERT INTO agent_messages(
                id, session_id, role, content, item_id, tool_name, tool_payload_json, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                message.id,
                message.session_id,
                message.role,
                message.content,
                message.item_id,
                message.tool_name,
                message
                    .tool_payload
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?,
                message.created_at
            ],
        )?;
        transaction.execute(
            "UPDATE agent_sessions SET updated_at=?2 WHERE id=?1",
            params![message.session_id, message.created_at],
        )?;
        transaction.commit()?;
        Ok(message)
    }

    /// Adds or updates a visible activity in one stable timeline slot. The
    /// caller chooses the slot key so activities can remain chronologically
    /// interleaved with assistant messages after reload.
    pub fn record_agent_turn_activity(
        &self,
        session_id: &str,
        timeline_id: &str,
        activity: Value,
    ) -> DomainResult<AgentMessage> {
        if timeline_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Tool-Aktivität benötigt eine Turn-ID".into(),
            ));
        }
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let existing = transaction
            .query_row(
                "SELECT id, tool_payload_json, created_at FROM agent_messages
                 WHERE session_id=?1 AND item_id=?2 AND tool_name='turn_timeline'
                 ORDER BY created_at DESC LIMIT 1",
                params![session_id, timeline_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        let mut tools = existing
            .as_ref()
            .and_then(|value| value.1.as_deref())
            .and_then(|payload| serde_json::from_str::<Value>(payload).ok())
            .and_then(|payload| payload.get("tools").cloned())
            .and_then(|tools| tools.as_array().cloned())
            .unwrap_or_default();
        let activity_id = activity
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if let Some(position) = tools
            .iter()
            .position(|item| item.get("id").and_then(Value::as_str) == Some(activity_id))
        {
            tools[position] = activity;
        } else {
            tools.push(activity);
        }
        let payload = serde_json::to_string(&json!({"tools":tools}))?;
        let now = now_rfc3339();
        let message = if let Some((id, _, created_at)) = existing {
            transaction.execute(
                "UPDATE agent_messages SET tool_payload_json=?2 WHERE id=?1",
                params![id, payload],
            )?;
            AgentMessage {
                id,
                session_id: session_id.into(),
                role: "assistant".into(),
                content: String::new(),
                item_id: Some(timeline_id.into()),
                tool_name: Some("turn_timeline".into()),
                tool_payload: Some(json!({"tools":tools})),
                created_at,
            }
        } else {
            let message = AgentMessage {
                id: new_id("message"),
                session_id: session_id.into(),
                role: "assistant".into(),
                content: String::new(),
                item_id: Some(timeline_id.into()),
                tool_name: Some("turn_timeline".into()),
                tool_payload: Some(json!({"tools":tools})),
                created_at: now.clone(),
            };
            transaction.execute(
                "INSERT INTO agent_messages(id, session_id, role, content, item_id, tool_name, tool_payload_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    message.id,
                    message.session_id,
                    message.role,
                    message.content,
                    message.item_id,
                    message.tool_name,
                    payload,
                    message.created_at
                ],
            )?;
            message
        };
        transaction.execute(
            "UPDATE agent_sessions SET updated_at=?2 WHERE id=?1",
            params![session_id, now],
        )?;
        transaction.commit()?;
        Ok(message)
    }

    pub fn list_agent_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> DomainResult<Vec<AgentMessage>> {
        let connection = self.conn();
        let mut statement = connection.prepare(
            "SELECT id, session_id, role, content, item_id, tool_name, tool_payload_json, created_at
             FROM (SELECT * FROM agent_messages WHERE session_id=?1 ORDER BY created_at DESC LIMIT ?2)
             ORDER BY created_at ASC",
        )?;
        let rows =
            statement.query_map(params![session_id, limit.clamp(1, 2000) as i64], |row| {
                let payload: Option<String> = row.get(6)?;
                Ok(AgentMessage {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    item_id: row.get(4)?,
                    tool_name: row.get(5)?,
                    tool_payload: payload.and_then(|value| serde_json::from_str(&value).ok()),
                    created_at: row.get(7)?,
                })
            })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn undo_last(&self) -> DomainResult<UndoRecord> {
        let mut connection = self.conn();
        let transaction = connection.transaction()?;
        let record = read_last_undo(&transaction)?;
        apply_undo(&transaction, &record)?;
        let undone_at = now_rfc3339();
        transaction.execute(
            "UPDATE undo_history SET undone=1, undone_at=?2 WHERE id=?1",
            params![record.id, undone_at],
        )?;
        transaction.commit()?;
        drop(connection);
        self.rebuild_all_existing_shopping_lists()?;
        Ok(UndoRecord {
            undone: true,
            undone_at: Some(undone_at),
            ..record
        })
    }

    pub fn list_undo_history(&self, limit: usize) -> DomainResult<Vec<UndoRecord>> {
        let connection = self.conn();
        let mut statement = connection.prepare(
            "SELECT id, entity_kind, entity_id, action, before_json, after_json,
                    undone, created_at, undone_at
             FROM undo_history ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([limit.clamp(1, 500) as i64], row_undo)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

fn validate_profile(profile: &UserProfile) -> DomainResult<()> {
    if profile.name.trim().is_empty() {
        return Err(DomainError::Validation(
            "Profilname darf nicht leer sein".into(),
        ));
    }
    if profile.household_size <= 0 || profile.household_size > 50 {
        return Err(DomainError::Validation(
            "Haushaltsgröße muss zwischen 1 und 50 liegen".into(),
        ));
    }
    if profile.default_servings <= 0.0 || profile.default_servings > 100.0 {
        return Err(DomainError::Validation(
            "Standardportionen sind ungültig".into(),
        ));
    }
    if profile.weekday_max_minutes < 0 || profile.weekend_max_minutes < 0 {
        return Err(DomainError::Validation(
            "Kochzeit darf nicht negativ sein".into(),
        ));
    }
    validate_optional_positive(profile.height_cm, "Größe")?;
    validate_optional_positive(profile.weight_kg, "Gewicht")?;
    validate_optional_positive(profile.manual_calorie_target_kcal, "Manuelles Kalorienziel")?;
    if !["inactive", "low_active", "active", "very_active"]
        .contains(&profile.activity_level.as_str())
    {
        return Err(DomainError::Validation(
            "Aktivitätsniveau muss inactive, low_active, active oder very_active sein".into(),
        ));
    }
    if !["manual", "calculated"].contains(&profile.calorie_target_mode.as_str()) {
        return Err(DomainError::Validation(
            "Kalorienmodus muss manual oder calculated sein".into(),
        ));
    }
    if let Some(sex) = &profile.sex_for_energy
        && !["male", "female"].contains(&sex.as_str())
    {
        return Err(DomainError::Validation(
            "Sex for energy muss male oder female sein".into(),
        ));
    }
    if let Some(date) = &profile.birth_date {
        NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| DomainError::Validation("Geburtsdatum muss YYYY-MM-DD sein".into()))?;
    }
    validate_targets(&profile.nutrition_targets)
}

/// NASEM 2023 Dietary Reference Intakes for Energy, Table S-3. These adult
/// EER equations estimate weight-maintenance energy (kcal/day), not a medical
/// diagnosis or a weight-loss prescription. Inputs are age in years, height
/// in cm and weight in kg. We deliberately require all inputs, so MealZ never
/// silently guesses a personal calorie target.
pub fn calculated_eer_kcal(profile: &UserProfile, today: NaiveDate) -> DomainResult<i64> {
    let birth_date = profile.birth_date.as_deref().ok_or_else(|| {
        DomainError::Validation("Für die Kalorienberechnung wird ein Geburtsdatum benötigt".into())
    })?;
    let birth_date = NaiveDate::parse_from_str(birth_date, "%Y-%m-%d")
        .map_err(|_| DomainError::Validation("Geburtsdatum muss YYYY-MM-DD sein".into()))?;
    let age = today.year()
        - birth_date.year()
        - i32::from((today.month(), today.day()) < (birth_date.month(), birth_date.day()));
    if age < 19 {
        return Err(DomainError::Validation(
            "Die NASEM-2023-Erwachsenenformel gilt erst ab 19 Jahren".into(),
        ));
    }
    let height = profile.height_cm.ok_or_else(|| {
        DomainError::Validation("Für die Kalorienberechnung wird die Größe benötigt".into())
    })?;
    let weight = profile.weight_kg.ok_or_else(|| {
        DomainError::Validation("Für die Kalorienberechnung wird das Gewicht benötigt".into())
    })?;
    let sex = profile.sex_for_energy.as_deref().ok_or_else(|| {
        DomainError::Validation("Für die Kalorienberechnung wird male oder female benötigt".into())
    })?;
    let (intercept, age_factor, height_factor, weight_factor) =
        match (sex, profile.activity_level.as_str()) {
            ("male", "inactive") => (753.07, -10.83, 6.50, 14.10),
            ("male", "low_active") => (581.47, -10.83, 8.30, 14.94),
            ("male", "active") => (1004.82, -10.83, 6.52, 15.91),
            ("male", "very_active") => (-517.88, -10.83, 15.61, 19.11),
            ("female", "inactive") => (584.90, -7.01, 5.72, 11.71),
            ("female", "low_active") => (575.77, -7.01, 6.60, 12.14),
            ("female", "active") => (710.25, -7.01, 6.54, 12.34),
            ("female", "very_active") => (511.83, -7.01, 9.07, 12.56),
            _ => unreachable!("profile validation precedes EER calculation"),
        };
    Ok(
        (intercept + age_factor * f64::from(age) + height_factor * height + weight_factor * weight)
            .round() as i64,
    )
}

fn apply_calorie_target_mode(profile: &mut UserProfile) -> DomainResult<()> {
    if profile.calorie_target_mode == "calculated" {
        profile.nutrition_targets.calories_kcal =
            Some(calculated_eer_kcal(profile, Local::now().date_naive())? as f64);
    } else {
        if profile.manual_calorie_target_kcal.is_none() {
            profile.manual_calorie_target_kcal = profile.nutrition_targets.calories_kcal;
        }
        profile.nutrition_targets.calories_kcal = profile.manual_calorie_target_kcal;
    }
    Ok(())
}

fn validate_targets(targets: &NutritionTargets) -> DomainResult<()> {
    for (label, value) in [
        ("Kalorien", targets.calories_kcal),
        ("Protein", targets.protein_g),
        ("Kohlenhydrate", targets.carbs_g),
        ("Fett", targets.fat_g),
        ("Ballaststoffe", targets.fiber_g),
    ] {
        validate_optional_positive(value, label)?;
    }
    Ok(())
}

fn validate_optional_positive(value: Option<f64>, label: &str) -> DomainResult<()> {
    if value.is_some_and(|value| !value.is_finite() || value < 0.0) {
        return Err(DomainError::Validation(format!(
            "{label} muss positiv sein"
        )));
    }
    Ok(())
}

fn read_profile(conn: &Connection) -> DomainResult<UserProfile> {
    let tuple = conn
        .query_row(
            "SELECT id, name, locale, timezone, household_size, height_cm, weight_kg,
                    birth_date, sex_for_energy, activity_level, calorie_target_mode,
                    manual_calorie_target_kcal, dietary_style, nutrition_targets_json, weekday_max_minutes,
                    weekend_max_minutes, default_servings, notes, created_at, updated_at
             FROM profile LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<f64>>(5)?,
                    row.get::<_, Option<f64>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<f64>>(11)?,
                    row.get::<_, Option<String>>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, i64>(15)?,
                    row.get::<_, f64>(16)?,
                    row.get::<_, Option<String>>(17)?,
                    row.get::<_, String>(18)?,
                    row.get::<_, String>(19)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| DomainError::NotFound("Profil".into()))?;
    Ok(UserProfile {
        id: tuple.0,
        name: tuple.1,
        locale: tuple.2,
        timezone: tuple.3,
        household_size: tuple.4,
        height_cm: tuple.5,
        weight_kg: tuple.6,
        birth_date: tuple.7,
        sex_for_energy: tuple.8,
        activity_level: tuple.9,
        calorie_target_mode: tuple.10,
        manual_calorie_target_kcal: tuple.11,
        dietary_style: tuple.12,
        nutrition_targets: serde_json::from_str(&tuple.13)?,
        weekday_max_minutes: tuple.14,
        weekend_max_minutes: tuple.15,
        default_servings: tuple.16,
        notes: tuple.17,
        created_at: tuple.18,
        updated_at: tuple.19,
    })
}

fn write_profile(conn: &Connection, profile: &UserProfile) -> DomainResult<()> {
    conn.execute(
        "INSERT INTO profile(
            id, name, locale, timezone, household_size, height_cm, weight_kg, birth_date,
            sex_for_energy, activity_level, calorie_target_mode, manual_calorie_target_kcal,
            dietary_style, nutrition_targets_json, weekday_max_minutes, weekend_max_minutes,
            default_servings, notes, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
         ON CONFLICT(id) DO UPDATE SET name=excluded.name, locale=excluded.locale,
             timezone=excluded.timezone, household_size=excluded.household_size,
             height_cm=excluded.height_cm, weight_kg=excluded.weight_kg,
             birth_date=excluded.birth_date, sex_for_energy=excluded.sex_for_energy,
             activity_level=excluded.activity_level, calorie_target_mode=excluded.calorie_target_mode,
             manual_calorie_target_kcal=excluded.manual_calorie_target_kcal,
             dietary_style=excluded.dietary_style,
             nutrition_targets_json=excluded.nutrition_targets_json,
             weekday_max_minutes=excluded.weekday_max_minutes,
             weekend_max_minutes=excluded.weekend_max_minutes,
             default_servings=excluded.default_servings, notes=excluded.notes,
             updated_at=excluded.updated_at",
        params![
            profile.id,
            profile.name,
            profile.locale,
            profile.timezone,
            profile.household_size,
            profile.height_cm,
            profile.weight_kg,
            profile.birth_date,
            profile.sex_for_energy,
            profile.activity_level,
            profile.calorie_target_mode,
            profile.manual_calorie_target_kcal,
            profile.dietary_style,
            serde_json::to_string(&profile.nutrition_targets)?,
            profile.weekday_max_minutes,
            profile.weekend_max_minutes,
            profile.default_servings,
            profile.notes,
            profile.created_at,
            profile.updated_at
        ],
    )?;
    Ok(())
}

fn read_equipment(conn: &Connection, id: &str) -> DomainResult<Equipment> {
    conn.query_row(
        "SELECT id, name, category, available, notes, created_at, updated_at
         FROM equipment WHERE id=?1",
        [id],
        |row| {
            Ok(Equipment {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                available: row.get::<_, i64>(3)? != 0,
                notes: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| DomainError::NotFound(format!("Equipment {id}")))
}

fn validate_recipe(recipe: &Recipe) -> DomainResult<()> {
    if recipe.title.trim().is_empty() {
        return Err(DomainError::Validation(
            "Rezept benötigt einen Titel".into(),
        ));
    }
    if recipe.servings <= 0.0 || !recipe.servings.is_finite() {
        return Err(DomainError::Validation(
            "Rezeptportionen müssen positiv sein".into(),
        ));
    }
    if recipe.prep_minutes < 0 || recipe.cook_minutes < 0 {
        return Err(DomainError::Validation(
            "Rezeptzeiten dürfen nicht negativ sein".into(),
        ));
    }
    if !(0.0..=1.0).contains(&recipe.confidence) {
        return Err(DomainError::Validation(
            "Rezept-Confidence muss zwischen 0 und 1 liegen".into(),
        ));
    }
    if recipe.ingredients.is_empty() {
        return Err(DomainError::Validation(
            "Rezept benötigt mindestens eine Zutat".into(),
        ));
    }
    if recipe.steps.is_empty() {
        return Err(DomainError::Validation(
            "Rezept benötigt mindestens einen Schritt".into(),
        ));
    }
    for ingredient in &recipe.ingredients {
        if ingredient.name.trim().is_empty()
            || ingredient.quantity < 0.0
            || !ingredient.quantity.is_finite()
        {
            return Err(DomainError::Validation(
                "Zutat hat ungültigen Namen oder Menge".into(),
            ));
        }
    }
    for step in &recipe.steps {
        if step.instruction.trim().is_empty() || step.timer_minutes.is_some_and(|value| value < 0) {
            return Err(DomainError::Validation("Rezeptschritt ist ungültig".into()));
        }
    }
    Ok(())
}

fn save_recipe_tx(
    transaction: &Transaction<'_>,
    mut recipe: Recipe,
    create_undo: bool,
) -> DomainResult<Recipe> {
    validate_recipe(&recipe)?;
    if recipe.id.is_empty() {
        recipe.id = new_id("recipe");
    }
    let before = if transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM recipes WHERE id=?1)",
        [&recipe.id],
        |row| row.get::<_, bool>(0),
    )? {
        Some(read_recipe(transaction, &recipe.id)?)
    } else {
        None
    };
    let now = now_rfc3339();
    if recipe.created_at.is_empty() {
        recipe.created_at = before
            .as_ref()
            .map(|value| value.created_at.clone())
            .unwrap_or_else(|| now.clone());
    }
    recipe.updated_at = now;
    for (position, ingredient) in recipe.ingredients.iter_mut().enumerate() {
        if ingredient.id.is_empty() {
            ingredient.id = new_id("ingredient");
        }
        ingredient.recipe_id = recipe.id.clone();
        ingredient.position = position as i64;
        if ingredient.aisle.trim().is_empty() {
            ingredient.aisle = "Sonstiges".into();
        }
    }
    for (position, step) in recipe.steps.iter_mut().enumerate() {
        if step.id.is_empty() {
            step.id = new_id("step");
        }
        step.recipe_id = recipe.id.clone();
        step.position = position as i64;
    }
    for source in &mut recipe.sources {
        if source.id.is_empty() {
            source.id = new_id("source");
        }
        source.recipe_id = recipe.id.clone();
        if source.source_type.trim().is_empty() {
            source.source_type = "web".into();
        }
    }
    for (position, image) in recipe.images.iter_mut().enumerate() {
        if image.id.is_empty() {
            image.id = new_id("image");
        }
        image.recipe_id = recipe.id.clone();
        image.position = position as i64;
        if image.kind.trim().is_empty() {
            image.kind = "remote".into();
        }
    }
    let mut calculated = Nutrition::default();
    for ingredient in &recipe.ingredients {
        calculated.add_scaled(&ingredient.nutrition, 1.0);
    }
    let has_calculated_nutrition = [
        calculated.calories_kcal,
        calculated.protein_g,
        calculated.carbs_g,
        calculated.fat_g,
        calculated.fiber_g,
    ]
    .iter()
    .any(|value| *value > 0.0);
    if has_calculated_nutrition {
        recipe.nutrition_total = calculated;
    }
    recipe.nutrition_per_serving = recipe.nutrition_total.divided_by(recipe.servings);

    transaction.execute(
        "INSERT INTO recipes(
            id, title, summary, servings, prep_minutes, cook_minutes, difficulty, cuisine,
            meal_types_json, tags_json, favorite, archived, source_kind, confidence,
            calories_kcal, protein_g, carbs_g, fat_g, fiber_g, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                   ?15, ?16, ?17, ?18, ?19, ?20, ?21)
         ON CONFLICT(id) DO UPDATE SET title=excluded.title, summary=excluded.summary,
             servings=excluded.servings, prep_minutes=excluded.prep_minutes,
             cook_minutes=excluded.cook_minutes, difficulty=excluded.difficulty,
             cuisine=excluded.cuisine, meal_types_json=excluded.meal_types_json,
             tags_json=excluded.tags_json, favorite=excluded.favorite, archived=excluded.archived,
             source_kind=excluded.source_kind, confidence=excluded.confidence,
             calories_kcal=excluded.calories_kcal, protein_g=excluded.protein_g,
             carbs_g=excluded.carbs_g, fat_g=excluded.fat_g, fiber_g=excluded.fiber_g,
             updated_at=excluded.updated_at",
        params![
            recipe.id,
            recipe.title.trim(),
            recipe.summary,
            recipe.servings,
            recipe.prep_minutes,
            recipe.cook_minutes,
            if recipe.difficulty.is_empty() {
                "easy"
            } else {
                &recipe.difficulty
            },
            recipe.cuisine,
            serde_json::to_string(&recipe.meal_types)?,
            serde_json::to_string(&recipe.tags)?,
            recipe.favorite as i64,
            recipe.archived as i64,
            if recipe.source_kind.is_empty() {
                "generated"
            } else {
                &recipe.source_kind
            },
            recipe.confidence,
            recipe.nutrition_total.calories_kcal,
            recipe.nutrition_total.protein_g,
            recipe.nutrition_total.carbs_g,
            recipe.nutrition_total.fat_g,
            recipe.nutrition_total.fiber_g,
            recipe.created_at,
            recipe.updated_at
        ],
    )?;
    transaction.execute(
        "DELETE FROM recipe_ingredients WHERE recipe_id=?1",
        [&recipe.id],
    )?;
    transaction.execute("DELETE FROM recipe_steps WHERE recipe_id=?1", [&recipe.id])?;
    transaction.execute(
        "DELETE FROM recipe_sources WHERE recipe_id=?1",
        [&recipe.id],
    )?;
    transaction.execute("DELETE FROM recipe_images WHERE recipe_id=?1", [&recipe.id])?;
    for ingredient in &recipe.ingredients {
        transaction.execute(
            "INSERT INTO recipe_ingredients(
                id, recipe_id, name, quantity, unit, aisle, preparation, optional,
                calories_kcal, protein_g, carbs_g, fat_g, fiber_g, position
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                ingredient.id,
                ingredient.recipe_id,
                ingredient.name,
                ingredient.quantity,
                ingredient.unit,
                ingredient.aisle,
                ingredient.preparation,
                ingredient.optional as i64,
                ingredient.nutrition.calories_kcal,
                ingredient.nutrition.protein_g,
                ingredient.nutrition.carbs_g,
                ingredient.nutrition.fat_g,
                ingredient.nutrition.fiber_g,
                ingredient.position
            ],
        )?;
    }
    for step in &recipe.steps {
        transaction.execute(
            "INSERT INTO recipe_steps(id, recipe_id, position, instruction, timer_minutes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                step.id,
                step.recipe_id,
                step.position,
                step.instruction,
                step.timer_minutes
            ],
        )?;
    }
    for source in &recipe.sources {
        transaction.execute(
            "INSERT INTO recipe_sources(id, recipe_id, title, url, publisher, source_type, accessed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                source.id,
                source.recipe_id,
                source.title,
                source.url,
                source.publisher,
                source.source_type,
                source.accessed_at
            ],
        )?;
    }
    for image in &recipe.images {
        transaction.execute(
            "INSERT INTO recipe_images(id, recipe_id, url, kind, alt_text, attribution, position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                image.id,
                image.recipe_id,
                image.url,
                image.kind,
                image.alt_text,
                image.attribution,
                image.position
            ],
        )?;
    }
    if create_undo {
        record_undo(
            transaction,
            "recipe",
            &recipe.id,
            if before.is_some() { "update" } else { "create" },
            before.map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&recipe)?),
        )?;
    }
    Ok(recipe)
}

fn read_recipe(conn: &Connection, id: &str) -> DomainResult<Recipe> {
    let base = conn
        .query_row(
            "SELECT id, title, summary, servings, prep_minutes, cook_minutes, difficulty,
                    cuisine, meal_types_json, tags_json, favorite, archived, source_kind,
                    confidence, calories_kcal, protein_g, carbs_g, fat_g, fiber_g,
                    created_at, updated_at
             FROM recipes WHERE id=?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)? != 0,
                    row.get::<_, i64>(11)? != 0,
                    row.get::<_, String>(12)?,
                    row.get::<_, f64>(13)?,
                    Nutrition {
                        calories_kcal: row.get(14)?,
                        protein_g: row.get(15)?,
                        carbs_g: row.get(16)?,
                        fat_g: row.get(17)?,
                        fiber_g: row.get(18)?,
                    },
                    row.get::<_, String>(19)?,
                    row.get::<_, String>(20)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| DomainError::NotFound(format!("Rezept {id}")))?;

    let mut ingredient_statement = conn.prepare(
        "SELECT id, recipe_id, name, quantity, unit, aisle, preparation, optional,
                calories_kcal, protein_g, carbs_g, fat_g, fiber_g, position
         FROM recipe_ingredients WHERE recipe_id=?1 ORDER BY position",
    )?;
    let ingredients = ingredient_statement
        .query_map([id], |row| {
            Ok(RecipeIngredient {
                id: row.get(0)?,
                recipe_id: row.get(1)?,
                name: row.get(2)?,
                quantity: row.get(3)?,
                unit: row.get(4)?,
                aisle: row.get(5)?,
                preparation: row.get(6)?,
                optional: row.get::<_, i64>(7)? != 0,
                nutrition: Nutrition {
                    calories_kcal: row.get(8)?,
                    protein_g: row.get(9)?,
                    carbs_g: row.get(10)?,
                    fat_g: row.get(11)?,
                    fiber_g: row.get(12)?,
                },
                position: row.get(13)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut step_statement = conn.prepare(
        "SELECT id, recipe_id, position, instruction, timer_minutes
         FROM recipe_steps WHERE recipe_id=?1 ORDER BY position",
    )?;
    let steps = step_statement
        .query_map([id], |row| {
            Ok(RecipeStep {
                id: row.get(0)?,
                recipe_id: row.get(1)?,
                position: row.get(2)?,
                instruction: row.get(3)?,
                timer_minutes: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut source_statement = conn.prepare(
        "SELECT id, recipe_id, title, url, publisher, source_type, accessed_at
         FROM recipe_sources WHERE recipe_id=?1 ORDER BY rowid",
    )?;
    let sources = source_statement
        .query_map([id], |row| {
            Ok(RecipeSource {
                id: row.get(0)?,
                recipe_id: row.get(1)?,
                title: row.get(2)?,
                url: row.get(3)?,
                publisher: row.get(4)?,
                source_type: row.get(5)?,
                accessed_at: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut image_statement = conn.prepare(
        "SELECT id, recipe_id, url, kind, alt_text, attribution, position
         FROM recipe_images WHERE recipe_id=?1 ORDER BY position",
    )?;
    let images = image_statement
        .query_map([id], |row| {
            Ok(RecipeImage {
                id: row.get(0)?,
                recipe_id: row.get(1)?,
                url: row.get(2)?,
                kind: row.get(3)?,
                alt_text: row.get(4)?,
                attribution: row.get(5)?,
                position: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(Recipe {
        id: base.0,
        title: base.1,
        summary: base.2,
        servings: base.3,
        prep_minutes: base.4,
        cook_minutes: base.5,
        difficulty: base.6,
        cuisine: base.7,
        meal_types: serde_json::from_str(&base.8)?,
        tags: serde_json::from_str(&base.9)?,
        favorite: base.10,
        archived: base.11,
        source_kind: base.12,
        confidence: base.13,
        ingredients,
        steps,
        sources,
        images,
        nutrition_per_serving: base.14.divided_by(base.3),
        nutrition_total: base.14,
        created_at: base.15,
        updated_at: base.16,
    })
}

fn read_recipe_summary(conn: &Connection, id: &str) -> DomainResult<RecipeSummary> {
    let recipe = read_recipe(conn, id)?;
    let (average_rating, last_cooked_at) = conn.query_row(
        "SELECT AVG(score), MAX(cooked_at) FROM recipe_ratings WHERE recipe_id=?1",
        [id],
        |row| {
            Ok((
                row.get::<_, Option<f64>>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        },
    )?;
    Ok(RecipeSummary {
        id: recipe.id,
        title: recipe.title,
        summary: recipe.summary,
        servings: recipe.servings,
        total_minutes: recipe.prep_minutes + recipe.cook_minutes,
        cuisine: recipe.cuisine,
        meal_types: recipe.meal_types,
        tags: recipe.tags,
        favorite: recipe.favorite,
        archived: recipe.archived,
        image_url: recipe.images.first().map(|image| image.url.clone()),
        average_rating,
        last_cooked_at,
        nutrition_per_serving: recipe.nutrition_per_serving,
    })
}

fn ensure_recipe_exists(conn: &Connection, id: &str) -> DomainResult<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM recipes WHERE id=?1)",
        [id],
        |row| row.get(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(DomainError::NotFound(format!("Rezept {id}")))
    }
}

fn planned_date_bounds_for_recipe(
    conn: &Connection,
    recipe_id: &str,
) -> DomainResult<Option<(NaiveDate, NaiveDate)>> {
    let (start, end) = conn.query_row(
        "SELECT MIN(date), MAX(date) FROM plan_entries WHERE recipe_id=?1",
        [recipe_id],
        |row| {
            Ok((
                row.get::<_, Option<NaiveDate>>(0)?,
                row.get::<_, Option<NaiveDate>>(1)?,
            ))
        },
    )?;
    Ok(start.zip(end))
}

fn row_rating(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecipeRating> {
    Ok(RecipeRating {
        id: row.get(0)?,
        recipe_id: row.get(1)?,
        score: row.get(2)?,
        comment: row.get(3)?,
        cooked_at: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn read_rating(conn: &Connection, id: &str) -> DomainResult<RecipeRating> {
    conn.query_row(
        "SELECT id, recipe_id, score, comment, cooked_at, created_at, updated_at
         FROM recipe_ratings WHERE id=?1",
        [id],
        row_rating,
    )
    .optional()?
    .ok_or_else(|| DomainError::NotFound(format!("Bewertung {id}")))
}

fn validate_plan_entry(entry: &PlanEntry) -> DomainResult<()> {
    const SLOTS: &[&str] = &[
        "breakfast",
        "lunch",
        "dinner",
        "snack",
        "shake",
        "dessert",
        "other",
    ];
    const STATUSES: &[&str] = &[
        "planned",
        "prepared",
        "cooked",
        "leftovers",
        "skipped",
        "eating_out",
        "cancelled",
    ];
    if !SLOTS.contains(&entry.slot.as_str()) {
        return Err(DomainError::Validation(format!(
            "Unbekannter Meal-Slot: {}",
            entry.slot
        )));
    }
    if !STATUSES.contains(&entry.status.as_str()) {
        return Err(DomainError::Validation(format!(
            "Unbekannter Planstatus: {}",
            entry.status
        )));
    }
    if entry.servings <= 0.0 || !entry.servings.is_finite() {
        return Err(DomainError::Validation(
            "Geplante Portionen müssen positiv sein".into(),
        ));
    }
    if entry.recipe_id.is_none()
        && entry
            .title_override
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        return Err(DomainError::Validation(
            "Plan-Eintrag benötigt ein Rezept oder einen eigenen Titel".into(),
        ));
    }
    Ok(())
}

fn save_plan_entry_tx(
    transaction: &Transaction<'_>,
    mut entry: PlanEntry,
    create_undo: bool,
) -> DomainResult<PlanEntry> {
    validate_plan_entry(&entry)?;
    if let Some(recipe_id) = &entry.recipe_id {
        ensure_recipe_exists(transaction, recipe_id)?;
    }
    if entry.id.is_empty() {
        entry.id = transaction
            .query_row(
                "SELECT id FROM plan_entries WHERE date=?1 AND slot=?2 AND sort_order=?3",
                params![entry.date.to_string(), entry.slot, entry.sort_order],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_else(|| new_id("plan-entry"));
    }
    let before = read_plan_entry(transaction, &entry.id).ok();
    let now = now_rfc3339();
    if entry.created_at.is_empty() {
        entry.created_at = before
            .as_ref()
            .map(|value| value.created_at.clone())
            .unwrap_or_else(|| now.clone());
    }
    entry.updated_at = now;
    transaction.execute(
        "INSERT INTO plan_entries(
            id, date, slot, recipe_id, title_override, servings, status, notes,
            sort_order, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET date=excluded.date, slot=excluded.slot,
             recipe_id=excluded.recipe_id, title_override=excluded.title_override,
             servings=excluded.servings, status=excluded.status, notes=excluded.notes,
             sort_order=excluded.sort_order, updated_at=excluded.updated_at",
        params![
            entry.id,
            entry.date.to_string(),
            entry.slot,
            entry.recipe_id,
            entry.title_override,
            entry.servings,
            entry.status,
            entry.notes,
            entry.sort_order,
            entry.created_at,
            entry.updated_at
        ],
    )?;
    if create_undo {
        record_undo(
            transaction,
            "plan_entry",
            &entry.id,
            if before.is_some() { "update" } else { "create" },
            before.map(serde_json::to_value).transpose()?,
            Some(serde_json::to_value(&entry)?),
        )?;
    }
    Ok(entry)
}

fn read_plan_entry(conn: &Connection, id: &str) -> DomainResult<PlanEntry> {
    conn.query_row(
        "SELECT id, date, slot, recipe_id, title_override, servings, status, notes,
                sort_order, created_at, updated_at
         FROM plan_entries WHERE id=?1",
        [id],
        row_plan_entry,
    )
    .optional()?
    .ok_or_else(|| DomainError::NotFound(format!("Plan-Eintrag {id}")))
}

fn row_plan_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<PlanEntry> {
    Ok(PlanEntry {
        id: row.get(0)?,
        date: row.get(1)?,
        slot: row.get(2)?,
        recipe_id: row.get(3)?,
        title_override: row.get(4)?,
        servings: row.get(5)?,
        status: row.get(6)?,
        notes: row.get(7)?,
        sort_order: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn read_plan_entries(
    conn: &Connection,
    start: NaiveDate,
    end: NaiveDate,
) -> DomainResult<Vec<PlanEntry>> {
    let mut statement = conn.prepare(
        "SELECT id, date, slot, recipe_id, title_override, servings, status, notes,
                sort_order, created_at, updated_at
         FROM plan_entries WHERE date BETWEEN ?1 AND ?2
         ORDER BY date, sort_order, slot",
    )?;
    let rows = statement.query_map(params![start.to_string(), end.to_string()], row_plan_entry)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn read_plan_entries_for_recipe(
    conn: &Connection,
    recipe_id: &str,
) -> DomainResult<Vec<PlanEntry>> {
    let mut statement = conn.prepare(
        "SELECT id, date, slot, recipe_id, title_override, servings, status, notes,
                sort_order, created_at, updated_at
         FROM plan_entries WHERE recipe_id=?1 ORDER BY date, sort_order, slot",
    )?;
    let rows = statement.query_map([recipe_id], row_plan_entry)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn build_week(
    conn: &Connection,
    week_start: NaiveDate,
    entries: Vec<PlanEntry>,
) -> DomainResult<WeekPlan> {
    let mut week_nutrition = Nutrition::default();
    let mut days = Vec::with_capacity(7);
    for offset in 0..7 {
        let date = week_start + Duration::days(offset);
        let day_entries: Vec<_> = entries
            .iter()
            .filter(|entry| entry.date == date)
            .cloned()
            .collect();
        let mut day_nutrition = Nutrition::default();
        for entry in &day_entries {
            if ["skipped", "eating_out", "cancelled"].contains(&entry.status.as_str()) {
                continue;
            }
            if let Some(recipe_id) = &entry.recipe_id {
                let recipe = read_recipe(conn, recipe_id)?;
                day_nutrition.add_scaled(&recipe.nutrition_per_serving, entry.servings);
            }
        }
        week_nutrition.add_scaled(&day_nutrition, 1.0);
        days.push(DayPlan {
            date,
            entries: day_entries,
            nutrition: day_nutrition,
        });
    }
    Ok(WeekPlan {
        week_start,
        week_end: week_start + Duration::days(6),
        days,
        nutrition: week_nutrition,
    })
}

fn normalize_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .replace(['-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn canonical_quantity(quantity: f64, unit: &str) -> (f64, String) {
    match unit.trim().to_lowercase().as_str() {
        "kg" | "kilogramm" | "kilogram" => (quantity * 1000.0, "g".into()),
        "g" | "gramm" | "gram" => (quantity, "g".into()),
        "l" | "liter" => (quantity * 1000.0, "ml".into()),
        "ml" | "milliliter" => (quantity, "ml".into()),
        "stück" | "stueck" | "stk" | "pcs" | "piece" | "pieces" => (quantity, "Stück".into()),
        "el" | "esslöffel" | "essloeffel" | "tbsp" => (quantity, "EL".into()),
        "tl" | "teelöffel" | "teeloeffel" | "tsp" => (quantity, "TL".into()),
        "prise" | "prisen" => (quantity, "Prise".into()),
        "" => (quantity, "Stück".into()),
        other => (quantity, other.to_string()),
    }
}

fn round_quantity(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn read_shopping_items(conn: &Connection, list_id: &str) -> DomainResult<Vec<ShoppingItem>> {
    let mut statement = conn.prepare(
        "SELECT id, list_id, name, quantity, unit, aisle, checked, manual,
                recipe_ids_json, note, created_at, updated_at
         FROM shopping_items WHERE list_id=?1
         ORDER BY checked, aisle COLLATE NOCASE, name COLLATE NOCASE",
    )?;
    let rows = statement.query_map([list_id], row_shopping_item)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn row_shopping_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<ShoppingItem> {
    let recipe_ids_json: String = row.get(8)?;
    Ok(ShoppingItem {
        id: row.get(0)?,
        list_id: row.get(1)?,
        name: row.get(2)?,
        quantity: row.get(3)?,
        unit: row.get(4)?,
        aisle: row.get(5)?,
        checked: row.get::<_, i64>(6)? != 0,
        manual: row.get::<_, i64>(7)? != 0,
        recipe_ids: serde_json::from_str(&recipe_ids_json).unwrap_or_default(),
        note: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn read_shopping_item(conn: &Connection, id: &str) -> DomainResult<ShoppingItem> {
    conn.query_row(
        "SELECT id, list_id, name, quantity, unit, aisle, checked, manual,
                recipe_ids_json, note, created_at, updated_at
         FROM shopping_items WHERE id=?1",
        [id],
        row_shopping_item,
    )
    .optional()?
    .ok_or_else(|| DomainError::NotFound(format!("Einkaufsartikel {id}")))
}

fn write_shopping_item(conn: &Connection, item: &ShoppingItem) -> DomainResult<()> {
    conn.execute(
        "INSERT INTO shopping_items(
            id, list_id, name, normalized_name, quantity, unit, aisle, checked,
            manual, recipe_ids_json, note, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(id) DO UPDATE SET name=excluded.name,
             normalized_name=excluded.normalized_name, quantity=excluded.quantity,
             unit=excluded.unit, aisle=excluded.aisle, checked=excluded.checked,
             manual=excluded.manual, recipe_ids_json=excluded.recipe_ids_json,
             note=excluded.note, updated_at=excluded.updated_at",
        params![
            item.id,
            item.list_id,
            item.name,
            normalize_name(&item.name),
            item.quantity,
            item.unit,
            item.aisle,
            item.checked as i64,
            item.manual as i64,
            serde_json::to_string(&item.recipe_ids)?,
            item.note,
            item.created_at,
            item.updated_at
        ],
    )?;
    Ok(())
}

fn validate_memory(memory: &Memory) -> DomainResult<()> {
    if memory.content.trim().is_empty() {
        return Err(DomainError::Validation(
            "Memory-Inhalt darf nicht leer sein".into(),
        ));
    }
    if ![
        "preference",
        "dislike",
        "allergy",
        "routine",
        "constraint",
        "equipment",
        "goal",
        "observation",
    ]
    .contains(&memory.kind.as_str())
    {
        return Err(DomainError::Validation(format!(
            "Unbekannter Memory-Typ: {}",
            memory.kind
        )));
    }
    if !(0.0..=1.0).contains(&memory.confidence) {
        return Err(DomainError::Validation(
            "Memory-Confidence muss zwischen 0 und 1 liegen".into(),
        ));
    }
    if memory
        .preference_score
        .is_some_and(|value| !(1.0..=10.0).contains(&value))
    {
        return Err(DomainError::Validation(
            "Vorliebenwert muss zwischen 1 und 10 liegen".into(),
        ));
    }
    if !["proposed", "confirmed", "dismissed"].contains(&memory.status.as_str()) {
        return Err(DomainError::Validation("Unbekannter Memory-Status".into()));
    }
    Ok(())
}

fn write_memory(conn: &Connection, memory: &Memory) -> DomainResult<()> {
    conn.execute(
        "INSERT INTO memories(
            id, kind, content, confidence, evidence_json, status, preference_score,
            source, created_at, updated_at, last_used_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(id) DO UPDATE SET kind=excluded.kind, content=excluded.content,
             confidence=excluded.confidence, evidence_json=excluded.evidence_json,
             status=excluded.status, preference_score=excluded.preference_score,
             source=excluded.source, updated_at=excluded.updated_at,
             last_used_at=excluded.last_used_at",
        params![
            memory.id,
            memory.kind,
            memory.content,
            memory.confidence,
            serde_json::to_string(&memory.evidence)?,
            memory.status,
            memory.preference_score,
            memory.source,
            memory.created_at,
            memory.updated_at,
            memory.last_used_at
        ],
    )?;
    Ok(())
}

fn read_memory(conn: &Connection, id: &str) -> DomainResult<Memory> {
    conn.query_row(
        "SELECT id, kind, content, confidence, evidence_json, status, preference_score,
                source, created_at, updated_at, last_used_at
         FROM memories WHERE id=?1",
        [id],
        row_memory,
    )
    .optional()?
    .ok_or_else(|| DomainError::NotFound(format!("Memory {id}")))
}

fn row_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    let evidence: String = row.get(4)?;
    Ok(Memory {
        id: row.get(0)?,
        kind: row.get(1)?,
        content: row.get(2)?,
        confidence: row.get(3)?,
        evidence: serde_json::from_str(&evidence).unwrap_or_default(),
        status: row.get(5)?,
        preference_score: row.get(6)?,
        source: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        last_used_at: row.get(10)?,
    })
}

fn record_undo(
    conn: &Connection,
    entity_kind: &str,
    entity_id: &str,
    action: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> DomainResult<()> {
    conn.execute(
        "INSERT INTO undo_history(
            id, entity_kind, entity_id, action, before_json, after_json, undone, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
        params![
            new_id("undo"),
            entity_kind,
            entity_id,
            action,
            before.as_ref().map(serde_json::to_string).transpose()?,
            after.as_ref().map(serde_json::to_string).transpose()?,
            now_rfc3339()
        ],
    )?;
    Ok(())
}

fn row_undo(row: &rusqlite::Row<'_>) -> rusqlite::Result<UndoRecord> {
    let before: Option<String> = row.get(4)?;
    let after: Option<String> = row.get(5)?;
    Ok(UndoRecord {
        id: row.get(0)?,
        entity_kind: row.get(1)?,
        entity_id: row.get(2)?,
        action: row.get(3)?,
        before: before.and_then(|value| serde_json::from_str(&value).ok()),
        after: after.and_then(|value| serde_json::from_str(&value).ok()),
        undone: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
        undone_at: row.get(8)?,
    })
}

fn read_last_undo(conn: &Connection) -> DomainResult<UndoRecord> {
    conn.query_row(
        "SELECT id, entity_kind, entity_id, action, before_json, after_json,
                undone, created_at, undone_at
         FROM undo_history WHERE undone=0 ORDER BY rowid DESC LIMIT 1",
        [],
        row_undo,
    )
    .optional()?
    .ok_or_else(|| DomainError::NotFound("Keine Änderung zum Rückgängigmachen".into()))
}

fn apply_undo(transaction: &Transaction<'_>, record: &UndoRecord) -> DomainResult<()> {
    match record.entity_kind.as_str() {
        "profile" => {
            let profile: UserProfile = from_before(record)?;
            write_profile(transaction, &profile)?;
        }
        "equipment" => restore_or_delete(transaction, record, "equipment", |conn, value| {
            let item: Equipment = serde_json::from_value(value)?;
            conn.execute(
                    "INSERT INTO equipment(id, name, category, available, notes, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(id) DO UPDATE SET name=excluded.name, category=excluded.category,
                         available=excluded.available, notes=excluded.notes, updated_at=excluded.updated_at",
                    params![item.id, item.name, item.category, item.available as i64, item.notes, item.created_at, item.updated_at],
                )?;
            Ok(())
        })?,
        "recipe" => {
            if let Some(before) = record.before.clone() {
                let recipe: Recipe = serde_json::from_value(before)?;
                save_recipe_tx(transaction, recipe, false)?;
            } else {
                transaction.execute("DELETE FROM recipes WHERE id=?1", [&record.entity_id])?;
            }
        }
        "recipe_with_plan" => {
            let snapshot = record
                .before
                .clone()
                .ok_or_else(|| DomainError::Validation("Undo-Snapshot fehlt".into()))?;
            let recipe: Recipe = serde_json::from_value(
                snapshot
                    .get("recipe")
                    .cloned()
                    .ok_or_else(|| DomainError::Validation("Rezept-Snapshot fehlt".into()))?,
            )?;
            let entries: Vec<PlanEntry> = serde_json::from_value(
                snapshot
                    .get("planEntries")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
            )?;
            save_recipe_tx(transaction, recipe, false)?;
            for entry in entries {
                save_plan_entry_tx(transaction, entry, false)?;
            }
        }
        "rating" => restore_or_delete(transaction, record, "recipe_ratings", |conn, value| {
            let rating: RecipeRating = serde_json::from_value(value)?;
            conn.execute(
                "INSERT INTO recipe_ratings(id, recipe_id, score, comment, cooked_at, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET score=excluded.score, comment=excluded.comment,
                    cooked_at=excluded.cooked_at, updated_at=excluded.updated_at",
                params![rating.id, rating.recipe_id, rating.score, rating.comment, rating.cooked_at, rating.created_at, rating.updated_at],
            )?;
            Ok(())
        })?,
        "plan_entry" => restore_or_delete(transaction, record, "plan_entries", |conn, value| {
            let entry: PlanEntry = serde_json::from_value(value)?;
            conn.execute(
                "INSERT INTO plan_entries(
                    id, date, slot, recipe_id, title_override, servings, status, notes,
                    sort_order, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET date=excluded.date, slot=excluded.slot,
                     recipe_id=excluded.recipe_id, title_override=excluded.title_override,
                     servings=excluded.servings, status=excluded.status, notes=excluded.notes,
                     sort_order=excluded.sort_order, updated_at=excluded.updated_at",
                params![
                    entry.id,
                    entry.date.to_string(),
                    entry.slot,
                    entry.recipe_id,
                    entry.title_override,
                    entry.servings,
                    entry.status,
                    entry.notes,
                    entry.sort_order,
                    entry.created_at,
                    entry.updated_at
                ],
            )?;
            Ok(())
        })?,
        "week_plan" => {
            let week_start = NaiveDate::parse_from_str(&record.entity_id, "%Y-%m-%d")
                .map_err(|_| DomainError::Validation("Ungültiges Datum im Undo-Verlauf".into()))?;
            let week_end = week_start + Duration::days(6);
            transaction.execute(
                "DELETE FROM plan_entries WHERE date BETWEEN ?1 AND ?2",
                params![week_start.to_string(), week_end.to_string()],
            )?;
            let entries: Vec<PlanEntry> = from_before(record)?;
            for entry in entries {
                save_plan_entry_tx(transaction, entry, false)?;
            }
        }
        "memory" => restore_or_delete(transaction, record, "memories", |conn, value| {
            let memory: Memory = serde_json::from_value(value)?;
            write_memory(conn, &memory)
        })?,
        "shopping_item" => {
            if let Some(before) = record.before.clone() {
                let item: ShoppingItem = serde_json::from_value(before)?;
                write_shopping_item(transaction, &item)?;
            } else {
                transaction.execute(
                    "DELETE FROM shopping_items WHERE id=?1",
                    [&record.entity_id],
                )?;
            }
        }
        other => {
            return Err(DomainError::Validation(format!(
                "Undo für Entität {other} wird nicht unterstützt"
            )));
        }
    }
    Ok(())
}

fn from_before<T: serde::de::DeserializeOwned>(record: &UndoRecord) -> DomainResult<T> {
    record
        .before
        .clone()
        .ok_or_else(|| DomainError::Validation("Undo-Snapshot fehlt".into()))
        .and_then(|value| serde_json::from_value(value).map_err(Into::into))
}

fn restore_or_delete<F>(
    transaction: &Transaction<'_>,
    record: &UndoRecord,
    table: &str,
    restore: F,
) -> DomainResult<()>
where
    F: FnOnce(&Connection, Value) -> DomainResult<()>,
{
    if let Some(before) = record.before.clone() {
        restore(transaction, before)
    } else {
        // Table names are internal constants from the match above, never user input.
        transaction.execute(
            &format!("DELETE FROM {table} WHERE id=?1"),
            [&record.entity_id],
        )?;
        Ok(())
    }
}
