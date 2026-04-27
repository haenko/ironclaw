# Tracking

Status of every tool and skill currently in this repository. Updated in the same commit that adds, modifies, or removes an entry.

Last updated: 2026-04-26

## Summary

- 1 tool live
- 1 skill live
- 0 open bugs against shipped integrations

## Tools

| Name | Status | Version | Description | Limits | Author |
|---|---|---|---|---|---|
| `microsoft-365` | live | 0.1.0 | Microsoft Graph integration. 14 actions across Outlook, Excel, Teams, OneDrive, SharePoint, Calendar, plus Word and PowerPoint document generation. OAuth via Microsoft Entra ID. | Teams actions return 403 on personal Microsoft accounts (Microsoft does not serve Teams business APIs to consumer MSAs). Simple upload capped at 4 MB; chunked upload session not yet implemented. | Brandon |

## Skills

| Name | Trunk | Status | Version | Description | Author |
|---|---|---|---|---|---|
| `microsoft-365-workflow` | `microsoft-365` | live | 1.0.0 | Microsoft 365 business workflow patterns. 18 activation keywords, 6 regex patterns, 6,500 token budget. | Brandon |

## Open work

Proposed and in-progress tools and skills are tracked as GitHub issues. Filter by label:

- `type:tool`, `type:skill`, `type:bug`
- `status:proposed`, `status:in-progress`, `status:blocked`
- `trunk:<tool-name>` (links a proposed skill to its required trunk)

## Status definitions

- **proposed**: issue filed, no code yet, no claimed author
- **in-progress**: branch exists, work underway
- **live**: merged to main, CI green, included in this table
- **blocked**: dependency or external decision required, named in the issue
- **deprecated**: superseded by a different integration or removed; documented in the relevant PR
