use anyhow::Result;
use genrl_core::Db;

/// Every season in a show's chain, with live download counts.
pub(crate) fn seasons_json(db: &Db, mal_id: i64) -> Result<Vec<serde_json::Value>> {
    let seasons = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT s.mal_id, s.title, COALESCE(s.season, 1), COALESCE(s.total_episodes, 0),
                    s.status, COALESCE(s.type, 'tv'), s.start_year, s.poster_path,
                    s.metadata_status,
                    (SELECT COUNT(*) FROM episodes e
                      WHERE e.mal_id = s.mal_id AND e.state = 'done'),
                    (SELECT COUNT(*) FROM jobs j JOIN episodes e ON e.id = j.episode_id
                      WHERE e.mal_id = s.mal_id AND j.state NOT IN ('done', 'failed')),
                    (SELECT COUNT(*) FROM jobs j JOIN episodes e ON e.id = j.episode_id
                      WHERE e.mal_id = s.mal_id AND j.state = 'failed')
             FROM shows s
             WHERE COALESCE(s.franchise_id, s.mal_id) =
                   (SELECT COALESCE(franchise_id, mal_id) FROM shows WHERE mal_id = ?1)
             ORDER BY COALESCE(s.season, 1), s.start_year, s.mal_id",
        )?;
        let rows = stmt.query_map([mal_id], |r| {
            Ok(serde_json::json!({
                "mal_id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "season": r.get::<_, i64>(2)?,
                "total_episodes": r.get::<_, i64>(3)?,
                "airing_status": r.get::<_, Option<String>>(4)?,
                "media_type": r.get::<_, String>(5)?,
                "start_year": r.get::<_, Option<i64>>(6)?,
                "poster_url": r.get::<_, Option<String>>(7)?,
                "metadata_status": r.get::<_, Option<String>>(8)?,
                "downloaded_episodes": r.get::<_, i64>(9)?,
                "active_jobs": r.get::<_, i64>(10)?,
                "failed_jobs": r.get::<_, i64>(11)?,
            }))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    })?;
    Ok(seasons)
}
