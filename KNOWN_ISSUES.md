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
