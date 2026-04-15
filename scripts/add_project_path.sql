-- Migration: add path column to projects table
-- Safe to run on both empty and populated databases.
-- Existing rows get NULL (no path set); update manually via `brain project update <id> --path <path>`.
ALTER TABLE projects ADD COLUMN path TEXT;
