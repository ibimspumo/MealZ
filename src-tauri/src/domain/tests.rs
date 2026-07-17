use chrono::{Duration, NaiveDate};
use serde_json::json;

use super::{
    MealzStore,
    models::{
        AgentMessage, Memory, Nutrition, PantryItem, PlanEntry, ProfilePatch, Recipe, RecipeImage,
        RecipeIngredient, RecipeSource, RecipeStep,
    },
    store::calculated_eer_kcal,
};

fn test_recipe(title: &str) -> Recipe {
    Recipe {
        id: String::new(),
        title: title.into(),
        summary: "Ein reproduzierbares Testrezept".into(),
        servings: 2.0,
        prep_minutes: 10,
        cook_minutes: 20,
        difficulty: "easy".into(),
        cuisine: "test".into(),
        meal_types: vec!["dinner".into()],
        tags: vec!["high-protein".into()],
        favorite: false,
        archived: false,
        source_kind: "test".into(),
        confidence: 0.95,
        ingredients: vec![
            RecipeIngredient {
                id: String::new(),
                recipe_id: String::new(),
                name: "Kartoffeln".into(),
                quantity: 1.0,
                unit: "kg".into(),
                aisle: "Gemüse".into(),
                preparation: None,
                optional: false,
                nutrition: Nutrition {
                    calories_kcal: 770.0,
                    protein_g: 20.0,
                    carbs_g: 170.0,
                    fat_g: 1.0,
                    fiber_g: 22.0,
                },
                position: 0,
            },
            RecipeIngredient {
                id: String::new(),
                recipe_id: String::new(),
                name: "Salz".into(),
                quantity: 1.0,
                unit: "Prise".into(),
                aisle: "Vorrat".into(),
                preparation: None,
                optional: true,
                nutrition: Nutrition::default(),
                position: 1,
            },
        ],
        steps: vec![RecipeStep {
            id: String::new(),
            recipe_id: String::new(),
            position: 0,
            instruction: "Kartoffeln garen.".into(),
            timer_minutes: Some(20),
        }],
        sources: vec![RecipeSource {
            id: String::new(),
            recipe_id: String::new(),
            title: "Testquelle".into(),
            url: Some("https://example.com/recipe".into()),
            publisher: Some("Example".into()),
            source_type: "web".into(),
            accessed_at: None,
        }],
        images: vec![RecipeImage {
            id: String::new(),
            recipe_id: String::new(),
            url: "https://example.com/food.jpg".into(),
            kind: "remote".into(),
            alt_text: Some("Kartoffeln".into()),
            attribution: None,
            position: 0,
        }],
        nutrition_total: Nutrition::default(),
        nutrition_per_serving: Nutrition::default(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

#[test]
fn bootstrap_seeds_personal_context_and_catalog() {
    let store = MealzStore::in_memory().unwrap();
    let bootstrap = store.bootstrap().unwrap();
    assert_eq!(bootstrap.profile.name, "");
    assert!(!store.onboarding_complete().unwrap());
    assert!(bootstrap.equipment.len() >= 8);
    assert!(!bootstrap.recipes.is_empty());
    assert!(
        bootstrap
            .memories
            .iter()
            .any(|memory| memory.content.contains("kein Frühstück"))
    );
    assert_eq!(bootstrap.current_week.days.len(), 7);
}

#[test]
fn recipe_roundtrip_calculates_nutrition_and_keeps_sources() {
    let store = MealzStore::in_memory().unwrap();
    let saved = store.save_recipe(test_recipe("Ofenkartoffeln")).unwrap();
    assert!(saved.id.starts_with("recipe-"));
    assert_eq!(saved.nutrition_total.calories_kcal, 770.0);
    assert_eq!(saved.nutrition_per_serving.protein_g, 10.0);
    assert_eq!(saved.ingredients[0].recipe_id, saved.id);
    let loaded = store.get_recipe(&saved.id).unwrap();
    assert_eq!(loaded.sources[0].publisher.as_deref(), Some("Example"));
    assert_eq!(loaded.images.len(), 1);
    assert!(
        store
            .search_recipes(Some("Ofenkart"), false, 10)
            .unwrap()
            .iter()
            .any(|recipe| recipe.id == saved.id)
    );
}

#[test]
fn week_nutrition_and_shopping_are_portion_aware_and_reproducible() {
    let store = MealzStore::in_memory().unwrap();
    let recipe = store.save_recipe(test_recipe("Kartoffelblech")).unwrap();
    let monday = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
    let mut first_entry = store
        .save_plan_entry(PlanEntry {
            id: String::new(),
            date: monday,
            slot: "dinner".into(),
            recipe_id: Some(recipe.id.clone()),
            title_override: None,
            servings: 1.0,
            status: "planned".into(),
            notes: None,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        })
        .unwrap();
    store
        .save_plan_entry(PlanEntry {
            id: String::new(),
            date: monday + Duration::days(1),
            slot: "dinner".into(),
            recipe_id: Some(recipe.id),
            title_override: None,
            servings: 2.0,
            status: "planned".into(),
            notes: None,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        })
        .unwrap();

    let week = store.get_week(monday).unwrap();
    assert_eq!(week.nutrition.calories_kcal, 1155.0);
    let list = store
        .rebuild_shopping_list(monday, monday + Duration::days(6))
        .unwrap();
    let potatoes = list
        .items
        .iter()
        .find(|item| item.name == "Kartoffeln")
        .unwrap();
    assert_eq!(potatoes.quantity, 1500.0);
    assert_eq!(potatoes.unit, "g");
    assert!(!list.items.iter().any(|item| item.name == "Salz"));

    store.set_shopping_checked(&potatoes.id, true).unwrap();
    first_entry.servings = 2.0;
    store.save_plan_entry(first_entry).unwrap();
    let auto_rebuilt = store
        .get_shopping_list(monday, monday + Duration::days(6))
        .unwrap();
    assert!(
        auto_rebuilt
            .items
            .iter()
            .any(|item| item.name == "Kartoffeln" && item.quantity == 2000.0 && item.checked)
    );
    store
        .add_manual_shopping_item(
            monday,
            monday + Duration::days(6),
            "Küchenrolle".into(),
            1.0,
            "Stück".into(),
            None,
            None,
        )
        .unwrap();
    let rebuilt = store
        .rebuild_shopping_list(monday, monday + Duration::days(6))
        .unwrap();
    assert!(
        rebuilt
            .items
            .iter()
            .any(|item| item.name == "Kartoffeln" && item.checked)
    );
    assert!(
        rebuilt
            .items
            .iter()
            .any(|item| item.name == "Küchenrolle" && item.manual)
    );
}

#[test]
fn memories_are_proposed_visible_and_undoable() {
    let store = MealzStore::in_memory().unwrap();
    let memory = store
        .save_memory(Memory {
            id: String::new(),
            kind: "preference".into(),
            content: "Karotten sind okay, aber nicht meine erste Wahl.".into(),
            confidence: 0.8,
            evidence: vec!["Direkte Aussage".into()],
            status: "proposed".into(),
            preference_score: Some(4.0),
            source: Some("chat".into()),
            created_at: String::new(),
            updated_at: String::new(),
            last_used_at: None,
        })
        .unwrap();
    let confirmed = store
        .set_memory_status(&memory.id, "confirmed", Some(1.0))
        .unwrap();
    assert_eq!(confirmed.status, "confirmed");
    store.undo_last().unwrap();
    let recalled = store
        .recall_memories(Some("Karotten"), None, Some("proposed"), 10)
        .unwrap();
    assert_eq!(recalled.len(), 1);
}

#[test]
fn dynamic_tools_validate_and_execute_structured_changes() {
    let store = MealzStore::in_memory().unwrap();
    let names: Vec<_> = store
        .dynamic_tools()
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    assert!(names.contains(&"plan_propose_week".to_string()));
    assert!(names.contains(&"shopping_rebuild".to_string()));
    assert!(names.contains(&"onboarding_complete".to_string()));
    assert!(names.contains(&"changes_undo".to_string()));

    let result = store
        .execute_tool(
            "memory_propose",
            json!({
                "kind":"dislike",
                "content":"Sehr scharfe Gerichte vermeiden.",
                "confidence":0.75,
                "evidence":["Chat-Aussage"],
                "preferenceScore":2
            }),
        )
        .unwrap();
    assert_eq!(result["status"], "proposed");
    assert!(
        store
            .execute_tool("onboarding_complete", json!({}))
            .is_err()
    );
    store
        .update_profile(ProfilePatch {
            name: Some("Timo".into()),
            ..ProfilePatch::default()
        })
        .unwrap();
    assert_eq!(
        store
            .execute_tool(
                "onboarding_complete",
                json!({"briefingSummary":"Kein Frühstück, Fokus auf Abendessen."})
            )
            .unwrap()["completed"],
        true
    );
    assert!(store.onboarding_complete().unwrap());
    assert!(store.execute_tool("unknown", json!({})).is_err());
}

#[test]
fn codex_thread_id_has_a_store_level_persistence_api() {
    let store = MealzStore::in_memory().unwrap();
    assert!(store.current_agent_session().unwrap().is_none());
    let session = store
        .set_current_codex_thread_id(Some("thread_123".into()))
        .unwrap();
    assert_eq!(session.codex_thread_id.as_deref(), Some("thread_123"));
    assert_eq!(
        store
            .current_agent_session()
            .unwrap()
            .unwrap()
            .codex_thread_id
            .as_deref(),
        Some("thread_123")
    );
    let cleared = store.set_current_codex_thread_id(None).unwrap();
    assert!(cleared.codex_thread_id.is_none());
}

#[test]
fn creating_a_new_agent_session_archives_the_previous_current_session() {
    let store = MealzStore::in_memory().unwrap();
    let first = store
        .set_current_codex_thread_id(Some("thread-old".into()))
        .unwrap();
    let second = store
        .create_agent_session(None, "Neues Gespräch".into(), json!({"agentName":"Mila"}))
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(
        store.current_agent_session().unwrap().unwrap().id,
        second.id
    );
    let old_messages = store.list_agent_messages(&first.id, 10).unwrap();
    assert!(old_messages.is_empty());
    let current = store
        .set_current_codex_thread_id(Some("thread-new".into()))
        .unwrap();
    assert_eq!(current.id, second.id);
    assert_eq!(current.codex_thread_id.as_deref(), Some("thread-new"));
}

#[test]
fn conversations_list_useful_metadata_and_keep_exactly_one_active_session() {
    let store = MealzStore::in_memory().unwrap();
    let first = store
        .create_agent_session(
            Some("thread-lasagne".into()),
            "Lasagne planen".into(),
            json!({}),
        )
        .unwrap();
    store
        .append_agent_message(AgentMessage {
            id: String::new(),
            session_id: first.id.clone(),
            role: "user".into(),
            content: "Plane mir eine Lasagne für Sonntag".into(),
            item_id: None,
            tool_name: None,
            tool_payload: None,
            created_at: String::new(),
        })
        .unwrap();
    let second = store
        .create_agent_session(Some("thread-curry".into()), "Curry".into(), json!({}))
        .unwrap();
    store
        .append_agent_message(AgentMessage {
            id: String::new(),
            session_id: second.id.clone(),
            role: "assistant".into(),
            content: "Ich plane ein schnelles Curry.".into(),
            item_id: None,
            tool_name: None,
            tool_payload: None,
            created_at: String::new(),
        })
        .unwrap();
    store
        .record_agent_turn_activity(
            &second.id,
            "turn-1:tool-1",
            json!({"id":"tool-1","name":"webSearch"}),
        )
        .unwrap();

    let conversations = store.list_agent_sessions().unwrap();
    assert_eq!(conversations.len(), 2);
    assert_eq!(conversations[0].session.id, second.id);
    assert_eq!(conversations[0].session.status, "active");
    assert_eq!(conversations[0].message_count, 1);
    assert_eq!(
        conversations[0].preview.as_deref(),
        Some("Ich plane ein schnelles Curry.")
    );
    assert_eq!(conversations[1].session.status, "archived");

    let restored = store.activate_agent_session(&first.id).unwrap();
    assert_eq!(restored.codex_thread_id.as_deref(), Some("thread-lasagne"));
    assert_eq!(store.list_agent_messages(&first.id, 10).unwrap().len(), 1);
    let conversations = store.list_agent_sessions().unwrap();
    assert_eq!(
        conversations
            .iter()
            .filter(|item| item.session.status == "active")
            .count(),
        1
    );
    assert_eq!(conversations[0].session.id, first.id);
}

#[test]
fn activating_an_unknown_conversation_does_not_archive_the_current_one() {
    let store = MealzStore::in_memory().unwrap();
    let current = store
        .create_agent_session(Some("thread-current".into()), "Aktuell".into(), json!({}))
        .unwrap();
    assert!(store.activate_agent_session("missing-session").is_err());
    assert_eq!(
        store.current_agent_session().unwrap().unwrap().id,
        current.id
    );
}

#[test]
fn pantry_exclusions_are_applied_on_every_rebuild() {
    let store = MealzStore::in_memory().unwrap();
    let recipe = store.save_recipe(test_recipe("Vorratstest")).unwrap();
    let monday = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
    store
        .save_plan_entry(PlanEntry {
            id: String::new(),
            date: monday,
            slot: "dinner".into(),
            recipe_id: Some(recipe.id),
            title_override: None,
            servings: 2.0,
            status: "planned".into(),
            notes: None,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        })
        .unwrap();
    store
        .save_pantry_item(PantryItem {
            id: String::new(),
            name: "Kartoffeln".into(),
            exclude_from_shopping: true,
            note: Some("Schon vorhanden".into()),
            created_at: String::new(),
            updated_at: String::new(),
        })
        .unwrap();
    let list = store
        .rebuild_shopping_list(monday, monday + Duration::days(6))
        .unwrap();
    assert!(!list.items.iter().any(|item| item.name == "Kartoffeln"));
}

#[test]
fn nasem_2023_eer_is_calculated_and_manual_target_is_preserved() {
    let store = MealzStore::in_memory().unwrap();
    let profile = store
        .update_profile(ProfilePatch {
            name: Some("Timo".into()),
            height_cm: Some(Some(180.0)),
            weight_kg: Some(Some(80.0)),
            birth_date: Some(Some("1995-01-01".into())),
            sex_for_energy: Some(Some("male".into())),
            activity_level: Some("low_active".into()),
            calorie_target_mode: Some("manual".into()),
            manual_calorie_target_kcal: Some(Some(2750.0)),
            ..ProfilePatch::default()
        })
        .unwrap();
    assert_eq!(profile.nutrition_targets.calories_kcal, Some(2750.0));
    let expected =
        calculated_eer_kcal(&profile, NaiveDate::from_ymd_opt(2026, 7, 17).unwrap()).unwrap();
    assert_eq!(expected, 2935);

    let calculated = store
        .update_profile(ProfilePatch {
            calorie_target_mode: Some("calculated".into()),
            ..ProfilePatch::default()
        })
        .unwrap();
    assert_eq!(calculated.nutrition_targets.calories_kcal, Some(2935.0));
    assert_eq!(calculated.manual_calorie_target_kcal, Some(2750.0));
    let restored = store
        .update_profile(ProfilePatch {
            calorie_target_mode: Some("manual".into()),
            ..ProfilePatch::default()
        })
        .unwrap();
    assert_eq!(restored.nutrition_targets.calories_kcal, Some(2750.0));
}

#[test]
fn saving_or_deleting_a_planned_recipe_rebuilds_an_existing_range_list() {
    let store = MealzStore::in_memory().unwrap();
    let recipe = store.save_recipe(test_recipe("Rebuild-Rezept")).unwrap();
    let monday = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
    store
        .save_plan_entry(PlanEntry {
            id: String::new(),
            date: monday,
            slot: "dinner".into(),
            recipe_id: Some(recipe.id.clone()),
            title_override: None,
            servings: 2.0,
            status: "planned".into(),
            notes: None,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        })
        .unwrap();
    store
        .rebuild_shopping_list(monday, monday + Duration::days(6))
        .unwrap();
    let mut changed = recipe.clone();
    changed.ingredients[0].quantity = 2.0;
    store.save_recipe(changed).unwrap();
    assert_eq!(
        store
            .get_shopping_list(monday, monday + Duration::days(6))
            .unwrap()
            .items
            .iter()
            .find(|item| item.name == "Kartoffeln")
            .unwrap()
            .quantity,
        2000.0
    );
    store.delete_recipe(&recipe.id).unwrap();
    assert!(
        !store
            .get_shopping_list(monday, monday + Duration::days(6))
            .unwrap()
            .items
            .iter()
            .any(|item| item.name == "Kartoffeln")
    );
    assert!(store.get_plan_range(monday, monday).unwrap().is_empty());
    store.undo_last().unwrap();
    assert_eq!(store.get_plan_range(monday, monday).unwrap().len(), 1);
    assert!(store.get_recipe(&recipe.id).is_ok());
}

#[test]
fn manual_shopping_items_stay_in_the_explicit_cross_month_range() {
    let store = MealzStore::in_memory().unwrap();
    let start = NaiveDate::from_ymd_opt(2026, 7, 29).unwrap();
    let end = NaiveDate::from_ymd_opt(2026, 8, 4).unwrap();
    store
        .add_manual_shopping_item(
            start,
            end,
            "Kaffee".into(),
            1.0,
            "Packung".into(),
            Some("Getränke".into()),
            None,
        )
        .unwrap();
    assert!(
        store
            .get_shopping_list(start, end)
            .unwrap()
            .items
            .iter()
            .any(|item| item.name == "Kaffee")
    );
    assert!(
        store
            .get_shopping_list(start + Duration::days(7), end + Duration::days(7))
            .is_err()
    );
}

#[test]
fn calendar_range_reader_is_exact_and_capped_to_31_days() {
    let store = MealzStore::in_memory().unwrap();
    let recipe = store.save_recipe(test_recipe("Kalenderbereich")).unwrap();
    let july = NaiveDate::from_ymd_opt(2026, 7, 30).unwrap();
    let august = NaiveDate::from_ymd_opt(2026, 8, 1).unwrap();
    for (date, slot) in [(july, "dinner"), (august, "lunch")] {
        store
            .save_plan_entry(PlanEntry {
                id: String::new(),
                date,
                slot: slot.into(),
                recipe_id: Some(recipe.id.clone()),
                title_override: None,
                servings: 1.0,
                status: "planned".into(),
                notes: None,
                sort_order: 0,
                created_at: String::new(),
                updated_at: String::new(),
            })
            .unwrap();
    }
    let range = store.get_plan_range(august, august).unwrap();
    assert_eq!(range.len(), 1);
    assert_eq!(range[0].date, august);
    assert!(
        store
            .get_plan_range(july, july + Duration::days(31))
            .is_err()
    );
}
