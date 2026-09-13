# Output contract

Respond with exactly one valid JSON object, nothing else: no markdown fences, no text before or after.

```json
{
  "name": "Display name of the overlay",
  "files": {
    "overlay_json": "full content of overlay.json (no top-level id field)",
    "index_html": "full content of index.html",
    "style_css": "full content of style.css",
    "script_js": "full content of script.js"
  }
}
```

The folder name is derived from "name" (kebab-case) — you do not provide the id.