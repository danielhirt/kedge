---
inclusion: manual
name: steer-check
description: Manually run a documentation drift check against the current branch
---

# Steer Check Skill

Run `steer check` to detect documentation drift against the current branch.

## Usage

Invoke this skill when you want to proactively verify that documentation
is still accurate after making code changes, before opening a pull request.

## Steps

1. Run `steer check --config steer.toml` in the repository root
2. Review the drift report output
3. If drift is detected, either:
   - Update the documentation manually
   - Run `steer update` to trigger the automated remediation pipeline
4. Run `steer sync` to advance provenance markers for no-update anchors
