# Environment Variables

Purpose: List environment variables recognized by Ralph and how they affect behavior.

## .env Files

> **Security Note:** `.env` files should NOT be committed to public repositories. They may contain secrets or sensitive configuration. This repo uses `.env.example` as the canonical template—copy it to `.env` and customize locally.

- `.env`: project-local environment configuration (ignored by git; never commit to public repos).
- `.env.example`: canonical template for new environments (committed; safe to share).

## Variables
- `RALPH_STRESS_BURN_IN`: `0` or `1`. Enables burn-in stress tests when set to `1`.

Example:
```bash
RALPH_STRESS_BURN_IN=1
```
