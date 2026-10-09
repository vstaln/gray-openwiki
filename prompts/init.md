Generate a structured codebase wiki for the repository at {{ROOT}}.

{{FILE_COUNT}} source files are tracked (HEAD {{HEAD}}). Top-level layout: {{TOP_DIRS}}.

Write Markdown pages under {{WIKI_DIR}}/:

- index.md — repo overview: what it does, how to build/test/run it, and relative links to every other wiki page.
- architecture.md — major components, their responsibilities, boundaries, and dependency direction.
- modules/<name>.md — one page per major module/package/directory: responsibility, key types, public surface.
- flows/<name>.md — one page per important end-to-end flow (entry point → processing → result), naming the real files and functions involved.
- glossary.md — project-specific terms, abbreviations, and concepts.

Rules:
- Read real source before writing; every claim must trace to code you inspected. Cite repo-relative paths like `src/main.rs`.
- Only write files under {{WIKI_DIR}} — never modify repository source.
- Prefer accurate and concise over exhaustive; skip generated and trivial files.
- Cross-link pages with relative links.

When all pages are written, finalize the wiki ledger by running:

    {{EXE}} commit --root {{ROOT}}
