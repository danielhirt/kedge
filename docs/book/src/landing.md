<div class="landing">

<div class="hero">
<img src="assets/logo.png" alt="kedge" class="hero-logo" style="max-width: 420px; margin-bottom: 1rem;">

<p class="tagline">Stop documentation drift before it reaches production.</p>
<p class="subtitle">AST-aware drift detection, AI severity triage, and agent-driven remediation for enterprise codebases.</p>

<div class="install-block">

```bash
cargo install kedge
```

</div>

<div class="hero-links">
<a href="getting-started/installation.html" class="btn btn-primary">Get Started</a>
<a href="getting-started/quick-start.html" class="btn btn-secondary">Quick Start</a>
</div>
</div>

<div class="features">

<div class="feature">
<h3>Detection</h3>
<p>Tree-sitter AST fingerprinting compares code structure, not text. Ignores whitespace and comment changes. Tracks specific methods and classes. Survives rebase and squash.</p>
</div>

<div class="feature">
<h3>Triage</h3>
<p>A lightweight LLM call classifies each drift as <code>no_update</code>, <code>minor</code>, or <code>major</code>. No AI cost for clean docs. Triage runs only on drifted anchors.</p>
</div>

<div class="feature">
<h3>Remediation</h3>
<p>Invokes your agent (Kiro, Claude Code, or custom) to update docs, stamp new provenance, and open merge requests. kedge's pipeline ends when the agent returns — MR review and merging follow your existing workflows.</p>
</div>

</div>

<div class="details">

<div class="detail-section">
<h3>Enterprise CI native</h3>
<p>Runs as an MR gate (<code>kedge check</code>, exit 1 on drift) or a scheduled pipeline (<code>kedge update</code> for full detect-triage-remediate). The repo includes GitLab CI and GitHub Actions examples.</p>
</div>

<div class="detail-section">
<h3>Multi-language, multi-team</h3>
<p>AST fingerprinting for Java, Go, TypeScript, Python, Rust, and XML. Content-hash fallback for everything else. Group-scoped steering files let each team own their docs.</p>
</div>

<div class="detail-section">
<h3>Zero runtime dependencies</h3>
<p>Single static binary. No JVM, no Node, no Python. Uses rustls (no OpenSSL) and git CLI (no libgit2 at runtime). Copy it to your CI runner and go.</p>
</div>

</div>

</div>
