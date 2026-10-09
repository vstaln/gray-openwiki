The codebase wiki at {{WIKI_DIR}} for the repository at {{ROOT}} is stale. Since it was generated ({{COMMITTED_AT}}), the source changed:

Modified files ({{MODIFIED_COUNT}}):
{{MODIFIED_LIST}}

Added files ({{ADDED_COUNT}}):
{{ADDED_LIST}}

Removed files ({{REMOVED_COUNT}}):
{{REMOVED_LIST}}

{{MISSING_PAGES_BLOCK}}Refresh only the wiki pages affected by these changes: read the changed files, then revise the relevant sections of the affected pages (architecture.md, modules/, flows/, glossary.md). Create a new page only when a change introduces a concept none of the existing pages cover, and regenerate index.md only if the page set changed. Preserve accurate unaffected content — do not rewrite untouched pages.

When done, finalize the ledger by running:

    {{EXE}} commit --root {{ROOT}}
