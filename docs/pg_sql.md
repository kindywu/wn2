``` sql
SELECT e.id, e.pos, f.form, f.rank
    FROM entries e
    JOIN forms f ON f.entry_rowid = e.rowid
    WHERE f.form = 'bank'
    ORDER BY f.rank;
```

``` sql
SELECT e.pos,
       se.entry_rank,
       f.form,
       STRING_AGG(m.form, ', ') AS synset_members,
       d.definition
  FROM entries e
  JOIN forms f ON f.entry_rowid = e.rowid
  JOIN senses se ON se.entry_rowid = e.rowid
  JOIN synsets s ON se.synset_rowid = s.rowid
  JOIN definitions d ON d.synset_rowid = s.rowid
  JOIN senses se2 ON se2.synset_rowid = s.rowid
  JOIN forms m ON m.entry_rowid = se2.entry_rowid AND m.rank = 0
  WHERE f.form = 'bank'
  GROUP BY se.rowid, e.pos, se.entry_rank, f.form, d.definition
  ORDER BY e.pos, se.entry_rank;
```