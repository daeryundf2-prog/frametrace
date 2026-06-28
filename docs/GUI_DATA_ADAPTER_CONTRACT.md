# GUI Data Adapter Contract

FrameTrace GUI shells read case state through Rust engine JSON surfaces. The GUI may cache view-local selection, scroll, and filters, but durable case state stays in `case.json`, `db/case.db`, generated artifacts, and chained JSONL logs.

## Source Of Truth

| GUI Need | Engine Surface | Durable Owner |
| --- | --- | --- |
| Case open and refresh | `frametrace workstation-status <case_dir>` | Rust engine, case manifest, SQLite, JSONL logs |
| Inventory page | `frametrace inventory <case_dir> --limit <n> --offset <n> [--extension <ext>] [--validation-state <state>] [--sort <sort>]` | SQLite `videos` projection |
| Inventory search | `frametrace inventory <case_dir> --search <query> --limit <n>` | SQLite `videos` projection |
| Facets and source tree | `frametrace inventory <case_dir> --facets` | SQLite grouped counts |
| Inventory detail | `frametrace inventory <case_dir> --file-id <file_id>` | SQLite row projection |
| Bulk preview | `frametrace inventory-bulk-preview <case_dir> --action <action> --operator <operator> [--filters-json <json>] <file_id>...` | Non-mutating engine preview |
| Export manifest | `frametrace inventory-export-manifest <case_dir> --operator <operator> [--filters-json <json>] [--output <case_path>] <file_id>...` | Engine-written manifest under case directory |
| Validation and playback state | `frametrace workstation-status <case_dir>` `validation` object | Validation JSONL log |
| Report/package status | `frametrace workstation-status <case_dir>` `generated_artifacts` object | Engine-generated files |

## Required JSON Contract

`workstation-status` includes `gui_data_adapter`:

```json
{
  "schema_version": 1,
  "state_owner": "rust-engine-sqlite-audit",
  "gui_durable_state_allowed": false,
  "full_json_load_allowed": false,
  "max_page_size": 500,
  "surfaces": {
    "case_open": { "command": "workstation-status", "response_view": "workstation-status" },
    "inventory_page": { "command": "inventory", "response_view": "inventory" },
    "inventory_search": { "command": "inventory", "response_view": "inventory" },
    "inventory_facets": { "command": "inventory --facets", "response_view": "facets" },
    "inventory_detail": { "command": "inventory --file-id", "response_view": "detail" },
    "source_tree": { "command": "inventory --facets", "response_view": "facets" },
    "bulk_preview": { "command": "inventory-bulk-preview", "response_view": "bulk-preview" },
    "export_manifest": { "command": "inventory-export-manifest", "response_view": "inventory-export-manifest" },
    "validation_playback_state": { "command": "workstation-status", "response_path": "validation" },
    "report_package_status": { "command": "workstation-status", "response_path": "generated_artifacts" }
  }
}
```

Inventory page responses must be treated as bounded pages. Requests above `max_page_size` are capped to 500 by the engine and return `page_size: 500`, never a full-case browser payload.

## Failure Contract

A non-case directory must fail non-zero with a clear `not a FrameTrace case` diagnostic before inventory reads, previews, or exports. GUI adapters should surface that as a case-open error and must not create replacement durable state.
