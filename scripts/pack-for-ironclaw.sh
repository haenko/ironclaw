#!/usr/bin/env bash
#
# Pack a tool from this repo into the upstream IronClaw layout.
#
# Usage:
#   scripts/pack-for-ironclaw.sh <tool-name> <path-to-ironclaw-checkout>
#
# Produces (inside the upstream checkout):
#   tools-src/<tool-name>/         a copy of tools/<tool-name>/
#   registry/tools/<tool-name>.json   generated from the tool's manifest
#
# Skills are copied separately. Skills that branch from this tool are listed
# below; the script copies each into <upstream>/skills/<skill-name>/.
#
# After running, review the diff inside the upstream checkout, then open a PR.

set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <tool-name> <path-to-ironclaw-checkout>" >&2
  exit 64
fi

TOOL="$1"
UPSTREAM="$2"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

SOURCE_DIR="$ROOT/tools/$TOOL"
if [ ! -d "$SOURCE_DIR" ]; then
  echo "error: tool not found at $SOURCE_DIR" >&2
  exit 66
fi

if [ ! -d "$UPSTREAM/tools-src" ]; then
  echo "error: $UPSTREAM does not look like an IronClaw checkout (no tools-src/)" >&2
  exit 66
fi

# Copy the tool source.
TARGET_TOOL_DIR="$UPSTREAM/tools-src/$TOOL"
echo "copying tool: $SOURCE_DIR -> $TARGET_TOOL_DIR"
rm -rf "$TARGET_TOOL_DIR"
mkdir -p "$TARGET_TOOL_DIR"
cp -r "$SOURCE_DIR"/. "$TARGET_TOOL_DIR/"
rm -rf "$TARGET_TOOL_DIR/target" "$TARGET_TOOL_DIR/Cargo.lock"

# Copy skills that branch from this trunk. A skill branches from a trunk when
# its SKILL.md mentions the trunk by name in the body. This is a heuristic;
# review the diff.
SKILLS_DIR="$ROOT/skills"
if [ -d "$SKILLS_DIR" ]; then
  while IFS= read -r skill_md; do
    skill_dir="$(dirname "$skill_md")"
    skill_name="$(basename "$skill_dir")"
    if grep -q "\b$TOOL\b" "$skill_md" 2>/dev/null; then
      target_skill_dir="$UPSTREAM/skills/$skill_name"
      echo "copying skill: $skill_dir -> $target_skill_dir"
      rm -rf "$target_skill_dir"
      mkdir -p "$target_skill_dir"
      cp -r "$skill_dir"/. "$target_skill_dir/"
    fi
  done < <(find "$SKILLS_DIR" -name SKILL.md)
fi

# Generate the registry entry from the tool's manifest.
CAPS="$SOURCE_DIR/$TOOL-tool.capabilities.json"
if [ ! -f "$CAPS" ]; then
  echo "warning: no $TOOL-tool.capabilities.json found; skipping registry entry" >&2
else
  REGISTRY_FILE="$UPSTREAM/registry/tools/$TOOL.json"
  mkdir -p "$(dirname "$REGISTRY_FILE")"
  description="$(jq -r '.description // ""' "$CAPS")"
  setup_url="$(jq -r '.auth.setup_url // ""' "$CAPS")"
  secret="$(jq -r '.auth.secret_name // ""' "$CAPS")"
  method="$(jq -r 'if .auth.oauth then "oauth" else "manual" end' "$CAPS")"
  provider="$(jq -r '.auth.display_name // ""' "$CAPS")"

  jq -n \
    --arg name "$TOOL" \
    --arg display_name "$provider" \
    --arg description "$description" \
    --arg setup_url "$setup_url" \
    --arg method "$method" \
    --arg provider "$provider" \
    --arg secret "$secret" \
    --arg dir "tools-src/$TOOL" \
    --arg caps "$TOOL-tool.capabilities.json" \
    --arg crate "${TOOL}-tool" \
    '{
      name: $name,
      display_name: $display_name,
      kind: "tool",
      version: "0.1.0",
      wit_version: "0.3.0",
      description: $description,
      keywords: [],
      source: { dir: $dir, capabilities: $caps, crate_name: $crate },
      artifacts: {},
      auth_summary: {
        method: $method,
        provider: $provider,
        secrets: [$secret],
        shared_auth: null,
        setup_url: $setup_url
      },
      tags: []
    }' > "$REGISTRY_FILE"
  echo "wrote registry: $REGISTRY_FILE"
  echo "note: keywords and tags are empty; populate before opening the upstream PR."
fi

echo
echo "done. review the diff inside $UPSTREAM, then open a PR upstream."
