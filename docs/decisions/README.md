# Decision records

One file per significant engineering or design decision, numbered and
dated. Each records the **context** (what forced a choice), the **options
considered** (including the rejected ones — they are half the value), the
**decision**, and the **consequences** we accepted.

The git log stays the fine-grained record (commit messages carry the why);
these records exist for the decisions a future reader would otherwise have
to reverse-engineer from a hundred commits.

Format: `NNNN-short-slug.md`, status one of `proposed | accepted |
superseded by NNNN`.
