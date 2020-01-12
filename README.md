sql-perf-linter
===============

This is a linter to identify potential downtime-causing performance issues in SQL migrations.
It is pretty specific to PostgreSQL 9.6 since that's primarily what we care about at Thought Machine
right now. Other databases or newer versions may not have exactly the same concerns (for example
in PostgreSQL 11+ it is possible to add columns with a default value without a table rewrite).
