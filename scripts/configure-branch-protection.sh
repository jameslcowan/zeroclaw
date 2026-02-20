#!/usr/bin/env bash
# scripts/configure-branch-protection.sh
#
# Configures a GitHub repository ruleset on 'main' that requires all CI gate
# checks to pass before a PR can be merged. PR Intake Checks is intentionally
# excluded (optional).
#
# Prerequisites:
#   - gh CLI authenticated with admin access to the repository
#
# Usage:
#   bash scripts/configure-branch-protection.sh [owner/repo]
#
# If owner/repo is omitted, it is inferred from the current git remote.

set -euo pipefail

REPO="${1:-$(gh repo view --json nameWithOwner -q .nameWithOwner)}"

echo "Configuring required status checks ruleset for: ${REPO}"
echo ""
echo "Required gate checks:"
echo "  - CI Required Gate         (ci-run.yml)"
echo "  - Sec Audit Gate           (sec-audit.yml)"
echo "  - Workflow Sanity Gate     (workflow-sanity.yml)"
echo "  - PR Label Policy Gate     (pr-label-policy-check.yml)"
echo "  - Docker Gate              (pub-docker-img.yml)"
echo ""
echo "Optional (not required):"
echo "  - Intake Checks            (pr-intake-checks.yml)"
echo ""

# Create the ruleset via the GitHub API.
# The integration_id for GitHub Actions is 15368.
gh api \
  --method POST \
  "repos/${REPO}/rulesets" \
  --input - <<'EOF'
{
  "name": "Require CI Gates",
  "target": "branch",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "include": ["refs/heads/main"],
      "exclude": []
    }
  },
  "rules": [
    {
      "type": "required_status_checks",
      "parameters": {
        "strict_required_status_checks_policy": false,
        "required_status_checks": [
          {
            "context": "CI Required Gate",
            "integration_id": 15368
          },
          {
            "context": "Sec Audit Gate",
            "integration_id": 15368
          },
          {
            "context": "Workflow Sanity Gate",
            "integration_id": 15368
          },
          {
            "context": "PR Label Policy Gate",
            "integration_id": 15368
          },
          {
            "context": "Docker Gate",
            "integration_id": 15368
          }
        ]
      }
    }
  ],
  "bypass_actors": []
}
EOF

echo ""
echo "Ruleset 'Require CI Gates' created successfully."
echo "PRs to main now require all gate checks to pass before merging."
