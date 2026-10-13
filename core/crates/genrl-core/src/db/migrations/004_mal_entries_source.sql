-- Migration 004: source column for mal_entries ('mal' or 'jikan')
ALTER TABLE mal_entries ADD COLUMN source TEXT NOT NULL DEFAULT 'mal';
