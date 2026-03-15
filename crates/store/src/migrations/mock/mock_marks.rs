use super::ChartIDMap;
use super::markdown::parse_markdown;
use crate::entities::{ActiveMark, MarkModel, Marks};
use crate::error::{StoreError, StoreResult};
use sea_orm::{ActiveModelTrait, ActiveValue::Unchanged, EntityTrait, TryIntoModel};
use sea_orm_migration::prelude::*;
use std::collections::HashMap;
use waypoints_domain::{
    chart::ChartID,
    mark::{Landmark, Mark, MarkID},
};

pub type MarkIDMap = HashMap<String, MarkID>;

/// A page definition: a landmark page plus markdown body content.
struct PageDef {
    /// Key used in the mark ID map (e.g., "page-japan-trip")
    key: &'static str,
    /// Title shown in the landmark
    title: &'static str,
    /// Markdown source for the body content
    markdown: &'static str,
}

/// All seed pages with their markdown bodies.
fn page_defs() -> Vec<PageDef> {
    vec![
        // ── Demo / Welcome ──────────────────────────────────────────
        PageDef {
            key: "page-welcome",
            title: "Welcome to Waypoints",
            markdown: include_str!("docs/demo.md"),
        },
        // ── Trip Planning ───────────────────────────────────────────
        PageDef {
            key: "page-japan-trip",
            title: "Japan Trip 2026",
            markdown: include_str!("docs/japan-trip.md"),
        },
        PageDef {
            key: "page-tokyo-restaurants",
            title: "Tokyo Restaurant Guide",
            markdown: include_str!("docs/tokyo-restaurants.md"),
        },
        PageDef {
            key: "page-travel-gear",
            title: "Travel Gear & Packing",
            markdown: include_str!("docs/travel-gear.md"),
        },
        // ── Pop-Sci Reading ─────────────────────────────────────────
        PageDef {
            key: "page-reading-list",
            title: "Reading List",
            markdown: include_str!("docs/reading-list.md"),
        },
        PageDef {
            key: "page-cosmos-notes",
            title: "Book Notes: Cosmos",
            markdown: include_str!("docs/cosmos-notes.md"),
        },
        PageDef {
            key: "page-elegant-universe",
            title: "Book Notes: The Elegant Universe",
            markdown: include_str!("docs/elegant-universe-notes.md"),
        },
    ]
}

/// Seed marks: creates landmark pages and their parsed content children.
///
/// Returns a `MarkIDMap` so that `seed_terrains` can wire up the parent→child
/// edges. Content marks are keyed as `"{page-key}:content:{index}"`.
pub(crate) async fn seed_marks(
    manager: &SchemaManager<'_>,
    chart_ids: &ChartIDMap,
) -> StoreResult<MarkIDMap> {
    let page_chart_id = try_get_chart(chart_ids, "Page")?;
    let db = manager.get_connection();
    let mut markmap = MarkIDMap::new();

    for def in page_defs() {
        // 1. Create the landmark page mark
        let page_mark = Mark::builder()
            .with_landmark(
                Landmark::builder()
                    .with_chart(page_chart_id.clone())
                    .with_title(def.title.to_string())
                    .build(),
            )
            .build();

        let page_id = insert_mark(db, &mut markmap, def.key, page_mark).await?;

        // 2. Parse markdown into ParsedBlocks and create child marks
        //    Compound blocks (e.g. blockquote) carry nested children that
        //    get their own marks keyed as "{page}:content:{i}:child:{j}".
        let blocks = parse_markdown(def.markdown);
        for (i, parsed) in blocks.into_iter().enumerate() {
            let child_key = format!("{}:content:{}", def.key, i);
            let child_mark = Mark::builder().with_content(parsed.content).build();
            insert_mark(db, &mut markmap, &child_key, child_mark).await?;

            // Nested children of compound blocks
            for (j, nested) in parsed.children.into_iter().enumerate() {
                let nested_key = format!("{}:content:{}:child:{}", def.key, i, j);
                let nested_mark = Mark::builder().with_content(nested).build();
                insert_mark(db, &mut markmap, &nested_key, nested_mark).await?;
            }
        }

        let _ = page_id; // used implicitly via markmap
    }

    Ok(markmap)
}

async fn insert_mark(
    db: &sea_orm_migration::SchemaManagerConnection<'_>,
    markmap: &mut MarkIDMap,
    key: &str,
    mark: Mark,
) -> StoreResult<MarkID> {
    let id = mark.id;
    let exists = Marks::find_by_id(id.inner()).one(db).await?;
    let mut active: ActiveMark = mark.try_into()?;

    let model: MarkModel = if exists.is_some() {
        active.id = Unchanged(id.inner());
        active.update(db).await?
    } else {
        active.insert(db).await?
    }
    .try_into_model()?;

    let mark_id: MarkID = model.id.into();
    markmap.insert(key.to_string(), mark_id);
    Ok(mark_id)
}

fn try_get_chart(chart_ids: &ChartIDMap, key: &str) -> StoreResult<ChartID> {
    chart_ids
        .get(key)
        .cloned()
        .ok_or_else(|| StoreError::Custom(format!("seed chart not found: {key}")))
}
