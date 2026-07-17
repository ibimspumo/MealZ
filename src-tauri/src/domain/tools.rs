use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{
    models::{
        DynamicToolDefinition, Memory, PantryItem, PlanEntry, ProfilePatch, Recipe, RecipeRating,
    },
    store::{DomainError, DomainResult, MealzStore},
};

pub fn dynamic_tool_definitions() -> Vec<DynamicToolDefinition> {
    vec![
        tool(
            "profile_get_context",
            "Liest das aktuelle Nutzerprofil, Ernährungsvorgaben, verfügbares Küchenequipment und bestätigte Memories. Vor jeder größeren Planung verwenden.",
            json!({}),
            &[],
        ),
        tool(
            "profile_update",
            "Aktualisiert ausdrücklich genannte Profilfelder. Nicht aus Vermutungen ableiten; Präferenzen gehören zuerst in memory_propose.",
            json!({
                "patch": profile_patch_schema()
            }),
            &["patch"],
        ),
        tool(
            "memory_recall",
            "Sucht transparente Erinnerungen über Vorlieben, Abneigungen, Routinen, Constraints und Beobachtungen.",
            json!({
                "query":{"type":"string"},
                "kind":{"type":"string","enum":["preference","dislike","allergy","routine","constraint","equipment","goal","observation"]},
                "status":{"type":"string","enum":["proposed","confirmed","dismissed"]},
                "limit":{"type":"integer","minimum":1,"maximum":100}
            }),
            &[],
        ),
        tool(
            "memory_propose",
            "Schlägt eine neue, sichtbare Erinnerung mit Confidence und Evidenz vor. Neue Schlussfolgerungen niemals direkt als bestätigt speichern.",
            json!({
                "kind":{"type":"string","enum":["preference","dislike","allergy","routine","constraint","equipment","goal","observation"]},
                "content":{"type":"string","minLength":1},
                "confidence":{"type":"number","minimum":0,"maximum":1},
                "evidence":{"type":"array","items":{"type":"string"}},
                "preferenceScore":{"type":["number","null"],"minimum":1,"maximum":10},
                "source":{"type":["string","null"]}
            }),
            &["kind", "content", "confidence", "evidence"],
        ),
        tool(
            "memory_set_status",
            "Bestätigt oder verwirft eine vorgeschlagene Erinnerung bzw. setzt sie erneut auf vorgeschlagen.",
            json!({
                "memoryId":{"type":"string"},
                "status":{"type":"string","enum":["proposed","confirmed","dismissed"]},
                "confidence":{"type":"number","minimum":0,"maximum":1}
            }),
            &["memoryId", "status"],
        ),
        tool(
            "memory_forget",
            "Löscht eine Erinnerung. Die Änderung kann über changes_undo rückgängig gemacht werden.",
            json!({"memoryId":{"type":"string"}}),
            &["memoryId"],
        ),
        tool(
            "recipes_search",
            "Durchsucht den lokalen Rezeptkatalog nach Titel, Beschreibung oder Tag.",
            json!({
                "query":{"type":"string"},
                "includeArchived":{"type":"boolean"},
                "limit":{"type":"integer","minimum":1,"maximum":100}
            }),
            &[],
        ),
        tool(
            "recipes_get",
            "Liest ein vollständiges strukturiertes Rezept einschließlich Zutaten, Schritte, Quellen, Bilder und Nährwerte.",
            json!({"recipeId":{"type":"string"}}),
            &["recipeId"],
        ),
        tool(
            "recipes_save",
            "Erstellt oder aktualisiert ein vollständiges Rezept. Zutaten, Schritte und Portionen müssen strukturiert sein; Nährwerte werden deterministisch aus Zutaten summiert, sofern vorhanden. Ein Rezept ist erst fertig, wenn imageComplete=true zurückkommt. Bei imageComplete=false ist requiredNextAction vor der finalen Antwort zwingend auszuführen.",
            json!({"recipe": recipe_schema()}),
            &["recipe"],
        ),
        tool(
            "recipes_delete",
            "Löscht ein Rezept samt abhängiger Zutaten und Schritte; per Undo wiederherstellbar.",
            json!({"recipeId":{"type":"string"}}),
            &["recipeId"],
        ),
        tool(
            "ratings_record",
            "Speichert eine Bewertung von 1 bis 5 samt optionalem Kommentar und Kochdatum.",
            json!({
                "recipeId":{"type":"string"},
                "score":{"type":"integer","minimum":1,"maximum":5},
                "comment":{"type":["string","null"]},
                "cookedAt":{"type":["string","null"],"description":"ISO-Datum oder RFC3339"}
            }),
            &["recipeId", "score"],
        ),
        tool(
            "plan_get_week",
            "Liest eine Montag-bis-Sonntag-Woche inklusive Plan-Einträgen und deterministisch summierten Nährwerten.",
            json!({"weekStart":{"type":"string","format":"date","description":"Montag als YYYY-MM-DD"}}),
            &["weekStart"],
        ),
        tool(
            "plan_propose_week",
            "Ersetzt nach Nutzerauftrag die gewählte Woche atomar durch strukturierte Meal-Slots. Verwende standardmäßig sieben unterschiedliche Hauptgerichte und keine Frühstücksslots.",
            json!({
                "weekStart":{"type":"string","format":"date"},
                "entries":{"type":"array","items":plan_entry_schema(),"maxItems":70}
            }),
            &["weekStart", "entries"],
        ),
        tool(
            "plan_set_meal",
            "Erstellt oder aktualisiert genau einen Meal-Slot. Für auswärts titleOverride setzen, recipeId weglassen und status=eating_out verwenden.",
            json!({"entry":plan_entry_schema()}),
            &["entry"],
        ),
        tool(
            "plan_remove_meal",
            "Entfernt einen einzelnen Plan-Eintrag.",
            json!({"entryId":{"type":"string"}}),
            &["entryId"],
        ),
        tool(
            "shopping_rebuild",
            "Berechnet die Einkaufsliste für einen Datumsbereich reproduzierbar aus aktiven Plan-Rezepten neu. Portionen und kompatible Einheiten werden aggregiert; manuelle Artikel und Abhakstatus bleiben erhalten.",
            json!({
                "rangeStart":{"type":"string","format":"date"},
                "rangeEnd":{"type":"string","format":"date"}
            }),
            &["rangeStart", "rangeEnd"],
        ),
        tool(
            "shopping_toggle",
            "Setzt den Abhakstatus eines Einkaufsartikels explizit.",
            json!({"itemId":{"type":"string"},"checked":{"type":"boolean"}}),
            &["itemId", "checked"],
        ),
        tool(
            "shopping_add_manual",
            "Fügt einer Datumsbereich-Einkaufsliste einen manuellen Artikel hinzu.",
            json!({
                "rangeStart":{"type":"string","format":"date"},
                "rangeEnd":{"type":"string","format":"date"},
                "name":{"type":"string","minLength":1},
                "quantity":{"type":"number","exclusiveMinimum":0},
                "unit":{"type":"string"},
                "aisle":{"type":["string","null"]},
                "note":{"type":["string","null"]}
            }),
            &["rangeStart", "rangeEnd", "name", "quantity", "unit"],
        ),
        tool(
            "shopping_remove_item",
            "Entfernt einen Einkaufsartikel. Abgeleitete Artikel erscheinen beim nächsten Rebuild erneut, falls weiterhin benötigt.",
            json!({"itemId":{"type":"string"}}),
            &["itemId"],
        ),
        tool(
            "pantry_list",
            "Liest Vorratsartikel, die bei der Einkaufslisten-Berechnung ausgelassen werden sollen.",
            json!({}),
            &[],
        ),
        tool(
            "pantry_set",
            "Speichert oder aktualisiert einen Vorratsartikel und ob er von Einkaufslisten ausgeschlossen wird.",
            json!({"item":{"type":"object","description":"PantryItem in camelCase"}}),
            &["item"],
        ),
        tool(
            "pantry_remove",
            "Entfernt einen Vorratsartikel; zukünftige Rebuilds nehmen die Zutat wieder auf.",
            json!({"itemId":{"type":"string"}}),
            &["itemId"],
        ),
        tool(
            "onboarding_complete",
            "Markiert das persönliche MealZ-Briefing erst dann als abgeschlossen, wenn Name, Alltag, Küchenkontext und erste Präferenzen ausreichend geklärt und über die passenden Profil-/Memory-Tools gespeichert wurden.",
            json!({
                "briefingSummary":{"type":"string","minLength":20,"description":"Kurze Zusammenfassung der geklärten persönlichen Ausgangslage, einschließlich Alltag und erster Essenspräferenzen"}
            }),
            &["briefingSummary"],
        ),
        tool(
            "changes_undo",
            "Macht die letzte noch nicht rückgängig gemachte strukturierte MealZ-Änderung rückgängig.",
            json!({}),
            &[],
        ),
    ]
}

fn nutrition_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "caloriesKcal":{"type":"number","minimum":0},
            "proteinG":{"type":"number","minimum":0},
            "carbsG":{"type":"number","minimum":0},
            "fatG":{"type":"number","minimum":0},
            "fiberG":{"type":"number","minimum":0}
        },
        "additionalProperties":false
    })
}

fn profile_patch_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "name":{"type":"string","minLength":1},
            "locale":{"type":"string"},
            "timezone":{"type":"string"},
            "householdSize":{"type":"integer","minimum":1,"maximum":50},
            "heightCm":{"type":["number","null"],"exclusiveMinimum":0},
            "weightKg":{"type":["number","null"],"exclusiveMinimum":0},
            "birthDate":{"type":["string","null"],"format":"date"},
            "sexForEnergy":{"type":["string","null"],"enum":["male","female",null]},
            "activityLevel":{"type":"string","enum":["inactive","low_active","active","very_active"]},
            "calorieTargetMode":{"type":"string","enum":["calculated","manual"]},
            "manualCalorieTargetKcal":{"type":["number","null"],"exclusiveMinimum":0},
            "dietaryStyle":{"type":["string","null"]},
            "nutritionTargets":{
                "type":"object",
                "properties":{
                    "caloriesKcal":{"type":["number","null"],"minimum":0},
                    "proteinG":{"type":["number","null"],"minimum":0},
                    "carbsG":{"type":["number","null"],"minimum":0},
                    "fatG":{"type":["number","null"],"minimum":0},
                    "fiberG":{"type":["number","null"],"minimum":0}
                },
                "additionalProperties":false
            },
            "weekdayMaxMinutes":{"type":"integer","minimum":0},
            "weekendMaxMinutes":{"type":"integer","minimum":0},
            "defaultServings":{"type":"number","exclusiveMinimum":0},
            "notes":{"type":["string","null"]}
        },
        "additionalProperties":false
    })
}

fn recipe_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "id":{"type":"string"},
            "title":{"type":"string","minLength":1},
            "summary":{"type":"string"},
            "servings":{"type":"number","exclusiveMinimum":0},
            "prepMinutes":{"type":"integer","minimum":0},
            "cookMinutes":{"type":"integer","minimum":0},
            "difficulty":{"type":"string"},
            "cuisine":{"type":"string"},
            "mealTypes":{"type":"array","items":{"type":"string"}},
            "tags":{"type":"array","items":{"type":"string"}},
            "favorite":{"type":"boolean"},
            "archived":{"type":"boolean"},
            "sourceKind":{"type":"string"},
            "confidence":{"type":"number","minimum":0,"maximum":1},
            "ingredients":{
                "type":"array",
                "minItems":1,
                "items":{
                    "type":"object",
                    "properties":{
                        "id":{"type":"string"},
                        "recipeId":{"type":"string"},
                        "name":{"type":"string","minLength":1},
                        "quantity":{"type":"number","minimum":0},
                        "unit":{"type":"string"},
                        "aisle":{"type":"string"},
                        "preparation":{"type":["string","null"]},
                        "optional":{"type":"boolean"},
                        "nutrition":nutrition_schema(),
                        "position":{"type":"integer","minimum":0}
                    },
                    "required":["name","quantity","unit"],
                    "additionalProperties":false
                }
            },
            "steps":{
                "type":"array",
                "minItems":1,
                "items":{
                    "type":"object",
                    "properties":{
                        "id":{"type":"string"},
                        "recipeId":{"type":"string"},
                        "position":{"type":"integer","minimum":0},
                        "instruction":{"type":"string","minLength":1},
                        "timerMinutes":{"type":["integer","null"],"minimum":0}
                    },
                    "required":["instruction"],
                    "additionalProperties":false
                }
            },
            "sources":{
                "type":"array",
                "items":{
                    "type":"object",
                    "properties":{
                        "id":{"type":"string"},
                        "recipeId":{"type":"string"},
                        "title":{"type":"string","minLength":1},
                        "url":{"type":["string","null"]},
                        "publisher":{"type":["string","null"]},
                        "sourceType":{"type":"string"},
                        "accessedAt":{"type":["string","null"]}
                    },
                    "required":["title"],
                    "additionalProperties":false
                }
            },
            "images":{
                "type":"array",
                "items":{
                    "type":"object",
                    "properties":{
                        "id":{"type":"string"},
                        "recipeId":{"type":"string"},
                        "url":{"type":"string","minLength":1},
                        "kind":{"type":"string"},
                        "altText":{"type":["string","null"]},
                        "attribution":{"type":["string","null"]},
                        "position":{"type":"integer","minimum":0}
                    },
                    "required":["url"],
                    "additionalProperties":false
                }
            },
            "nutritionTotal":nutrition_schema(),
            "nutritionPerServing":nutrition_schema(),
            "createdAt":{"type":"string"},
            "updatedAt":{"type":"string"}
        },
        "required":["title","servings","prepMinutes","cookMinutes","ingredients","steps"],
        "additionalProperties":false
    })
}

fn plan_entry_schema() -> Value {
    json!({
        "type":"object",
        "properties":{
            "id":{"type":"string"},
            "date":{"type":"string","format":"date"},
            "slot":{"type":"string","enum":["breakfast","lunch","dinner","snack","shake","dessert","other"]},
            "recipeId":{"type":["string","null"]},
            "titleOverride":{"type":["string","null"]},
            "servings":{"type":"number","exclusiveMinimum":0},
            "status":{"type":"string","enum":["planned","prepared","cooked","leftovers","skipped","eating_out","cancelled"]},
            "notes":{"type":["string","null"]},
            "sortOrder":{"type":"integer","minimum":0},
            "createdAt":{"type":"string"},
            "updatedAt":{"type":"string"}
        },
        "required":["date","slot"],
        "additionalProperties":false,
        "anyOf":[
            {"required":["recipeId"]},
            {"required":["titleOverride"]}
        ]
    })
}

fn tool(
    name: &str,
    description: &str,
    properties: Value,
    required: &[&str],
) -> DynamicToolDefinition {
    DynamicToolDefinition {
        name: name.into(),
        description: description.into(),
        input_schema: json!({
            "type":"object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        }),
    }
}

impl MealzStore {
    pub fn dynamic_tools(&self) -> Vec<DynamicToolDefinition> {
        dynamic_tool_definitions()
    }

    pub fn execute_tool(&self, name: &str, arguments: Value) -> DomainResult<Value> {
        let object = arguments.as_object().ok_or_else(|| {
            DomainError::Validation("Tool-Argumente müssen ein Objekt sein".into())
        })?;
        match name {
            "profile_get_context" => Ok(json!({
                "profile": self.get_profile()?,
                "equipment": self.list_equipment()?,
                "memories": self.recall_memories(None, None, Some("confirmed"), 100)?
            })),
            "profile_update" => {
                let patch: ProfilePatch = from_field(object, "patch")?;
                Ok(serde_json::to_value(self.update_profile(patch)?)?)
            }
            "memory_recall" => {
                let query = optional_string(object, "query");
                let kind = optional_string(object, "kind");
                let status = optional_string(object, "status");
                let limit = optional_u64(object, "limit").unwrap_or(50) as usize;
                Ok(serde_json::to_value(self.recall_memories(
                    query.as_deref(),
                    kind.as_deref(),
                    status.as_deref(),
                    limit,
                )?)?)
            }
            "memory_propose" => {
                let memory = Memory {
                    id: String::new(),
                    kind: required_string(object, "kind")?,
                    content: required_string(object, "content")?,
                    confidence: required_f64(object, "confidence")?,
                    evidence: object
                        .get("evidence")
                        .cloned()
                        .map(serde_json::from_value)
                        .transpose()?
                        .unwrap_or_default(),
                    status: "proposed".into(),
                    preference_score: optional_f64(object, "preferenceScore"),
                    source: optional_string(object, "source"),
                    created_at: String::new(),
                    updated_at: String::new(),
                    last_used_at: None,
                };
                Ok(serde_json::to_value(self.save_memory(memory)?)?)
            }
            "memory_set_status" => Ok(serde_json::to_value(self.set_memory_status(
                &required_string(object, "memoryId")?,
                &required_string(object, "status")?,
                optional_f64(object, "confidence"),
            )?)?),
            "memory_forget" => {
                let id = required_string(object, "memoryId")?;
                self.delete_memory(&id)?;
                Ok(json!({"deleted":true,"memoryId":id}))
            }
            "recipes_search" => {
                let query = optional_string(object, "query");
                let include_archived = optional_bool(object, "includeArchived").unwrap_or(false);
                let limit = optional_u64(object, "limit").unwrap_or(50) as usize;
                Ok(serde_json::to_value(self.search_recipes(
                    query.as_deref(),
                    include_archived,
                    limit,
                )?)?)
            }
            "recipes_get" => Ok(serde_json::to_value(
                self.get_recipe(&required_string(object, "recipeId")?)?,
            )?),
            "recipes_save" => {
                let recipe: Recipe = from_field(object, "recipe")?;
                let saved = self.save_recipe(recipe)?;
                let needs_image = saved.images.is_empty();
                let mut value = serde_json::to_value(&saved)?;
                if let Some(result) = value.as_object_mut() {
                    result.insert("imageComplete".into(), json!(!needs_image));
                    if needs_image {
                        result.insert(
                            "requiredNextAction".into(),
                            json!(format!(
                                "Dieses Rezept hat noch kein Bild. Rufe jetzt recipe_image_generation_prepare mit recipeId={} auf und starte danach die native imageGeneration. Beende den Turn erst, wenn das Bild gespeichert wurde.",
                                saved.id
                            )),
                        );
                    }
                }
                Ok(value)
            }
            "recipes_delete" => {
                let id = required_string(object, "recipeId")?;
                self.delete_recipe(&id)?;
                Ok(json!({"deleted":true,"recipeId":id}))
            }
            "ratings_record" => {
                let rating = RecipeRating {
                    id: String::new(),
                    recipe_id: required_string(object, "recipeId")?,
                    score: required_i64(object, "score")?,
                    comment: optional_string(object, "comment"),
                    cooked_at: optional_string(object, "cookedAt"),
                    created_at: String::new(),
                    updated_at: String::new(),
                };
                Ok(serde_json::to_value(self.rate_recipe(rating)?)?)
            }
            "plan_get_week" => Ok(serde_json::to_value(
                self.get_week(required_date(object, "weekStart")?)?,
            )?),
            "plan_propose_week" => {
                let week_start = required_date(object, "weekStart")?;
                let entries: Vec<PlanEntry> = from_field(object, "entries")?;
                Ok(serde_json::to_value(
                    self.replace_week(week_start, entries)?,
                )?)
            }
            "plan_set_meal" => {
                let entry: PlanEntry = from_field(object, "entry")?;
                Ok(serde_json::to_value(self.save_plan_entry(entry)?)?)
            }
            "plan_remove_meal" => {
                let id = required_string(object, "entryId")?;
                self.delete_plan_entry(&id)?;
                Ok(json!({"deleted":true,"entryId":id}))
            }
            "shopping_rebuild" => Ok(serde_json::to_value(self.rebuild_shopping_list(
                required_date(object, "rangeStart")?,
                required_date(object, "rangeEnd")?,
            )?)?),
            "shopping_toggle" => Ok(serde_json::to_value(self.set_shopping_checked(
                &required_string(object, "itemId")?,
                required_bool(object, "checked")?,
            )?)?),
            "shopping_add_manual" => Ok(serde_json::to_value(self.add_manual_shopping_item(
                required_date(object, "rangeStart")?,
                required_date(object, "rangeEnd")?,
                required_string(object, "name")?,
                required_f64(object, "quantity")?,
                required_string(object, "unit")?,
                optional_string(object, "aisle"),
                optional_string(object, "note"),
            )?)?),
            "shopping_remove_item" => {
                let id = required_string(object, "itemId")?;
                self.delete_shopping_item(&id)?;
                Ok(json!({"deleted":true,"itemId":id}))
            }
            "pantry_list" => Ok(serde_json::to_value(self.list_pantry_items()?)?),
            "pantry_set" => {
                let item: PantryItem = from_field(object, "item")?;
                Ok(serde_json::to_value(self.save_pantry_item(item)?)?)
            }
            "pantry_remove" => {
                let id = required_string(object, "itemId")?;
                self.delete_pantry_item(&id)?;
                Ok(json!({"deleted":true,"itemId":id}))
            }
            "onboarding_complete" => {
                let profile = self.get_profile()?;
                if profile.name.trim().is_empty() {
                    return Err(DomainError::Validation(
                        "Onboarding kann erst nach dem Speichern eines Namens abgeschlossen werden"
                            .into(),
                    ));
                }
                let summary = required_string(object, "briefingSummary")?;
                if summary.chars().count() < 20 {
                    return Err(DomainError::Validation(
                        "Das Onboarding-Briefing ist noch zu kurz; Alltag und erste Essenspräferenzen müssen zusammengefasst werden".into(),
                    ));
                }
                if !self.list_equipment()?.iter().any(|item| item.available) {
                    return Err(DomainError::Validation(
                        "Vor Abschluss muss mindestens ein verfügbares Küchengerät hinterlegt sein"
                            .into(),
                    ));
                }
                if self
                    .recall_memories(None, None, Some("confirmed"), 1)?
                    .is_empty()
                {
                    return Err(DomainError::Validation(
                        "Vor Abschluss muss mindestens eine bestätigte Routine oder Präferenz hinterlegt sein"
                            .into(),
                    ));
                }
                self.save_memory(Memory {
                    id: String::new(),
                    kind: "routine".into(),
                    content: summary,
                    confidence: 1.0,
                    evidence: vec!["Persönliches Onboarding mit Mila".into()],
                    status: "confirmed".into(),
                    preference_score: None,
                    source: Some("onboarding".into()),
                    created_at: String::new(),
                    updated_at: String::new(),
                    last_used_at: None,
                })?;
                self.set_app_meta("onboarding_complete", "true")?;
                Ok(json!({"completed":true}))
            }
            "changes_undo" => Ok(serde_json::to_value(self.undo_last()?)?),
            other => Err(DomainError::Validation(format!(
                "Unbekanntes MealZ-Tool: {other}"
            ))),
        }
    }
}

fn from_field<T: for<'de> Deserialize<'de>>(
    object: &Map<String, Value>,
    key: &str,
) -> DomainResult<T> {
    let value = object
        .get(key)
        .cloned()
        .ok_or_else(|| DomainError::Validation(format!("Pflichtfeld {key} fehlt")))?;
    Ok(serde_json::from_value(value)?)
}

fn required_string(object: &Map<String, Value>, key: &str) -> DomainResult<String> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| DomainError::Validation(format!("Pflichtfeld {key} fehlt")))?;
    Ok(value.to_string())
}

fn optional_string(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn required_f64(object: &Map<String, Value>, key: &str) -> DomainResult<f64> {
    object
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| DomainError::Validation(format!("Numerisches Pflichtfeld {key} fehlt")))
}

fn optional_f64(object: &Map<String, Value>, key: &str) -> Option<f64> {
    object
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn required_i64(object: &Map<String, Value>, key: &str) -> DomainResult<i64> {
    object
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| DomainError::Validation(format!("Ganzzahliges Pflichtfeld {key} fehlt")))
}

fn optional_u64(object: &Map<String, Value>, key: &str) -> Option<u64> {
    object.get(key).and_then(Value::as_u64)
}

fn required_bool(object: &Map<String, Value>, key: &str) -> DomainResult<bool> {
    object
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| DomainError::Validation(format!("Boolesches Pflichtfeld {key} fehlt")))
}

fn optional_bool(object: &Map<String, Value>, key: &str) -> Option<bool> {
    object.get(key).and_then(Value::as_bool)
}

fn required_date(object: &Map<String, Value>, key: &str) -> DomainResult<NaiveDate> {
    let value = required_string(object, key)?;
    NaiveDate::parse_from_str(&value, "%Y-%m-%d")
        .map_err(|_| DomainError::Validation(format!("{key} muss YYYY-MM-DD sein")))
}
