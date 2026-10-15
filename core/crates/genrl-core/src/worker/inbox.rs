use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::jobs::{self, MoveOptions};
use crate::models::{JobState, Question};
use crate::planner;
use crate::questions::{Answer, Choice, ExpectedShow};
use crate::settings::Settings;

/// Apply an Inbox answer to the job that asked.
///
/// The design's answers are verbs on the queue: "use this one" resumes at add,
/// "none of these" parks the episode as missing, "wait 24 h" defers, and the
/// "always do this" half of the answer is what the caller stores (an alias or
/// a rule) — see [`crate::names::NameSet`].
pub fn apply_answer(
    db: &Db,
    clock: &dyn Clock,
    question: &Question,
    answer: &Answer,
) -> Result<Option<JobState>> {
    let Some(job_id) = question.job_id else {
        return Ok(None);
    };
    let Some(job) = jobs::get(db, job_id)? else {
        return Ok(None);
    };
    // Only a job actually waiting on the Inbox may be answered; anything else
    // was answered, expired or moved on, and applying twice must be safe.
    if !matches!(job.state, JobState::NeedsInput | JobState::Failed) {
        return Ok(Some(job.state));
    }
    let mal_id = question.mal_id;
    let now = clock.now();

    // A saved alias ("always do this") lands before the state moves, so the
    // next search starts from it.
    if answer.always {
        if let Choice::Alias { name } = &answer.choice {
            if let Some(mal_id) = mal_id {
                names_alias(db, mal_id, name)?;
            }
        }
    }

    let next = match &answer.choice {
        Choice::UseThis { info_hash } => {
            db.with(|c| {
                c.execute(
                    "UPDATE jobs SET torrent_id = (SELECT id FROM torrents WHERE info_hash = ?2)
                     WHERE id = ?1",
                    rusqlite::params![job_id, info_hash],
                )?;
                Ok(())
            })?;
            Some(JobState::Add)
        }
        Choice::NoneOfThese => {
            // Parked: the episode returns to missing and the weekly plan can
            // offer it again (the user can edit aliases meanwhile).
            Some(JobState::Failed)
        }
        Choice::SearchAgain => Some(JobState::Search),
        Choice::Alias { name } => {
            if let Some(mal_id) = mal_id {
                names_alias(db, mal_id, name)?;
            }
            Some(JobState::Search)
        }
        Choice::Download => Some(JobState::Add),
        Choice::Wait => {
            jobs::reschedule(db, clock, job_id, now + 24 * 3600, "waiting 24 h for seeds")?;
            Some(JobState::Search)
        }
        Choice::PickAnother => Some(JobState::Search),
        Choice::Keep => {
            // Already on disk: the episode is owned, the job is done.
            let (episode_id, path): (i64, Option<String>) = db.with(|c| {
                Ok(c.query_row(
                    "SELECT j.episode_id, e.file_path FROM jobs j JOIN episodes e ON e.id = j.episode_id
                     WHERE j.id = ?1",
                    [job_id],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?)
            })?;
            let size = path
                .as_deref()
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            planner::mark_done(db, episode_id, path.as_deref().unwrap_or(""), size)?;
            Some(JobState::Done)
        }
        Choice::Replace | Choice::KeepBoth => {
            if jobs::is_allowed(job.state, JobState::Finalize) {
                Some(JobState::Finalize)
            } else {
                Some(JobState::Add)
            }
        }
        Choice::Batch => Some(JobState::Add),
        Choice::WaitForSingles => Some(JobState::Search),
        Choice::Pause => {
            // The global pause switch: the scheduler stops, downloads continue.
            let mut settings = Settings::load(db)?;
            settings.automation_on = false;
            Settings::save(&settings, db)?;
            Some(JobState::NeedsInput)
        }
        // Changing the folder is the settings page's job; the job keeps waiting
        // so the human can answer again once a folder that works exists.
        Choice::ChangeFolder => None,
        Choice::RetryLater => {
            // Answered on a failed job: a fresh attempt at the first step.
            jobs::transition(
                db,
                clock,
                job_id,
                JobState::Search,
                "retry later",
                &MoveOptions {
                    reset_attempts: true,
                    ..Default::default()
                },
            )?;
            return Ok(Some(JobState::Search));
        }
        Choice::ManualSearch => None,
        Choice::SkipEpisode => Some(JobState::Failed),
        Choice::Expired => None,
    };

    let Some(next) = next else {
        return Ok(Some(job.state));
    };

    // An answer that would move the job somewhere the diagram does not allow
    // is ignored rather than errored: a `failed` job's AllFailed question only
    // offers retry-later / manual-search / skip, and the rest would need a
    // transition that does not exist (failed → add, say).
    if next != job.state && !jobs::is_allowed(job.state, next) {
        return Ok(Some(job.state));
    }

    let opts = MoveOptions {
        reset_attempts: matches!(next, JobState::Search),
        clear_torrent: matches!(
            answer.choice,
            Choice::PickAnother | Choice::NoneOfThese
        ),
        blocked_from: None,
        count_attempt: false,
    };
    jobs::transition(
        db,
        clock,
        job_id,
        next,
        &format!("answered: {}", answer_label(&answer.choice)),
        &opts,
    )?;
    Ok(Some(next))
}

fn answer_label(choice: &Choice) -> String {
    match choice {
        Choice::UseThis { .. } => "use this one".to_string(),
        Choice::NoneOfThese => "none of these".to_string(),
        Choice::SearchAgain => "search again".to_string(),
        Choice::Alias { name } => format!("alias {name}"),
        Choice::Download => "download anyway".to_string(),
        Choice::Wait => "wait 24 h".to_string(),
        Choice::PickAnother => "pick another".to_string(),
        Choice::Keep => "keep what is on disk".to_string(),
        Choice::Replace => "replace".to_string(),
        Choice::KeepBoth => "keep both".to_string(),
        Choice::Batch => "take the batch".to_string(),
        Choice::WaitForSingles => "wait for singles".to_string(),
        Choice::Pause => "pause the queue".to_string(),
        Choice::ChangeFolder => "choose another folder".to_string(),
        Choice::RetryLater => "retry later".to_string(),
        Choice::ManualSearch => "manual search".to_string(),
        Choice::SkipEpisode => "skip this episode".to_string(),
        Choice::Expired => "expired".to_string(),
    }
}

/// Attempts are a counter in the database; a question payload counts them as
/// a `u32`, and a nonsensical value must not wrap.
pub(crate) fn tried(attempts: i64) -> u32 {
    attempts.clamp(0, u32::MAX as i64) as u32
}

fn names_alias(db: &Db, mal_id: i64, name: &str) -> Result<()> {
    crate::names::NameSet::add(
        db,
        mal_id,
        name,
        crate::models::NameSource::User,
        crate::names::PRIORITY_USER,
    )
    .map(|_| ())
}

/// The expected-show card for a question (used by the UI and tests).
pub fn expected_show(mal_id: i64, title: &str, english_title: Option<String>) -> ExpectedShow {
    ExpectedShow {
        mal_id,
        title: title.to_string(),
        english_title,
        year: None,
    }
}
