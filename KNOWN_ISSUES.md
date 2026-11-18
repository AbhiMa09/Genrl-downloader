# Known Issues & Unfinished Items

The following items are documented as known limitations, unverified behaviors, or pending engineering work:

1. **Posters: Real MAL Cover Download Not Proven**
   - While poster download utilities and caching structures exist, live end-to-end MAL cover downloading directly to `<data-dir>/posters` in automated runs has not been conclusively proven against live endpoints without cached fallbacks.

2. **Dashboard "Next Release" from Real Air Date: Not Proven**
   - The Next Release indicator on the Dashboard displays `—` when no verified air schedule exists in SQLite; deriving dynamic countdowns and next-air schedules directly from upstream Jikan broadcast timestamps is not yet verified against live metadata feeds.

3. **Stage-by-Stage Snapshot Timing Unproven**
   - In earlier integration verification tests, stage transition frames captured identical or same-second timestamps due to synchronous test fixture staging rather than real rate-limited network progression (e.g. 2 req/s over 21 shows taking 10+ s).

4. **Dual-Audio Scoring Rule Undefined**
   - The candidate scoring engine lacks an explicit specification/toggle for whether dual-audio releases receive a preference bonus (+10) or penalty relative to original Japanese audio.

5. **Storage Estimate is an Assumed 1.4 GB/Episode**
   - Storage calculations in the setup wizard and review dialog assume a constant factor of 1.4 GB per episode rather than calculating from actual torrent payload byte counts or resolution-specific bitrates.

6. **Test Totals (Cargo / Flutter) Not Recorded in Last Report**
   - Full test run logs with definitive aggregate counts (`cargo test` total pass count and `flutter test` test counts) were omitted from prior diagnostic summaries.

7. **Preflight Recovery Frames Missing**
   - The visual frame capture sequence illustrating recovery from an unreachable qBittorrent WebUI (Step 5 Review error banner -> Open Settings -> Step 2 re-configuration -> Step 5 Launch re-enabled) was not fully preserved as independent screenshots.

8. **Preferred-Groups UI Lists Stale Release Groups**
   - The default selection lists in the Quality & Groups setup screen include defunct or inactive groups such as HorribleSubs and Commie.

9. **Upstream Public Jikan Gateway Availability**
   - Public endpoint `api.jikan.moe` has experienced persistent connection drops/shutdowns, requiring reliance on local/offline cache layers or alternative metadata proxies for reliable resolution.

10. **Public Indexer Noise & False Positives**
    - Public indexers give noisy or fake results; scoring rejects CAM/TS/TC and bad sizes but cannot catch everything.

11. **Prowlarr External Dependency & Indexer Health**
    - Prowlarr must be running separately and must have healthy indexers; some are blocked by ISPs or need FlareSolverr.

12. **Private Tracker Support**
    - No direct private-tracker support; users add them in Prowlarr.

13. **Filesystem Volume Constraint for Hardlinks**
    - Hardlinks require the same volume (copy fallback otherwise).

14. **Series Detail Dialog Level of Detail**
    - Series detail dialog shows only totals, not per-season or per-episode state.

15. **Multi-Season Torrent Packs**
    - Multi-season packs: only files for wanted seasons are placed; others are skipped.

16. **TMDB Air Date Window**
    - Air dates: an episode is "aired" one day after TMDB's date.

17. **Real Data Wizard Verification Test Fragility**
    - real_data_wizard_verification_test.dart: 10 tests skipped by design without a scratch DB, and 7 of them fail when the scratch DB exists because of UI string changes from commit 73ebb86 (pre-existing, not from this work; they need updating).

18. **Desktop Visual Verification Scope**
    - Movies/Dramas visuals were verified only with headless widget tests, not on a live desktop session (until user confirms otherwise).

19. **Legacy Media Row Overview Field**
    - Overview text is stored only for items added after Phase 8; older rows show a placeholder.

20. **Doctor Command Indexer Status Scope**
    - doctor reports "configured, last test ok/failed/never", not a live probe.

