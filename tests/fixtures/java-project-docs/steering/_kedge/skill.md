---
inclusion: manual
name: kedge-check
description: Manually run a documentation drift check against the current branch
---

# Steer Check Skill

Run `kedge check` to detect documentation drift against the current branch.

## Usage

Invoke this skill when you want to proactively verify that documentation
is still accurate after making code changes, before opening a pull request.

## Steps

1. Run `kedge check --config kedge.toml` in the repository root
2. Review the drift report output
3. If drift is detected, either:
   - Update the documentation manually
   - Run `kedge update` to trigger the automated remediation pipeline
4. Run `kedge sync` to advance provenance markers for no-update anchors
