"""
Scheduler for automated tasks in General Downloader
"""
import threading
import time
import schedule
import logging
from datetime import datetime, time as dt_time
from typing import Callable, Optional
import os
import sys

class TaskScheduler:
    def __init__(self):
        self.is_running = False
        self.scheduler_thread = None
        self.logger = logging.getLogger(__name__)
        self.jobs = {}  # Store job references for cancellation

    def start(self):
        """Start the scheduler"""
        if self.is_running:
            self.logger.warning("Scheduler is already running")
            return

        self.is_running = True
        self.scheduler_thread = threading.Thread(target=self._run_scheduler, daemon=False)
        self.scheduler_thread.start()
        self.logger.info("Task scheduler started")

    def stop(self):
        """Stop the scheduler"""
        if not self.is_running:
            self.logger.warning("Scheduler is not running")
            return

        self.is_running = False
        # Clear all scheduled jobs
        schedule.clear()
        self.jobs.clear()

        if self.scheduler_thread and self.scheduler_thread.is_alive():
            print("[SCHEDULER] Waiting for thread to finish...")
            self.scheduler_thread.join(timeout=3)
            print("[SCHEDULER] Thread joined")
        self.logger.info("Task scheduler stopped")

    def _run_scheduler(self):
        """Main scheduler loop"""
        self.logger.info("Scheduler thread started")
        while self.is_running:
            try:
                schedule.run_pending()
                time.sleep(1)  # Check every second
            except Exception as e:
                self.logger.error(f"Error in scheduler loop: {e}")
                time.sleep(5)

    def schedule_daily(self, task_time: str, job_func: Callable, job_id: str = None) -> str:
        """
        Schedule a daily task
        task_time: String in format "HH:MM" (24-hour format)
        """
        if not self.is_running:
            self.logger.warning("Scheduler is not started. Call start() first.")
            return None

        job_id = job_id or f"daily_{int(time.time())}"

        try:
            # Parse the time
            hour, minute = map(int, task_time.split(':'))

            # Schedule the job
            job = schedule.every().day.at(task_time).do(job_func).tag(job_id)
            self.jobs[job_id] = job

            self.logger.info(f"Scheduled daily task '{job_id}' at {task_time}")
            return job_id
        except Exception as e:
            self.logger.error(f"Error scheduling daily task: {e}")
            return None

    def schedule_interval(self, interval_minutes: int, job_func: Callable, job_id: str = None) -> str:
        """
        Schedule a task to run every N minutes
        """
        if not self.is_running:
            self.logger.warning("Scheduler is not started. Call start() first.")
            return None

        job_id = job_id or f"interval_{int(time.time())}"

        try:
            job = schedule.every(interval_minutes).minutes.do(job_func).tag(job_id)
            self.jobs[job_id] = job

            self.logger.info(f"Scheduled interval task '{job_id}' every {interval_minutes} minutes")
            return job_id
        except Exception as e:
            self.logger.error(f"Error scheduling interval task: {e}")
            return None

    def schedule_once_at(self, run_time: datetime, job_func: Callable, job_id: str = None) -> str:
        """
        Schedule a task to run once at a specific time
        """
        if not self.is_running:
            self.logger.warning("Scheduler is not started. Call start() first.")
            return None

        job_id = job_id or f"once_{int(time.time())}"

        try:
            # Calculate delay
            now = datetime.now()
            delay = (run_time - now).total_seconds()

            if delay <= 0:
                self.logger.warning("Scheduled time is in the past")
                return None

            def delayed_job():
                time.sleep(delay)
                if self.is_running:  # Check if still running
                    job_func()

            # Schedule the delayed job
            job = schedule.every().seconds.do(delayed_job).tag(job_id)
            self.jobs[job_id] = job

            self.logger.info(f"Scheduled one-time task '{job_id}' at {run_time}")
            return job_id
        except Exception as e:
            self.logger.error(f"Error scheduling one-time task: {e}")
            return None

    def cancel_job(self, job_id: str) -> bool:
        """Cancel a scheduled job"""
        if job_id in self.jobs:
            try:
                schedule.clear(job_id)
                del self.jobs[job_id]
                self.logger.info(f"Cancelled job: {job_id}")
                return True
            except Exception as e:
                self.logger.error(f"Error cancelling job {job_id}: {e}")
                return False
        else:
            self.logger.warning(f"Job not found: {job_id}")
            return False

    def get_jobs(self) -> Dict[str, str]:
        """Get list of scheduled jobs"""
        # Return a simplified view
        return {job_id: str(job) for job_id, job in self.jobs.items()}

    def run_pending(self):
        """Run pending jobs (for manual control)"""
        schedule.run_pending()

# Global scheduler instance
scheduler = TaskScheduler()

def start_scheduler():
    """Start the global scheduler"""
    scheduler.start()

def stop_scheduler():
    """Stop the global scheduler"""
    scheduler.stop()

def schedule_daily_10pm_icon_update():
    """
    Example function to schedule the daily 10 PM icon update
    This would call your existing anime_icon_manager.py script
    """
    def update_icons():
        logging.info("Running scheduled 10 PM icon update...")
        try:
            import subprocess
            import sys
            script_path = r"C:\Users\Abhi\anime_icon_manager.py"
            result = subprocess.run([sys.executable, script_path],
                                  capture_output=True, text=True, timeout=300)
            if result.returncode == 0:
                logging.info("Icon update completed successfully")
                logging.debug(f"Output: {result.stdout}")
            else:
                logging.error(f"Icon update failed: {result.stderr}")
        except subprocess.TimeoutExpired:
            logging.error("Icon update timed out")
        except Exception as e:
            logging.error(f"Error running icon update: {e}")

    # Schedule for 10:00 PM daily
    return scheduler.schedule_daily("22:00", update_icons, "daily_10pm_icon_update")

def schedule_anime_tab_checker(browser_monitor_callback: Callable = None):
    """
    Schedule periodic checking of browser tabs for anime
    """
    def check_tabs():
        logging.info("Checking for anime tabs...")
        if browser_monitor_callback:
            try:
                browser_monitor_callback()
            except Exception as e:
                logging.error(f"Error in browser tab check callback: {e}")

    # Check every 15 minutes
    return scheduler.schedule_interval(15, check_tabs, "anime_tab_checker")

if __name__ == "__main__":
    # Test the scheduler
    logging.basicConfig(level=logging.INFO)

    def sample_task():
        print(f"[{datetime.now().strftime('%H:%M:%S')}] Sample task executed!")

    # Start scheduler
    start_scheduler()

    # Schedule a task to run every 10 seconds for testing
    job_id = scheduler.schedule_interval(0.16, sample_task, "test_job")  # 10 seconds = 0.16 minutes

    # Schedule a daily task for testing (set to 1 minute from now)
    test_time = (datetime.now() + timedelta(minutes=1)).strftime("%H:%M")
    daily_job_id = scheduler.schedule_daily(test_time, sample_task, "test_daily")

    print(f"Scheduled jobs: {scheduler.get_jobs()}")
    print("Running for 2 minutes...")

    try:
        time.sleep(120)  # Run for 2 minutes
    except KeyboardInterrupt:
        print("\nStopping...")
    finally:
        # Clean up
        scheduler.cancel_job("test_job")
        scheduler.cancel_job("test_daily")
        stop_scheduler()
        print("Scheduler stopped.")